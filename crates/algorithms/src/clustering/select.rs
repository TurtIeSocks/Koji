//! `max_clusters` enforcement: when an algorithm produces more clusters than
//! the cap, keep the subset that covers the most points — weighted greedy
//! max-coverage on MARGINAL gain. Not absolute counts (they double-count
//! overlap: two overlapping dense clusters beat a disjoint medium one) and not
//! arbitrary truncation/input-order prefixes.
//!
//! Max-k-cover is NP-hard; greedy carries the classic (1−1/e) ≈ 63%
//! worst-case guarantee and is NOT globally optimal — partial-overlap shapes
//! exist where it keeps a strictly worse subset than the best one (see
//! `greedy_is_approximate_not_optimal`). Good, cheap, deterministic; a
//! coverage shortfall after capping can be inherent to the heuristic rather
//! than a bug elsewhere.
//!
//! The RADIUS arm does not select from the algorithm's own centers alone: a
//! binding cap changes the objective from "cover everything minimally" to
//! "cover the most with k disks", and the best k disks are generally NOT a
//! subset of any full-coverage tiling. It therefore widens the candidate pool
//! with the complete single-disk arrangement space — every distinct coord
//! plus the pairwise circle-intersection vertices of nearby coords — so each
//! greedy step can pick the true max-gain disk, then polishes with a bounded
//! interchange + relocate loop. The S2 arm keeps pool-only selection (its
//! centers are cell-quantized; there is no continuous placement to
//! synthesize).
//!
//! One shared implementation, applied once in [`super::main`] so EVERY mode
//! (greedy, crucible, fastest, s2, plugins) gets identical cap semantics.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use koji_core::{PointArray, Precision, SingleVec};
use rayon::prelude::*;

use super::geometry::circle_intersections;
use crate::rtree::{self, point::Point};

/// Same margin as the crucible solver/refiner: synthesized centers are placed
/// with r_eff = r·(1−1e-3) so the full-radius scorer can only agree or do
/// better on the points they were constructed to cover.
const MARGIN: Precision = 1.0 - 1e-3;

/// Global budget for synthesized pair-intersection candidates (each pair
/// yields TWO vertices). Per-rep partner fan-out is derived from it so dense
/// inputs can't blow memory; within budget the pair space is complete (every
/// lens vertex of two nearby coords is a candidate).
const PAIR_BUDGET: usize = 3_000_000;
/// Upper bound on partners per rep even when the budget allows more (beyond
/// ~2r there are no partners; this caps pathological local density).
const PER_REP_PARTNER_MAX: usize = 128;
/// Above this many distinct coords, skip vertex synthesis entirely: even the
/// clamped fan-out floor would multiply candidates (and the relocate pass's
/// candidate rtree) into hundreds of MB. Point-anchored candidates + the
/// algorithm pool remain — still a far richer space than pool-only selection.
const SYNTH_MAX_REPS: usize = 200_000;

/// Interchange works on materialized cover lists, so it runs over a thinned
/// pool: the kept centers plus this many runners-up per kept slot, ranked by
/// RESIDUAL gain (weight over points the kept set leaves uncovered) — the
/// candidates that can actually buy coverage in a swap.
const SWAP_POOL_PER_SLOT: usize = 8;
/// Outer polish rounds: each re-ranks the runner pool against the current
/// residual and re-runs the interchange; stops early at a fixpoint.
const SWAP_ROUNDS: usize = 12;

/// Greedy weighted max-coverage: pick ≤ `cap` candidates maximizing the summed
/// weight of covered items. `candidates[i]` lists the dense item indices
/// candidate `i` covers; `item_weights[j]` is item `j`'s weight (e.g. how many
/// input points share that location/cell). Returns the kept candidate indices.
///
/// PRECONDITION: each candidate's item list must be duplicate-free — a repeated
/// index double-counts its weight in the gain sums. Both wrappers below
/// `sort_unstable` + `dedup` before calling.
///
/// Deterministic (gain ties break toward the lower candidate index) and
/// lazy-greedy: gains only shrink as coverage grows (submodularity), so a
/// popped entry whose recomputed gain still tops the heap is the true argmax.
/// Stops early once the best remaining marginal gain is 0 — a zero-gain
/// cluster adds `min_points` of cluster_cost and covers nothing new, so it can
/// only hurt the score.
pub(crate) fn select_max_coverage(
    candidates: &[Vec<u32>],
    item_weights: &[usize],
    cap: usize,
) -> Vec<usize> {
    if candidates.len() <= cap {
        return (0..candidates.len()).collect();
    }
    let mut covered = vec![false; item_weights.len()];
    let mut heap: BinaryHeap<(usize, Reverse<usize>)> = candidates
        .iter()
        .enumerate()
        .map(|(i, items)| {
            let bound: usize = items.iter().map(|&j| item_weights[j as usize]).sum();
            (bound, Reverse(i))
        })
        .collect();
    let mut kept = Vec::with_capacity(cap);
    while kept.len() < cap {
        let Some((bound, Reverse(i))) = heap.pop() else {
            break;
        };
        // The heap max's upper bound is 0 → every remaining candidate is 0.
        if bound == 0 {
            break;
        }
        let fresh: usize = candidates[i]
            .iter()
            .filter(|&&j| !covered[j as usize])
            .map(|&j| item_weights[j as usize])
            .sum();
        if fresh == 0 {
            continue; // fully-covered candidate: drop it, keep popping
        }
        if fresh < bound {
            heap.push((fresh, Reverse(i))); // stale: re-file with the fresh bound
            continue;
        }
        for &j in &candidates[i] {
            covered[j as usize] = true;
        }
        kept.push(i);
    }
    kept
}

/// Keep the best ≤ `cap` cluster centers for RADIUS-disk coverage of `points`.
/// Items are the distinct S2 level-20 cells of `points` at UNIT weight, and a
/// candidate covers a cell when ANY of the cell's points lies within the
/// radius — exactly how the scoreboard counts: `Stats::cluster_stats` collects
/// covered points into a `HashSet<&Point>` whose Eq/Hash is the L20 cell id,
/// so `points_covered` (and therefore the mygod score) is cell-granular.
/// Optimizing anything else (e.g. true per-point coverage) diverges from the
/// reported score on stacked-point data and let a "better" solution score
/// worse.
///
/// The candidate pool is `clusters` PLUS the synthesized arrangement space
/// over the distinct coords (see the module doc): a binding cap means the
/// algorithm's own full-coverage centers are the wrong shape for max-k-cover,
/// and pool-only selection leaves real coverage on the table. Greedy runs
/// lazily against the point rtree (no materialized cover matrix over the
/// large synthesized pool), then a bounded interchange polishes the pick over
/// the kept centers + strongest runners-up. Deterministic throughout.
pub(crate) fn cap_radius_clusters(
    mut clusters: SingleVec,
    points: &SingleVec,
    radius: Precision,
    cap: usize,
) -> SingleVec {
    if clusters.len() <= cap {
        return clusters;
    }
    if points.is_empty() {
        // Nothing to rank by — enforce the cap deterministically.
        clusters.truncate(cap);
        return clusters;
    }
    let t0 = web_time::Instant::now();
    let tree = rtree::spawn(radius, points);
    // One item per distinct L20 cell (scoreboard granularity); the first point
    // seen in a cell is its representative coord for candidate synthesis.
    let mut cell_idx: HashMap<u64, u32> = HashMap::new();
    let mut uniq_coords: SingleVec = Vec::new();
    for p in points {
        let id = Point::new(radius, 20, *p).cell_id.0;
        if let Entry::Vacant(e) = cell_idx.entry(id) {
            e.insert(uniq_coords.len() as u32);
            uniq_coords.push(*p);
        }
    }
    let weights = vec![1usize; uniq_coords.len()];

    // Candidate pool: the algorithm's own centers first (ties in the greedy
    // break toward lower indices, favoring refined positions), then the
    // synthesized arrangement space. No cell-grid dedupe: near-identical
    // vertices can differ by <1 m and that difference decides coverage right
    // at the disk boundary (a coarser-cell dedupe measurably lost coverage).
    let mut cands = clusters.clone();
    cands.extend(arrangement_candidates(&uniq_coords, radius));
    let t_gen = t0.elapsed().as_secs_f64();

    let cover_of = |c: &PointArray| -> Vec<u32> {
        let mut v: Vec<u32> = tree
            .locate_all_at_point(*c)
            .filter_map(|p| cell_idx.get(&p.cell_id.0).copied())
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };

    // Materialize every candidate's cover list ONCE (the rtree query + sort
    // is the hot cost — greedy, interchange and relocate all re-derived these
    // lists before, dominating the wall clock), then drop empty-cover
    // candidates and dedupe candidates with IDENTICAL cover lists: they are
    // interchangeable for selection, and lens vertices duplicate each other
    // heavily. Keep-first preserves the pool-before-synthesized preference.
    // Hash-then-verify so a hash collision can only keep a candidate, never
    // silently drop a distinct one.
    let t1 = web_time::Instant::now();
    let mut all_lists: Vec<Vec<u32>> = cands.par_iter().map(&cover_of).collect();
    let mut by_hash: HashMap<u64, Vec<u32>> = HashMap::new();
    let mut keep_cand: Vec<bool> = vec![false; cands.len()];
    for (i, list) in all_lists.iter().enumerate() {
        if list.is_empty() {
            continue;
        }
        let mut hasher = std::hash::DefaultHasher::new();
        std::hash::Hash::hash(list.as_slice(), &mut hasher);
        let h = std::hash::Hasher::finish(&hasher);
        let bucket = by_hash.entry(h).or_default();
        if bucket.iter().all(|&prev| all_lists[prev as usize] != *list) {
            bucket.push(i as u32);
            keep_cand[i] = true;
        }
    }
    drop(by_hash);
    let mut kept_cands: SingleVec = Vec::new();
    let mut cover_lists: Vec<Vec<u32>> = Vec::new();
    for (i, keep) in keep_cand.iter().enumerate() {
        if *keep {
            kept_cands.push(cands[i]);
            cover_lists.push(std::mem::take(&mut all_lists[i]));
        }
    }
    drop(all_lists);
    let cands = kept_cands;
    let t_cover = t1.elapsed().as_secs_f64();

    let t1 = web_time::Instant::now();
    let mut kept_idx = select_max_coverage(&cover_lists, &weights, cap);
    if kept_idx.is_empty() {
        clusters.truncate(cap);
        return clusters;
    }
    let t_greedy = t1.elapsed().as_secs_f64();

    // Spatial index over the deduped candidates for the relocate pass: which
    // candidates sit within 3r of a kept center (a paired remove-reinsert
    // rarely pays beyond that; far moves are the interchange's job). Tree
    // points carry no candidate index, so hits map back through their exact
    // coords; coincident candidates resolve to the lowest index.
    let t2 = web_time::Instant::now();
    let cand_tree = rtree::spawn(3.0 * radius, &cands);
    let mut cand_of_coord: HashMap<[u64; 2], u32> = HashMap::with_capacity(cands.len());
    for (i, c) in cands.iter().enumerate() {
        cand_of_coord
            .entry([c[0].to_bits(), c[1].to_bits()])
            .or_insert(i as u32);
    }
    let t_tree = t2.elapsed().as_secs_f64();
    let t3 = web_time::Instant::now();

    // Polish rounds: rank the runners-up by RESIDUAL gain against the current
    // kept set (absolute weight ranks dense-area duplicates; residual ranks
    // the candidates that can actually add coverage), materialize cover lists
    // for that small pool, interchange, then relocate (the interchange's blind
    // spot: an inbound that only pays AFTER a specific outbound leaves — its
    // residual gain is ~0 while the outbound still sits there, so no
    // residual-ranked pool ever proposes it). Repeat until nothing moves.
    for _ in 0..SWAP_ROUNDS {
        let mut covered = vec![false; weights.len()];
        let mut is_kept = vec![false; cands.len()];
        for &i in &kept_idx {
            is_kept[i] = true;
            for &j in &cover_lists[i] {
                covered[j as usize] = true;
            }
        }
        let mut runners: Vec<(usize, usize)> = cover_lists
            .par_iter()
            .enumerate()
            .filter(|(i, _)| !is_kept[*i])
            .map(|(i, list)| {
                let residual: usize = list
                    .iter()
                    .filter(|&&j| !covered[j as usize])
                    .map(|&j| weights[j as usize])
                    .sum();
                (residual, i)
            })
            .filter(|&(residual, _)| residual > 0)
            .collect();
        runners.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut pool_idx = kept_idx.clone();
        pool_idx.extend(
            runners
                .into_iter()
                .take(cap.saturating_mul(SWAP_POOL_PER_SLOT))
                .map(|(_, i)| i),
        );
        let pool_cover: Vec<Vec<u32>> = pool_idx.iter().map(|&i| cover_lists[i].clone()).collect();
        // The kept centers occupy the first `kept_idx.len()` pool slots.
        let kept = improve_by_swap((0..kept_idx.len()).collect(), &pool_cover, &weights);
        let new_kept: Vec<usize> = kept.into_iter().map(|i| pool_idx[i]).collect();
        let swapped = new_kept != kept_idx;
        kept_idx = new_kept;
        let relocated = relocate_capped(
            &mut kept_idx,
            &cands,
            &cover_lists,
            &cand_tree,
            &cand_of_coord,
            &weights,
        );
        if !swapped && !relocated {
            break;
        }
    }
    log::info!(
        "cap_radius: {} cands | gen {:.2}s cover {:.2}s greedy {:.2}s tree {:.2}s polish {:.2}s",
        cands.len(),
        t_gen,
        t_cover,
        t_greedy,
        t_tree,
        t3.elapsed().as_secs_f64()
    );
    kept_idx.into_iter().map(|i| cands[i]).collect()
}

/// Paired remove-reinsert at fixed size: for each kept center, the best
/// replacement among the candidates within 3r of it, valued EXACTLY as "items
/// this candidate covers that would go uncovered without the center" (items
/// nothing covers, plus items only that center covers). Replaces the center
/// when the replacement's value beats the center's exclusive weight. Proposes
/// in parallel against a snapshot, applies serially in kept order with exact
/// revalidation (interacting proposals re-price honestly). Returns whether
/// anything moved.
fn relocate_capped(
    kept: &mut [usize],
    cands: &SingleVec,
    cover_lists: &[Vec<u32>],
    cand_tree: &rstar::RTree<Point>,
    cand_of_coord: &HashMap<[u64; 2], u32>,
    item_weights: &[usize],
) -> bool {
    let mut count = vec![0u32; item_weights.len()];
    for &i in kept.iter() {
        for &j in &cover_lists[i] {
            count[j as usize] += 1;
        }
    }
    let mut is_kept = vec![false; cands.len()];
    for &i in kept.iter() {
        is_kept[i] = true;
    }

    // Value of a replacement's cover, given the slot's cover leaves first.
    let value_for = |cov_c: &[u32], cov_i: &[u32], count: &[u32]| -> usize {
        cov_c
            .iter()
            .filter(|&&j| match count[j as usize] {
                0 => true,
                1 => cov_i.binary_search(&j).is_ok(),
                _ => false,
            })
            .map(|&j| item_weights[j as usize])
            .sum()
    };
    let exclusive_w = |cov_i: &[u32], count: &[u32]| -> usize {
        cov_i
            .iter()
            .filter(|&&j| count[j as usize] == 1)
            .map(|&j| item_weights[j as usize])
            .sum()
    };

    // Parallel propose against the snapshot (deterministic: best is tracked by
    // explicit (value desc, index asc) comparison, independent of tree order).
    let count_ref = &count;
    let is_kept_ref = &is_kept;
    let proposals: Vec<(usize, u32)> = kept
        .par_iter()
        .enumerate()
        .filter_map(|(pos, &i)| {
            let cov_i = &cover_lists[i];
            let excl = exclusive_w(cov_i, count_ref);
            let mut best: Option<(usize, u32)> = None; // (value, cand idx)
            for q in cand_tree.locate_all_at_point(cands[i]) {
                let Some(&c_idx) =
                    cand_of_coord.get(&[q.center[0].to_bits(), q.center[1].to_bits()])
                else {
                    continue;
                };
                if is_kept_ref[c_idx as usize] {
                    continue;
                }
                let val = value_for(&cover_lists[c_idx as usize], cov_i, count_ref);
                if val > excl && best.is_none_or(|(bv, bc)| val > bv || (val == bv && c_idx < bc)) {
                    best = Some((val, c_idx));
                }
            }
            best.map(|(_, c_idx)| (pos, c_idx))
        })
        .collect();

    // Serial apply in kept order, revalidating against the live state.
    let mut changed = false;
    for (pos, c_idx) in proposals {
        let c_idx = c_idx as usize;
        if is_kept[c_idx] {
            continue;
        }
        let i = kept[pos];
        let cov_i = &cover_lists[i];
        let cov_c = &cover_lists[c_idx];
        if value_for(cov_c, cov_i, &count) <= exclusive_w(cov_i, &count) {
            continue;
        }
        for &j in cov_i.iter() {
            count[j as usize] -= 1;
        }
        for &j in cov_c.iter() {
            count[j as usize] += 1;
        }
        is_kept[i] = false;
        is_kept[c_idx] = true;
        kept[pos] = c_idx;
        changed = true;
    }
    changed
}

/// The complete single-disk candidate space over the distinct coords: every
/// coord itself plus the two radius-r_eff circle-intersection vertices of
/// every nearby coord pair. A disk maximizing covered weight can always be
/// moved so it either sits on a point or has two points on its boundary, so
/// within the pair budget this space contains a true per-step optimum.
fn arrangement_candidates(reps: &SingleVec, radius: Precision) -> SingleVec {
    let r_eff = radius * MARGIN;
    let mut out: SingleVec = reps.clone();
    if reps.len() < 2 || reps.len() > SYNTH_MAX_REPS {
        return out;
    }
    // Neighbor discovery via a 2·r_eff "coverage" tree: locate_all_at_point
    // returns exactly the reps within 2·r_eff (exact Haversine contains).
    let pair_tree = rtree::spawn(2.0 * r_eff, reps);
    let idx_of: HashMap<[u64; 2], u32> = reps
        .iter()
        .enumerate()
        .map(|(i, p)| ([p[0].to_bits(), p[1].to_bits()], i as u32))
        .collect();
    // Derived per-rep fan-out keeps the global candidate count bounded
    // (÷2 because every pair emits two intersection vertices).
    let per_rep_cap = (PAIR_BUDGET / 2 / reps.len()).clamp(4, PER_REP_PARTNER_MAX);

    let pairs: Vec<[PointArray; 2]> = reps
        .par_iter()
        .enumerate()
        .flat_map_iter(|(i, p)| {
            let mut partners: Vec<u32> = pair_tree
                .locate_all_at_point(*p)
                .filter_map(|q| {
                    idx_of
                        .get(&[q.center[0].to_bits(), q.center[1].to_bits()])
                        .copied()
                })
                .filter(|&j| j as usize > i)
                .collect();
            partners.sort_unstable();
            partners.dedup();
            // Deterministic thinning: closest partners first, index tiebreak.
            if partners.len() > per_rep_cap {
                let frame = LocalFrame::new(*p);
                let xy0 = frame.to_xy(*p);
                partners.sort_unstable_by(|&a, &b| {
                    let da = dist2_planar(xy0, frame.to_xy(reps[a as usize]));
                    let db = dist2_planar(xy0, frame.to_xy(reps[b as usize]));
                    da.total_cmp(&db).then(a.cmp(&b))
                });
                partners.truncate(per_rep_cap);
            }
            let p = *p;
            partners
                .into_iter()
                .map(move |j| [p, reps[j as usize]])
                .collect::<Vec<_>>()
        })
        .collect();

    let vertices: Vec<PointArray> = pairs
        .par_iter()
        .flat_map_iter(|[a, b]| {
            let frame = LocalFrame::new(*a);
            circle_intersections(frame.to_xy(*a), frame.to_xy(*b), r_eff)
                .map(|[v1, v2]| vec![frame.to_latlng(v1), frame.to_latlng(v2)])
                .unwrap_or_default()
        })
        .collect();
    out.extend(vertices);
    out
}

/// Local equirectangular frame for pair-vertex synthesis (meters), accurate to
/// ~1e-8 relative over the ≤ ~300 m extents involved.
struct LocalFrame {
    origin: PointArray,
    m_per_deg_lon: Precision,
}

const M_PER_DEG_LAT: Precision = 111_132.0;

impl LocalFrame {
    fn new(origin: PointArray) -> Self {
        LocalFrame {
            origin,
            m_per_deg_lon: 111_320.0 * origin[0].to_radians().cos().abs().max(1e-9),
        }
    }
    fn to_xy(&self, p: PointArray) -> [Precision; 2] {
        [
            (p[1] - self.origin[1]) * self.m_per_deg_lon,
            (p[0] - self.origin[0]) * M_PER_DEG_LAT,
        ]
    }
    fn to_latlng(&self, xy: [Precision; 2]) -> PointArray {
        [
            self.origin[0] + xy[1] / M_PER_DEG_LAT,
            self.origin[1] + xy[0] / self.m_per_deg_lon,
        ]
    }
}

fn dist2_planar(a: [Precision; 2], b: [Precision; 2]) -> Precision {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

/// Keep the best ≤ `cap` S2-cell cluster centers. Items are the level-`level`
/// cells that contain input points, weighted by their point counts; a
/// candidate covers its `size`×`size` neighborhood (mirrors the s2 clusterer's
/// own counting). Preserves the input order of the kept centers.
pub(crate) fn cap_s2_clusters(
    mut clusters: SingleVec,
    points: &SingleVec,
    level: u8,
    size: u8,
    cap: usize,
) -> SingleVec {
    if clusters.len() <= cap {
        return clusters;
    }
    if points.is_empty() {
        clusters.truncate(cap);
        return clusters;
    }
    let mut cell_idx: HashMap<u64, u32> = HashMap::new();
    let mut weights: Vec<usize> = Vec::new();
    for p in points {
        let id = ::s2::cellid::CellID::from(::s2::latlng::LatLng::from_degrees(p[0], p[1]))
            .parent(level as u64)
            .0;
        match cell_idx.entry(id) {
            Entry::Occupied(e) => weights[*e.get() as usize] += 1,
            Entry::Vacant(e) => {
                e.insert(weights.len() as u32);
                weights.push(1);
            }
        }
    }
    let cover: Vec<Vec<u32>> = clusters
        .iter()
        .map(|c| {
            let mut v: Vec<u32> = koji_core::s2::cell_coverage(c[0], c[1], size, level)
                .iter()
                .filter_map(|id| cell_idx.get(id).copied())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();
    let kept = improve_by_swap(select_max_coverage(&cover, &weights, cap), &cover, &weights);
    keep_indices(clusters, kept, cap)
}

/// Post-selection interchange: at FIXED size, repeatedly swap a kept candidate
/// with small exclusive contribution for an unkept candidate with large
/// uncovered gain, when the exact delta is positive. Repairs greedy's early
/// commits (it can't undo a pick). Each iteration tries the top few inbound
/// candidates by gain — pairing only the single best inbound with the single
/// cheapest outbound stalls on instances where that one pairing loses but the
/// runner-up inbound wins. Bounded passes, deterministic.
pub(crate) fn improve_by_swap(
    mut kept: Vec<usize>,
    candidates: &[Vec<u32>],
    item_weights: &[usize],
) -> Vec<usize> {
    if kept.is_empty() || kept.len() >= candidates.len() {
        return kept;
    }
    /// Inbound candidates to try per iteration before concluding no swap helps.
    const TRY_INBOUND: usize = 24;
    // cover_count[j] = how many kept candidates cover item j.
    let mut count = vec![0u32; item_weights.len()];
    let mut is_kept = vec![false; candidates.len()];
    for &k in &kept {
        is_kept[k] = true;
        for &j in &candidates[k] {
            count[j as usize] += 1;
        }
    }
    let mut in_covers = vec![false; item_weights.len()];
    // A swap can enable further swaps; bound the loop so a cycle can't spin.
    'outer: for _ in 0..kept.len().max(64) {
        // Inbound candidates by uncovered gain, best first (index tiebreak).
        let mut inbound: Vec<(usize, usize)> = candidates
            .iter()
            .enumerate()
            .filter(|(c, _)| !is_kept[*c])
            .map(|(c, items)| {
                let gain: usize = items
                    .iter()
                    .filter(|&&j| count[j as usize] == 0)
                    .map(|&j| item_weights[j as usize])
                    .sum();
                (gain, c)
            })
            .filter(|&(gain, _)| gain > 0)
            .collect();
        inbound.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        for &(gain, c_in) in inbound.iter().take(TRY_INBOUND) {
            // Cheapest outbound: min summed weight of items ONLY it covers —
            // ignoring items the inbound candidate would re-cover (exact delta).
            for &j in &candidates[c_in] {
                in_covers[j as usize] = true;
            }
            let mut best_out: Option<(usize, usize)> = None; // (loss, kept idx position)
            for (pos, &k) in kept.iter().enumerate() {
                let loss: usize = candidates[k]
                    .iter()
                    .filter(|&&j| count[j as usize] == 1 && !in_covers[j as usize])
                    .map(|&j| item_weights[j as usize])
                    .sum();
                if best_out.is_none_or(|(l, bp)| loss < l || (loss == l && kept[pos] < kept[bp])) {
                    best_out = Some((loss, pos));
                }
            }
            for &j in &candidates[c_in] {
                in_covers[j as usize] = false;
            }
            let Some((loss, pos)) = best_out else {
                continue;
            };
            if gain <= loss {
                continue; // this inbound can't pay for any outbound — try next
            }
            let k_out = kept[pos];
            for &j in &candidates[k_out] {
                count[j as usize] -= 1;
            }
            for &j in &candidates[c_in] {
                count[j as usize] += 1;
            }
            is_kept[k_out] = false;
            is_kept[c_in] = true;
            kept[pos] = c_in;
            continue 'outer;
        }
        break; // none of the top inbound candidates has a profitable pairing
    }
    kept
}

/// Filter `clusters` down to the selected indices (input order preserved).
/// Safety net: if selection kept nothing (all candidates covered zero items —
/// bogus centers), fall back to a deterministic truncate rather than erasing
/// the whole solution.
fn keep_indices(mut clusters: SingleVec, kept: Vec<usize>, cap: usize) -> SingleVec {
    if kept.is_empty() {
        clusters.truncate(cap);
        return clusters;
    }
    let mut keep = vec![false; clusters.len()];
    for i in kept {
        keep[i] = true;
    }
    let mut idx = 0;
    clusters.retain(|_| {
        let k = keep[idx];
        idx += 1;
        k
    });
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marginal_gain_beats_absolute_coverage() {
        // The overlap trap: A and B both cover items 0..9; C covers 10..14.
        // Absolute top-2 keeps {A, B} (10, 10) and leaves C's items uncovered.
        // Marginal keeps {A, C} (10 + 5 > 10 + 0).
        let candidates = vec![
            (0..10).collect::<Vec<u32>>(),
            (0..10).collect::<Vec<u32>>(),
            (10..15).collect::<Vec<u32>>(),
        ];
        let weights = vec![1; 15];
        assert_eq!(select_max_coverage(&candidates, &weights, 2), vec![0, 2]);
    }

    #[test]
    fn weighted_items_drive_selection() {
        // One heavy item (10 points at the same spot) beats two light ones.
        let candidates = vec![vec![0], vec![1, 2]];
        let weights = vec![10, 1, 1];
        assert_eq!(select_max_coverage(&candidates, &weights, 1), vec![0]);
    }

    #[test]
    fn zero_marginal_gain_stops_selection() {
        // Three identical candidates: after the first, the rest add nothing —
        // keep 1, not cap (a zero-gain cluster only costs score).
        let candidates = vec![vec![0], vec![0], vec![0]];
        let weights = vec![1];
        assert_eq!(select_max_coverage(&candidates, &weights, 2), vec![0]);
    }

    #[test]
    fn greedy_is_approximate_not_optimal() {
        // Documents the (1−1/e) approximation, NOT a bug: A={0,1,2,3},
        // B={0,1,4}, C={2,3,5}, cap 2. Greedy takes A (gain 4) then B (gain 1)
        // → weight 5; the optimum {B, C} covers all 6. If a future change makes
        // this return [1, 2], the selector became BETTER — update the test.
        let candidates = vec![vec![0, 1, 2, 3], vec![0, 1, 4], vec![2, 3, 5]];
        let weights = vec![1; 6];
        assert_eq!(select_max_coverage(&candidates, &weights, 2), vec![0, 1]);
    }

    #[test]
    fn ties_break_toward_lower_index_deterministically() {
        let candidates = vec![vec![0], vec![1], vec![2]];
        let weights = vec![1, 1, 1];
        assert_eq!(select_max_coverage(&candidates, &weights, 2), vec![0, 1]);
    }

    #[test]
    fn under_cap_is_identity() {
        let candidates = vec![vec![0], vec![1]];
        let weights = vec![1, 1];
        assert_eq!(select_max_coverage(&candidates, &weights, 5), vec![0, 1]);
    }

    #[test]
    fn swap_replaces_dominated_pick() {
        // kept = just B ({0}); C ({1,2,3}) gains 3 for a loss of 1 → swapped in.
        let candidates = vec![vec![0, 1, 2], vec![0], vec![1, 2, 3]];
        let weights = vec![1; 4];
        assert_eq!(improve_by_swap(vec![1], &candidates, &weights), vec![2]);
    }

    #[test]
    fn swap_leaves_optimal_selection_alone() {
        // kept = A ({0,1}); B ({0}) has zero uncovered gain → no swap.
        let candidates = vec![vec![0, 1], vec![0]];
        let weights = vec![1; 2];
        assert_eq!(improve_by_swap(vec![0], &candidates, &weights), vec![0]);
    }

    /// Count points within `radius` m of any center (Haversine).
    fn covered_count(centers: &SingleVec, points: &SingleVec, radius: Precision) -> usize {
        use geo::{Distance, Haversine};
        points
            .iter()
            .filter(|p| {
                centers.iter().any(|c| {
                    Haversine.distance(geo::Point::new(c[1], c[0]), geo::Point::new(p[1], p[0]))
                        <= radius
                })
            })
            .count()
    }

    #[test]
    fn cap_radius_keeps_dense_disjoint_centers() {
        // Dense clump (5 pts) + sparse clump (2 pts) + a duplicate center over
        // the dense clump. cap=2 → both clumps fully covered; the duplicate
        // (zero marginal) never eats the second slot. Exact coords are the
        // selector's business now that it can synthesize better centers.
        let mut points: SingleVec = Vec::new();
        for j in 0..5 {
            points.push([40.0 + j as Precision * 0.0001, -74.0]); // ~11 m apart
        }
        points.push([41.0, -74.0]);
        points.push([41.0001, -74.0]);
        let clusters: SingleVec = vec![
            [40.0002, -74.0],  // dense center
            [40.0002, -74.0],  // duplicate of it
            [41.00005, -74.0], // sparse center
        ];
        let kept = cap_radius_clusters(clusters, &points, 70.0, 2);
        assert_eq!(kept.len(), 2);
        assert_eq!(covered_count(&kept, &points, 70.0), 7);
    }

    #[test]
    fn cap_radius_synthesizes_better_centers_than_the_pool() {
        // Three points in a row, 60 m apart: a disk at the MIDDLE point covers
        // all three, but the pool only offers centers on the two OUTER points
        // (2 covered each). cap=1 must find the middle placement — pool-only
        // selection cannot (this was the capped-quality gap vs main's greedy).
        let step = 0.00054; // ~60 m of latitude
        let points: SingleVec = vec![
            [40.0, -74.0],
            [40.0 + step, -74.0],
            [40.0 + 2.0 * step, -74.0],
        ];
        let clusters: SingleVec = vec![points[0], points[2]];
        let kept = cap_radius_clusters(clusters, &points, 70.0, 1);
        assert_eq!(kept.len(), 1);
        assert_eq!(
            covered_count(&kept, &points, 70.0),
            3,
            "cap=1 must place the disk where it covers all three points, got {kept:?}"
        );
    }

    #[test]
    fn cap_radius_pair_vertices_beat_point_anchored_disks() {
        // Two pairs ~110 m apart (each pair ~22 m wide, so every point is its
        // own L20 cell): no single POINT-anchored disk covers more than one
        // pair, but a disk at the circle-intersection vertex between the outer
        // points does. cap=1 must cover all 4 via a synthesized vertex.
        let pair = 0.0002; // ~22 m of latitude
        let d = 0.00099; // ~110 m of latitude
        let points: SingleVec = vec![
            [40.0, -74.0],
            [40.0 + pair, -74.0],
            [40.0 + d, -74.0],
            [40.0 + d + pair, -74.0],
        ];
        let clusters: SingleVec = vec![points[0], points[2]];
        let kept = cap_radius_clusters(clusters, &points, 70.0, 1);
        assert_eq!(kept.len(), 1);
        assert_eq!(
            covered_count(&kept, &points, 70.0),
            4,
            "cap=1 must use a pair-intersection vertex, got {kept:?}"
        );
    }

    #[test]
    fn cap_radius_is_deterministic() {
        // Pseudo-random 200-point cloud, cap 5 — two runs must agree exactly.
        let mut pts: SingleVec = Vec::new();
        let mut x: u64 = 99;
        for _ in 0..200 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = 40.0 + ((x >> 11) as Precision / (1u64 << 53) as Precision) * 0.01;
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let b = -74.0 + ((x >> 11) as Precision / (1u64 << 53) as Precision) * 0.01;
            pts.push([a, b]);
        }
        let clusters: SingleVec = pts.iter().step_by(20).copied().collect();
        let a = cap_radius_clusters(clusters.clone(), &pts, 70.0, 5);
        let b = cap_radius_clusters(clusters, &pts, 70.0, 5);
        assert_eq!(a, b);
        assert_eq!(a.len(), 5);
    }

    #[test]
    fn cap_radius_under_cap_untouched() {
        let points: SingleVec = vec![[40.0, -74.0]];
        let clusters: SingleVec = vec![[40.0, -74.0]];
        assert_eq!(
            cap_radius_clusters(clusters.clone(), &points, 70.0, 5),
            clusters
        );
    }

    #[test]
    fn cap_radius_empty_points_truncates() {
        let clusters: SingleVec = vec![[1.0, 1.0], [2.0, 2.0], [3.0, 3.0]];
        assert_eq!(cap_radius_clusters(clusters, &vec![], 70.0, 2).len(), 2);
    }
}
