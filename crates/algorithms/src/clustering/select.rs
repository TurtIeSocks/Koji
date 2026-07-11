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
//! One shared implementation, applied once in [`super::main`] so EVERY mode
//! (greedy, crucible, fastest, s2, plugins) gets identical cap semantics.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use hashbrown::HashMap;
use hashbrown::hash_map::Entry;
use koji_core::{Precision, SingleVec};

use crate::rtree::{self, point::Point};

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
/// Items are the distinct level-20 cells of `points`, weighted by how many
/// input points share the cell (so duplicated locations count once per point).
/// Preserves the input order of the kept centers.
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
    let tree = rtree::spawn(radius, points);
    let mut cell_idx: HashMap<u64, u32> = HashMap::new();
    let mut weights: Vec<usize> = Vec::new();
    for p in points {
        let id = Point::new(radius, 20, *p).cell_id.0;
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
            let mut v: Vec<u32> = tree
                .locate_all_at_point(*c)
                .filter_map(|p| cell_idx.get(&p.cell_id.0).copied())
                .collect();
            v.sort_unstable();
            v.dedup();
            v
        })
        .collect();
    keep_indices(clusters, select_max_coverage(&cover, &weights, cap), cap)
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
    keep_indices(clusters, select_max_coverage(&cover, &weights, cap), cap)
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
    fn cap_radius_keeps_dense_disjoint_centers() {
        // Dense clump (5 pts) + sparse clump (2 pts) + a duplicate center over
        // the dense clump. cap=2 → the dense center + the sparse one; the
        // duplicate (zero marginal) is dropped.
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
        assert_eq!(kept, vec![[40.0002, -74.0], [41.00005, -74.0]]);
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
