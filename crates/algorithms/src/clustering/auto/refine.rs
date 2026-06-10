//! Global refinement passes over the stitched chunk solutions.
//!
//! Operates in lat/lng space with exact (conservatively-margined) Haversine
//! coverage via the same rtree machinery the scorer uses, so every accepted
//! move is a true mygod_score improvement:
//!
//! - **drop**: remove a center whose exclusive coverage < m (Δ = excl − m < 0)
//! - **relocate**: move a center, keeping its exclusive set covered, to the
//!   position capturing the most uncovered points (Δ ≤ 0)
//! - **merge**: replace two centers by one when the points only they cover
//!   fit a single disk (Δ = −m)
//! - **swap 3→2**: replace three mutually-near centers by two when their
//!   required points split across two disks (Δ = −m; runs at stalls only)
//! - **gap-fill**: greedy add centers covering ≥ m uncovered points (Δ ≤ 0)
//! - **LNS**: ruin-and-recreate windows around uncovered points — remove the
//!   centers inside, re-solve locally, accept strict wins (stalls only)
//!
//! Score Δ math: see stats.rs::Stats::get_score (clusters·m + uncovered).

use geo::{Distance, Haversine};
use hashbrown::HashMap;
use rayon::prelude::*;
use s2::{cellid::CellID, latlng::LatLng};

use koji_core::{PointArray, Precision, SingleVec};

use super::geometry::{circle_intersections, smallest_enclosing_circle};
use crate::rtree;

/// Inputs up to this many cells get full-sweep LNS (every center cell
/// repacked, not just uncovered neighborhoods).
const LNS_SWEEP_ALL_MAX: usize = 25_000;

/// Same margin as the solver: all refinement decisions use r_eff = r·(1−1e-3)
/// so the full-radius scorer can only agree or do better.
const MARGIN: Precision = 1.0 - 1e-3;

/// Local equirectangular frame, accurate to ~1e-8 relative over the ≤ ~600 m
/// extents it is used for (pair merges, single-cluster exclusive sets).
struct LocalFrame {
    origin: PointArray,
    m_per_deg_lat: Precision,
    m_per_deg_lon: Precision,
}

impl LocalFrame {
    fn new(origin: PointArray) -> Self {
        let lat = origin[0].to_radians();
        LocalFrame {
            origin,
            m_per_deg_lat: 111_132.92 - 559.82 * (2.0 * lat).cos() + 1.175 * (4.0 * lat).cos(),
            m_per_deg_lon: 111_412.84 * lat.cos() - 93.5 * (3.0 * lat).cos(),
        }
    }
    fn to_xy(&self, p: PointArray) -> [f64; 2] {
        [
            (p[1] - self.origin[1]) * self.m_per_deg_lon,
            (p[0] - self.origin[0]) * self.m_per_deg_lat,
        ]
    }
    fn to_latlng(&self, xy: [f64; 2]) -> PointArray {
        [
            self.origin[0] + xy[1] / self.m_per_deg_lat,
            self.origin[1] + xy[0] / self.m_per_deg_lon,
        ]
    }
}

fn haversine_m(a: PointArray, b: PointArray) -> Precision {
    Haversine.distance(geo::Point::new(a[1], a[0]), geo::Point::new(b[1], b[0]))
}

/// An accepted-by-the-proposer LNS window rewrite, pending serial commit.
struct LnsProposal {
    removed: Vec<usize>,
    /// Sorted local point set (rep indices).
    local: Vec<u32>,
    /// Per-local-point: covered by a proposed center (planar, RHO).
    prop_mask: Vec<bool>,
    centers: Vec<PointArray>,
    /// Annealing slack this window may regress by (0 when not annealing).
    slack: usize,
}

/// Result of evaluating one LNS window.
enum LnsOutcome {
    /// Nothing to do (or no improvement found).
    Skip,
    /// Lost set too big for the exact tier — descend into child cells.
    Split,
    /// A strict (or anneal-slack) improvement.
    Proposal(LnsProposal),
}

/// 2·RHO-connected components of a planar point set (radius units), by
/// index. O(n²) union-find — callers cap n.
fn planar_components(pts: &[[f64; 2]], reach: f64) -> Vec<Vec<usize>> {
    let n = pts.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut root = i;
        while parent[root] != root {
            root = parent[root];
        }
        let mut cur = i;
        while parent[cur] != root {
            let next = parent[cur];
            parent[cur] = root;
            cur = next;
        }
        root
    }
    let reach2 = reach * reach;
    for a in 0..n {
        for b in (a + 1)..n {
            let d2 = (pts[a][0] - pts[b][0]).powi(2) + (pts[a][1] - pts[b][1]).powi(2);
            if d2 <= reach2 {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
                    parent[hi] = lo;
                }
            }
        }
    }
    let mut comps: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        comps.entry(r).or_default().push(i);
    }
    let mut out: Vec<Vec<usize>> = comps.into_values().collect();
    out.sort_by_key(|c| c[0]);
    out
}

/// Lat/lng bucket grid over the reps with row-scaled longitude cells —
/// replaces the rstar tree in the refiner's hot loops (~2–3× per query).
/// All distance predicates remain exact Haversine; the grid only prunes.
struct GeoGrid {
    cell_m: Precision,
    dlat: Precision,
    buckets: HashMap<(i32, i32), Vec<u32>>,
}

const M_PER_DEG_LAT: Precision = 111_132.0;
const M_PER_DEG_LON_EQ: Precision = 111_320.0;

impl GeoGrid {
    fn build(reps: &SingleVec, cell_m: Precision) -> GeoGrid {
        let dlat = cell_m / M_PER_DEG_LAT;
        let mut grid = GeoGrid {
            cell_m,
            dlat,
            buckets: HashMap::with_capacity(reps.len() / 2),
        };
        for (i, p) in reps.iter().enumerate() {
            let key = grid.key_of(*p);
            grid.buckets.entry(key).or_default().push(i as u32);
        }
        grid
    }

    fn row_of(&self, lat: Precision) -> i32 {
        (lat / self.dlat).floor() as i32
    }

    fn dlon_for_row(&self, row: i32) -> Precision {
        let lat = (row as Precision + 0.5) * self.dlat;
        self.cell_m / (M_PER_DEG_LON_EQ * lat.to_radians().cos().abs().max(0.01))
    }

    fn key_of(&self, p: PointArray) -> (i32, i32) {
        let row = self.row_of(p[0]);
        let col = (p[1] / self.dlon_for_row(row)).floor() as i32;
        (row, col)
    }

    /// Visit every rep index within `dist_m` (exact Haversine) of `q`.
    fn for_each_within(
        &self,
        reps: &SingleVec,
        q: PointArray,
        dist_m: Precision,
        mut f: impl FnMut(u32),
    ) {
        let dlat_q = dist_m / M_PER_DEG_LAT;
        let min_row = self.row_of(q[0] - dlat_q);
        let max_row = self.row_of(q[0] + dlat_q);
        let dlon_q = dist_m / (M_PER_DEG_LON_EQ * q[0].to_radians().cos().abs().max(0.01));
        // Longitude ranges, split when the query crosses the antimeridian.
        let (lo, hi) = (q[1] - dlon_q, q[1] + dlon_q);
        let mut ranges: [(Precision, Precision); 2] = [(lo, hi), (0.0, -1.0)];
        if lo < -180.0 {
            ranges = [(-180.0, hi), (lo + 360.0, 180.0)];
        } else if hi > 180.0 {
            ranges = [(lo, 180.0), (-180.0, hi - 360.0)];
        }
        for row in min_row..=max_row {
            let dlon = self.dlon_for_row(row);
            for &(rlo, rhi) in &ranges {
                if rlo > rhi {
                    continue;
                }
                let min_col = (rlo / dlon).floor() as i32;
                let max_col = (rhi / dlon).floor() as i32;
                for col in min_col..=max_col {
                    if let Some(bucket) = self.buckets.get(&(row, col)) {
                        for &i in bucket {
                            if haversine_m(q, reps[i as usize]) <= dist_m {
                                f(i);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Visit every rep index inside a lat/lon bbox (no distance filter).
    fn for_each_in_bbox(
        &self,
        reps: &SingleVec,
        min_lat: Precision,
        max_lat: Precision,
        min_lon: Precision,
        max_lon: Precision,
        mut f: impl FnMut(u32),
    ) {
        for row in self.row_of(min_lat)..=self.row_of(max_lat) {
            let dlon = self.dlon_for_row(row);
            let min_col = (min_lon / dlon).floor() as i32;
            let max_col = (max_lon / dlon).floor() as i32;
            for col in min_col..=max_col {
                if let Some(bucket) = self.buckets.get(&(row, col)) {
                    for &i in bucket {
                        let p = reps[i as usize];
                        if p[0] >= min_lat && p[0] <= max_lat && p[1] >= min_lon && p[1] <= max_lon
                        {
                            f(i);
                        }
                    }
                }
            }
        }
    }
}

pub struct Refiner<'a> {
    reps: &'a SingleVec,
    grid: GeoGrid,
    r_eff: Precision,
    m: usize,
    /// Per-center state; killed centers have `live == false`.
    pos: Vec<PointArray>,
    covered: Vec<Vec<u32>>,
    live: Vec<bool>,
    /// Per-rep count of live centers covering it.
    count: Vec<u32>,
    /// Dirty-region bookkeeping: S2 cells (edge ≥ ~8r) get their epoch bumped
    /// whenever a mutation lands nearby; passes skip centers whose region is
    /// unchanged since their last scan. Pure scheduling — never affects which
    /// moves are legal, only when they are looked for.
    region_level: u64,
    region_epoch: HashMap<u64, u64>,
    epoch: u64,
    relocate_seen: HashMap<u64, u64>,
    merge_seen: HashMap<u64, u64>,
    swap_seen: HashMap<u64, u64>,
    /// Annealing temperature + salt for the current LNS pass (None = strict
    /// improvement only). Set only by the flag-gated anneal phase.
    anneal_state: Option<(Precision, u64)>,
}

impl<'a> Refiner<'a> {
    pub fn new(
        centers: Vec<PointArray>,
        reps: &'a SingleVec,
        radius: Precision,
        min_points: usize,
    ) -> Self {
        let r_eff = radius * MARGIN;
        let grid = GeoGrid::build(reps, r_eff.max(1.0));

        // Deepest S2 level whose average edge is still ≥ 8r, so a 3×3
        // footprint bump reaches every center that could interact with a
        // mutation (merge gate is 4r, coverage is r).
        let mut region_level: u64 = 0;
        while region_level < 18
            && 8_000_000.0 / (1u64 << (region_level + 1)) as Precision >= 8.0 * r_eff
        {
            region_level += 1;
        }

        let mut refiner = Refiner {
            reps,
            grid,
            r_eff,
            m: min_points.max(1),
            pos: Vec::new(),
            covered: Vec::new(),
            live: Vec::new(),
            count: vec![0; reps.len()],
            region_level,
            region_epoch: HashMap::new(),
            epoch: 0,
            relocate_seen: HashMap::new(),
            merge_seen: HashMap::new(),
            swap_seen: HashMap::new(),
            anneal_state: None,
        };

        // Initial coverage, computed in parallel then applied serially.
        let cov: Vec<Vec<u32>> = centers
            .par_iter()
            .map(|c| refiner.query_covered(*c))
            .collect();
        for (c, cv) in centers.into_iter().zip(cov) {
            refiner.add_center(c, cv);
        }
        refiner
    }

    /// All rep indices within r_eff (exact Haversine) of `p`, sorted.
    fn query_covered(&self, p: PointArray) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        self.grid
            .for_each_within(self.reps, p, self.r_eff, |i| out.push(i));
        out.sort_unstable();
        out
    }

    fn add_center(&mut self, p: PointArray, cov: Vec<u32>) -> usize {
        for &r in &cov {
            self.count[r as usize] += 1;
        }
        self.pos.push(p);
        self.covered.push(cov);
        self.live.push(true);
        self.bump_footprint(p);
        self.pos.len() - 1
    }

    fn kill_center(&mut self, i: usize) {
        // Hard assert: a double-kill silently underflows the u32 coverage
        // counts in release builds (seen via a duplicated swap32 trio index).
        assert!(self.live[i], "kill_center called on dead center {i}");
        self.live[i] = false;
        for &r in &self.covered[i] {
            self.count[r as usize] -= 1;
        }
        self.bump_footprint(self.pos[i]);
    }

    /// Region cell of a position at the dirty-tracking level.
    fn region_of(&self, p: PointArray) -> u64 {
        CellID::from(LatLng::from_degrees(p[0], p[1]))
            .parent(self.region_level)
            .0
    }

    /// Mark the 3×3 region neighborhood of `p` as changed. Neighbor cells are
    /// reached by ±6r coordinate offsets (no S2 neighbor API needed; the
    /// region edge is ≥ 8r so the offsets land in the adjacent cells).
    fn bump_footprint(&mut self, p: PointArray) {
        self.epoch += 1;
        let dlat = 6.0 * self.r_eff / 111_132.0;
        let dlon = 6.0 * self.r_eff / (111_320.0 * p[0].to_radians().cos().abs().max(0.01));
        for di in -1i8..=1 {
            for dj in -1i8..=1 {
                let q = [
                    (p[0] + di as f64 * dlat).clamp(-89.999, 89.999),
                    p[1] + dj as f64 * dlon,
                ];
                let cell = self.region_of(q);
                self.region_epoch.insert(cell, self.epoch);
            }
        }
    }

    /// Has `p`'s region changed since this pass last scanned it?
    fn is_dirty(&self, seen: &HashMap<u64, u64>, p: PointArray) -> bool {
        let cell = self.region_of(p);
        let current = self.region_epoch.get(&cell).copied().unwrap_or(1);
        current > seen.get(&cell).copied().unwrap_or(0)
    }

    /// Record the scanned cells' epochs so the next pass run skips them
    /// unless something bumps them again.
    fn mark_seen(seen: &mut HashMap<u64, u64>, snapshot: Vec<(u64, u64)>) {
        for (cell, ep) in snapshot {
            seen.insert(cell, ep);
        }
    }

    fn exclusive_of(&self, i: usize) -> Vec<u32> {
        self.covered[i]
            .iter()
            .copied()
            .filter(|&r| self.count[r as usize] == 1)
            .collect()
    }

    /// Drop centers whose exclusive coverage is below m. Each drop is a strict
    /// score win; processed in ascending exclusive-count order because drops
    /// only grow other centers' exclusive sets.
    fn drop_pass(&mut self) -> bool {
        let mut changed = false;
        loop {
            let mut eligible: Vec<(usize, usize)> = (0..self.pos.len())
                .filter(|&i| self.live[i])
                .map(|i| (self.exclusive_of(i).len(), i))
                .filter(|&(e, _)| e < self.m)
                .collect();
            if eligible.is_empty() {
                return changed;
            }
            eligible.sort_unstable();
            let mut dropped_this_sweep = false;
            for (_, i) in eligible {
                if !self.live[i] {
                    continue;
                }
                // Re-check: earlier drops in this sweep may have grown it.
                if self.exclusive_of(i).len() < self.m {
                    self.kill_center(i);
                    changed = true;
                    dropped_this_sweep = true;
                }
            }
            if !dropped_this_sweep {
                return changed;
            }
        }
    }

    /// Relocate each center, keeping every exclusive point covered, to the
    /// position capturing the most still-uncovered points (−1 each).
    /// Candidate positions: the SEC center of the exclusive set plus local
    /// arrangement vertices over (exclusive extremes ∪ nearby uncovered).
    /// Non-exclusive covered points are never lost (another coverer exists
    /// by definition), so every accepted move is a true score improvement.
    fn relocate_pass(&mut self) -> bool {
        // Dirty-region gating: only centers whose neighborhood changed since
        // this pass last scanned them are searched again.
        let candidates: Vec<usize> = (0..self.pos.len())
            .filter(|&i| self.live[i] && self.is_dirty(&self.relocate_seen, self.pos[i]))
            .collect();
        let snapshot: Vec<(u64, u64)> = {
            let cells: HashMap<u64, u64> = candidates
                .iter()
                .map(|&i| {
                    let cell = self.region_of(self.pos[i]);
                    let ep = self.region_epoch.get(&cell).copied().unwrap_or(1);
                    (cell, ep)
                })
                .collect();
            cells.into_iter().collect()
        };

        // Propose in parallel against the current snapshot (the search is the
        // expensive part), then apply serially in index order with cheap
        // revalidation — deterministic and race-free.
        let this = &*self;
        let proposals: Vec<(usize, PointArray)> = candidates
            .par_iter()
            .filter_map(|&i| this.propose_relocate(i).map(|cand| (i, cand)))
            .collect();

        let mut changed = false;
        for (i, cand) in proposals {
            if !self.live[i] {
                continue;
            }
            // Revalidate against current state: every exclusive point must
            // stay covered, and the move must still strictly improve.
            let excl = self.exclusive_of(i);
            if excl
                .iter()
                .any(|&r| haversine_m(cand, self.reps[r as usize]) > self.r_eff)
            {
                continue;
            }
            let new_cov = self.query_covered(cand);
            let captured = new_cov
                .iter()
                .filter(|&&r| self.count[r as usize] == 0)
                .count();
            if captured > 0 || new_cov.len() > self.covered[i].len() {
                for &r in &self.covered[i] {
                    self.count[r as usize] -= 1;
                }
                for &r in &new_cov {
                    self.count[r as usize] += 1;
                }
                let old_pos = self.pos[i];
                self.pos[i] = cand;
                self.covered[i] = new_cov;
                self.bump_footprint(old_pos);
                self.bump_footprint(cand);
                changed = true;
            }
        }
        Self::mark_seen(&mut self.relocate_seen, snapshot);
        changed
    }

    /// Search the relocation neighborhood of live center `i` and return the
    /// best strictly-improving position, judged against the current snapshot.
    /// Read-only; used by the parallel propose phase of [`Self::relocate_pass`].
    fn propose_relocate(&self, i: usize) -> Option<PointArray> {
        if !self.live[i] {
            return None;
        }
        let excl = self.exclusive_of(i);
        if excl.is_empty() {
            return None; // drop_pass territory
        }
        let frame = LocalFrame::new(self.pos[i]);
        let excl_xy: Vec<[f64; 2]> = excl
            .iter()
            .map(|&r| frame.to_xy(self.reps[r as usize]))
            .collect();
        let sec = smallest_enclosing_circle(&excl_xy);

        // All local reps once; candidates get a cheap planar ranking and
        // only the leaders pay for exact verification.
        let local: Vec<(u32, [f64; 2])> = self
            .reps_within(self.pos[i], 3.0 * self.r_eff)
            .into_iter()
            .map(|r| (r, frame.to_xy(self.reps[r as usize])))
            .collect();
        let nearby_uncov: Vec<[f64; 2]> = local
            .iter()
            .filter(|(r, _)| self.count[*r as usize] == 0)
            .map(|(_, xy)| *xy)
            .take(12)
            .collect();

        let mut cand_xy: Vec<[f64; 2]> = Vec::new();
        if sec.radius <= self.r_eff {
            cand_xy.push(sec.center);
        }
        if !nearby_uncov.is_empty() {
            // Vertex pool: exclusive extremes (the binding constraints)
            // plus the nearby uncovered points.
            let mut pool: Vec<[f64; 2]> = excl_xy.clone();
            pool.sort_by(|a, b| {
                let da = (a[0] - sec.center[0]).powi(2) + (a[1] - sec.center[1]).powi(2);
                let db = (b[0] - sec.center[0]).powi(2) + (b[1] - sec.center[1]).powi(2);
                db.partial_cmp(&da).unwrap()
            });
            pool.truncate(8);
            pool.extend(nearby_uncov.iter().copied());
            for a in 0..pool.len() {
                for b in (a + 1)..pool.len() {
                    if let Some(vs) = circle_intersections(pool[a], pool[b], self.r_eff) {
                        cand_xy.push(vs[0]);
                        cand_xy.push(vs[1]);
                    }
                }
            }
        }

        // Loss-free moves only: every exclusive point must stay covered.
        // (Shedding exclusives for a bigger capture is an immediate −1
        // win but stretches exclusive sets toward extreme positions,
        // blocking −m merges downstream — measured net regression on
        // every m≥2 dataset.)
        let r2 = self.r_eff * self.r_eff;
        cand_xy.retain(|c| {
            excl_xy
                .iter()
                .all(|p| (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r2)
        });
        if cand_xy.is_empty() {
            return None;
        }

        // Planar ranking by (captured uncovered, total coverage).
        let mut ranked: Vec<(usize, usize, [f64; 2])> = cand_xy
            .into_iter()
            .map(|xy| {
                let mut captured = 0usize;
                let mut total = 0usize;
                for (r, p) in &local {
                    let d2 = (p[0] - xy[0]).powi(2) + (p[1] - xy[1]).powi(2);
                    if d2 <= r2 {
                        total += 1;
                        if self.count[*r as usize] == 0 {
                            captured += 1;
                        }
                    }
                }
                (captured, total, xy)
            })
            .collect();
        ranked.sort_unstable_by_key(|c| std::cmp::Reverse((c.0, c.1)));

        // Exact check (Haversine, conservative radius) on the top planar
        // candidates; first exact improvement wins.
        let old_total = self.covered[i].len();
        for &(_, _, xy) in ranked.iter().take(8) {
            let cand = frame.to_latlng(xy);
            if excl
                .iter()
                .any(|&r| haversine_m(cand, self.reps[r as usize]) > self.r_eff)
            {
                continue;
            }
            let new_cov = self.query_covered(cand);
            let captured = new_cov
                .iter()
                .filter(|&&r| self.count[r as usize] == 0)
                .count();
            if captured > 0 || new_cov.len() > old_total {
                return Some(cand);
            }
        }
        None
    }

    /// Merge pairs of centers when the set of points *only they* cover fits in
    /// one disk. Always Δ = −m. Pairs are gated at center distance ≤ 4r (the
    /// theoretical max for a feasible merge of two radius-r exclusive sets).
    fn merge_pass(&mut self, radius: Precision) -> bool {
        let mut changed = false;
        // Snapshot of live centers for neighbor discovery; staleness handled
        // by live[] re-checks. Multi-map because two centers can share an
        // S2 level-20 cell.
        let live_idx: Vec<usize> = (0..self.pos.len()).filter(|&i| self.live[i]).collect();
        let live_pos: SingleVec = live_idx.iter().map(|&i| self.pos[i]).collect();
        let center_tree = rtree::spawn(4.0 * radius, &live_pos);
        let mut cell_to_centers: HashMap<u64, Vec<usize>> = HashMap::new();
        for (k, &i) in live_idx.iter().enumerate() {
            let id = CellID::from(LatLng::from_degrees(live_pos[k][0], live_pos[k][1])).parent(20);
            cell_to_centers.entry(id.0).or_default().push(i);
        }

        // Dirty-region gating: only centers in changed regions look for
        // partners (a viable partner's mutation bumps this center's region).
        let dirty_idx: Vec<usize> = live_idx
            .iter()
            .copied()
            .filter(|&i| self.is_dirty(&self.merge_seen, self.pos[i]))
            .collect();
        let snapshot: Vec<(u64, u64)> = {
            let cells: HashMap<u64, u64> = dirty_idx
                .iter()
                .map(|&i| {
                    let cell = self.region_of(self.pos[i]);
                    let ep = self.region_epoch.get(&cell).copied().unwrap_or(1);
                    (cell, ep)
                })
                .collect();
            cells.into_iter().collect()
        };

        // Parallel propose: each dirty center finds its first feasible merge
        // partner on the snapshot. Serial apply revalidates liveness and the
        // (possibly grown) required set before committing.
        let this = &*self;
        let proposals: Vec<(usize, usize, PointArray)> = dirty_idx
            .par_iter()
            .filter_map(|&i| {
                let neighbors: Vec<usize> = center_tree
                    .locate_all_at_point(&this.pos[i])
                    .filter_map(|pt| cell_to_centers.get(&pt.cell_id.0))
                    .flatten()
                    .copied()
                    .filter(|&j| j > i && this.live[j])
                    .collect();
                for j in neighbors {
                    if let Some(cand) = this.propose_merge_pair(i, j) {
                        return Some((i, j, cand));
                    }
                }
                None
            })
            .collect();

        for (i, j, cand) in proposals {
            if !self.live[i] || !self.live[j] {
                continue;
            }
            // The required set may have grown since the proposal (a third
            // coverer died); the candidate must still cover all of it.
            let required = self.required_of_pair(i, j);
            if required.is_empty()
                || required
                    .iter()
                    .any(|&r| haversine_m(cand, self.reps[r as usize]) > self.r_eff)
            {
                continue;
            }
            let new_cov = self.query_covered(cand);
            self.kill_center(i);
            self.kill_center(j);
            self.add_center(cand, new_cov);
            changed = true;
        }
        Self::mark_seen(&mut self.merge_seen, snapshot);
        changed
    }

    /// SEC-feasibility check for merging pair (i, j) on the current snapshot;
    /// returns the merged center position when the pair's required points fit
    /// one disk. Read-only.
    fn propose_merge_pair(&self, i: usize, j: usize) -> Option<PointArray> {
        let required = self.required_of_pair(i, j);
        if required.is_empty() {
            return None;
        }
        let mid = [
            (self.pos[i][0] + self.pos[j][0]) / 2.0,
            (self.pos[i][1] + self.pos[j][1]) / 2.0,
        ];
        let frame = LocalFrame::new(mid);
        let xy: Vec<[f64; 2]> = required
            .iter()
            .map(|&r| frame.to_xy(self.reps[r as usize]))
            .collect();
        let sec = smallest_enclosing_circle(&xy);
        if sec.radius > self.r_eff {
            return None;
        }
        let cand = frame.to_latlng(sec.center);
        if required
            .iter()
            .any(|&r| haversine_m(cand, self.reps[r as usize]) > self.r_eff)
        {
            return None;
        }
        Some(cand)
    }

    /// Group repack: each dirty center + its ≤3 nearest neighbors gets its
    /// required points re-solved exactly (optimal disks + abandonment).
    /// Subsumes 3→2 swaps and adds 4→3 / 4→2 / mixed-abandonment moves.
    /// Groups are bounded, so the pass stays near-linear in centers.
    fn swap_group_pass(&mut self) -> bool {
        let live_idx: Vec<usize> = (0..self.pos.len()).filter(|&i| self.live[i]).collect();
        let live_pos: SingleVec = live_idx.iter().map(|&i| self.pos[i]).collect();
        let center_tree = rtree::spawn(4.0 * self.r_eff, &live_pos);
        let mut cell_to_centers: HashMap<u64, Vec<usize>> = HashMap::new();
        for (k, &i) in live_idx.iter().enumerate() {
            let id = CellID::from(LatLng::from_degrees(live_pos[k][0], live_pos[k][1])).parent(20);
            cell_to_centers.entry(id.0).or_default().push(i);
        }

        // Dirty-region gating, as in merge_pass.
        let dirty_idx: Vec<usize> = live_idx
            .iter()
            .copied()
            .filter(|&i| self.is_dirty(&self.swap_seen, self.pos[i]))
            .collect();
        let snapshot: Vec<(u64, u64)> = {
            let cells: HashMap<u64, u64> = dirty_idx
                .iter()
                .map(|&i| {
                    let cell = self.region_of(self.pos[i]);
                    let ep = self.region_epoch.get(&cell).copied().unwrap_or(1);
                    (cell, ep)
                })
                .collect();
            cells.into_iter().collect()
        };

        // Parallel propose: each dirty center forms a group with its ≤3
        // nearest neighbors and re-solves the group's required points with
        // the exact window solver (optimal disks + abandonment) — this
        // subsumes the former 3→2 tier and adds 4→3 / 4→2 / partial
        // abandonment in one move. Serial apply revalidates everything.
        let this = &*self;
        let radius = self.r_eff / MARGIN;
        let proposals: Vec<(Vec<usize>, Vec<PointArray>)> = dirty_idx
            .par_iter()
            .filter_map(|&i| {
                // NOTE: the cell multi-map can yield the same center index
                // more than once (two centers sharing an S2 L20 cell each map
                // the other's tree point back to the full cell bucket).
                // Dedupe before forming groups — a duplicated index would
                // double-kill a center and corrupt the coverage counts.
                let mut neigh: Vec<(Precision, usize)> = center_tree
                    .locate_all_at_point(&this.pos[i])
                    .filter_map(|pt| cell_to_centers.get(&pt.cell_id.0))
                    .flatten()
                    .copied()
                    .filter(|&j| j != i && this.live[j])
                    .map(|j| (haversine_m(this.pos[i], this.pos[j]), j))
                    .collect();
                neigh.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
                neigh.dedup_by_key(|&mut (_, j)| j);
                neigh.truncate(3);
                if neigh.is_empty() {
                    return None;
                }
                let mut group = vec![i];
                group.extend(neigh.iter().map(|&(_, j)| j));

                let required = this.required_of_group(&group);
                if required.is_empty() || required.len() > super::exact::MAX_EXACT_POINTS {
                    return None;
                }
                let mut centroid = [0.0, 0.0];
                for &g in &group {
                    centroid[0] += this.pos[g][0];
                    centroid[1] += this.pos[g][1];
                }
                centroid[0] /= group.len() as f64;
                centroid[1] /= group.len() as f64;
                let frame = LocalFrame::new(centroid);
                let lost_xy: Vec<[f64; 2]> = required
                    .iter()
                    .map(|&r| {
                        let xy = frame.to_xy(this.reps[r as usize]);
                        [xy[0] / radius, xy[1] / radius]
                    })
                    .collect();
                // Current group cost: every required point is covered now.
                let incumbent = this.m * group.len();
                let (_, centers_xy) =
                    super::exact::solve_window_exact(&lost_xy, this.m, incumbent)?;
                let centers: Vec<PointArray> = centers_xy
                    .into_iter()
                    .map(|xy| frame.to_latlng([xy[0] * radius, xy[1] * radius]))
                    .collect();
                Some((group, centers))
            })
            .collect();

        let mut changed = false;
        for (group, centers) in proposals {
            if group.iter().any(|&c| !self.live[c]) {
                continue;
            }
            // Exact-Haversine cost revalidation against the CURRENT required
            // set (it may have grown); abandonment is priced honestly.
            let required = self.required_of_group(&group);
            let new_uncov = required
                .iter()
                .filter(|&&r| {
                    let rp = self.reps[r as usize];
                    !centers.iter().any(|c| haversine_m(*c, rp) <= self.r_eff)
                })
                .count();
            if self.m * centers.len() + new_uncov >= self.m * group.len() {
                continue;
            }
            for &c in &group {
                self.kill_center(c);
            }
            for cand in &centers {
                let cov = self.query_covered(*cand);
                self.add_center(*cand, cov);
            }
            changed = true;
        }
        Self::mark_seen(&mut self.swap_seen, snapshot);
        changed
    }

    /// Cheap 3→2 heuristic tier (anchor disk-1 at a required vertex, SEC the
    /// remainder). The exact group repack above caps its required set at 64
    /// points, which dense m=1 quads routinely exceed — this tier keeps those
    /// moves alive. Runs at stalls before the exact tier.
    fn swap32_pass(&mut self) -> bool {
        let live_idx: Vec<usize> = (0..self.pos.len()).filter(|&i| self.live[i]).collect();
        let live_pos: SingleVec = live_idx.iter().map(|&i| self.pos[i]).collect();
        let center_tree = rtree::spawn(4.0 * self.r_eff, &live_pos);
        let mut cell_to_centers: HashMap<u64, Vec<usize>> = HashMap::new();
        for (k, &i) in live_idx.iter().enumerate() {
            let id = CellID::from(LatLng::from_degrees(live_pos[k][0], live_pos[k][1])).parent(20);
            cell_to_centers.entry(id.0).or_default().push(i);
        }
        let dirty_idx: Vec<usize> = live_idx
            .iter()
            .copied()
            .filter(|&i| self.is_dirty(&self.swap_seen, self.pos[i]))
            .collect();

        let this = &*self;
        let proposals: Vec<(usize, usize, usize, PointArray, PointArray)> = dirty_idx
            .par_iter()
            .filter_map(|&i| {
                let mut neigh: Vec<(Precision, usize)> = center_tree
                    .locate_all_at_point(&this.pos[i])
                    .filter_map(|pt| cell_to_centers.get(&pt.cell_id.0))
                    .flatten()
                    .copied()
                    .filter(|&j| j != i && this.live[j])
                    .map(|j| (haversine_m(this.pos[i], this.pos[j]), j))
                    .collect();
                neigh.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
                neigh.dedup_by_key(|&mut (_, j)| j);
                neigh.truncate(4);
                for a in 0..neigh.len() {
                    for b in (a + 1)..neigh.len() {
                        let (j, k) = (neigh[a].1, neigh[b].1);
                        if let Some((p1, p2)) = this.propose_swap32(i, j, k) {
                            return Some((i, j, k, p1, p2));
                        }
                    }
                }
                None
            })
            .collect();

        let mut changed = false;
        for (i, j, k, p1, p2) in proposals {
            if !self.live[i] || !self.live[j] || !self.live[k] {
                continue;
            }
            let required = self.required_of_group(&[i, j, k]);
            if required.is_empty() {
                continue;
            }
            let ok = required.iter().all(|&r| {
                let rp = self.reps[r as usize];
                haversine_m(p1, rp) <= self.r_eff || haversine_m(p2, rp) <= self.r_eff
            });
            if !ok {
                continue;
            }
            let cov1 = self.query_covered(p1);
            let cov2 = self.query_covered(p2);
            self.kill_center(i);
            self.kill_center(j);
            self.kill_center(k);
            self.add_center(p1, cov1);
            self.add_center(p2, cov2);
            changed = true;
        }
        changed
    }

    /// Find a 3→2 swap for trio (i, j, k) on the current snapshot; returns
    /// the two replacement disk centers. Read-only.
    fn propose_swap32(&self, i: usize, j: usize, k: usize) -> Option<(PointArray, PointArray)> {
        if i == j || j == k || i == k {
            return None;
        }
        let required = self.required_of_group(&[i, j, k]);
        if required.is_empty() || required.len() > 64 {
            return None;
        }

        let mid = [
            (self.pos[i][0] + self.pos[j][0] + self.pos[k][0]) / 3.0,
            (self.pos[i][1] + self.pos[j][1] + self.pos[k][1]) / 3.0,
        ];
        let frame = LocalFrame::new(mid);
        let req_xy: Vec<[f64; 2]> = required
            .iter()
            .map(|&r| frame.to_xy(self.reps[r as usize]))
            .collect();

        // Disk-1 anchors: required points + their pair vertices.
        let mut anchors: Vec<[f64; 2]> = req_xy.clone();
        for a in 0..req_xy.len().min(16) {
            for b in (a + 1)..req_xy.len().min(16) {
                if let Some(vs) = circle_intersections(req_xy[a], req_xy[b], self.r_eff) {
                    anchors.push(vs[0]);
                    anchors.push(vs[1]);
                }
            }
        }
        let r2 = self.r_eff * self.r_eff;
        for c1 in anchors {
            let mut rest: Vec<[f64; 2]> = Vec::new();
            for &xy in &req_xy {
                let d2 = (xy[0] - c1[0]).powi(2) + (xy[1] - c1[1]).powi(2);
                if d2 > r2 {
                    rest.push(xy);
                }
            }
            if rest.is_empty() {
                continue; // one disk suffices — merge_pass territory
            }
            let sec = smallest_enclosing_circle(&rest);
            if sec.radius > self.r_eff {
                continue;
            }
            let p1 = frame.to_latlng(c1);
            let p2 = frame.to_latlng(sec.center);
            let ok = required.iter().all(|&r| {
                let rp = self.reps[r as usize];
                haversine_m(p1, rp) <= self.r_eff || haversine_m(p2, rp) <= self.r_eff
            });
            if !ok {
                continue;
            }
            return Some((p1, p2));
        }
        None
    }

    /// Points whose only live coverers are within `group`.
    fn required_of_group(&self, group: &[usize]) -> Vec<u32> {
        let mut required: Vec<u32> = Vec::new();
        for &c in group {
            for &p in &self.covered[c] {
                let cnt = self.count[p as usize] as usize;
                if cnt
                    == group
                        .iter()
                        .filter(|&&o| self.covered[o].binary_search(&p).is_ok())
                        .count()
                {
                    required.push(p);
                }
            }
        }
        required.sort_unstable();
        required.dedup();
        required
    }

    /// Points whose only live coverers are i and/or j.
    fn required_of_pair(&self, i: usize, j: usize) -> Vec<u32> {
        let cj = &self.covered[j];
        let mut req: Vec<u32> = Vec::new();
        for &p in &self.covered[i] {
            match self.count[p as usize] {
                1 => req.push(p),
                2 if cj.binary_search(&p).is_ok() => req.push(p),
                _ => {}
            }
        }
        for &p in cj {
            if self.count[p as usize] == 1 {
                req.push(p);
            }
        }
        req
    }

    /// Reps within `dist_m` of `p` (Haversine), sorted.
    fn reps_within(&self, p: PointArray, dist_m: Precision) -> Vec<u32> {
        let mut out: Vec<u32> = Vec::new();
        self.grid
            .for_each_within(self.reps, p, dist_m, |i| out.push(i));
        out.sort_unstable();
        out
    }

    /// Greedy cover of the still-uncovered points: candidates are the
    /// uncovered reps plus pair intersections of nearby uncovered pairs;
    /// commit while best gain ≥ m (gain counts only uncovered reps).
    fn gapfill_pass(&mut self) -> bool {
        let uncovered: Vec<u32> = (0..self.reps.len() as u32)
            .filter(|&r| self.count[r as usize] == 0)
            .collect();
        if uncovered.is_empty() {
            return false;
        }

        // Candidates.
        let mut cands: Vec<PointArray> = uncovered.iter().map(|&r| self.reps[r as usize]).collect();
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for &u in &uncovered {
            let p = self.reps[u as usize];
            let mut partners: Vec<(Precision, u32)> = self
                .reps_within(p, 2.0 * self.r_eff)
                .into_iter()
                .filter(|&v| v != u && self.count[v as usize] == 0)
                .map(|v| (haversine_m(p, self.reps[v as usize]), v))
                .collect();
            partners.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));
            for &(_, v) in partners.iter().take(8) {
                pairs.push((u.min(v), u.max(v)));
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        for &(u, v) in &pairs {
            let a = self.reps[u as usize];
            let frame = LocalFrame::new(a);
            let xy_a = frame.to_xy(a);
            let xy_b = frame.to_xy(self.reps[v as usize]);
            if let Some(inters) = circle_intersections(xy_a, xy_b, self.r_eff) {
                cands.push(frame.to_latlng(inters[0]));
                cands.push(frame.to_latlng(inters[1]));
            }
        }

        // Lazy greedy bucket queue over uncovered-gain.
        fn gain_of(refiner: &Refiner, c: PointArray) -> usize {
            refiner
                .query_covered(c)
                .into_iter()
                .filter(|&r| refiner.count[r as usize] == 0)
                .count()
        }
        let gains: Vec<usize> = {
            let this: &Refiner = &*self;
            cands.par_iter().map(|c| gain_of(this, *c)).collect()
        };
        let max_gain = gains.iter().copied().max().unwrap_or(0);
        if max_gain < self.m {
            return false;
        }
        let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); max_gain + 1];
        for (ci, &g) in gains.iter().enumerate() {
            if g >= self.m {
                buckets[g].push(ci as u32);
            }
        }
        let mut changed = false;
        let mut cur = max_gain;
        loop {
            while cur >= self.m && buckets[cur].is_empty() {
                if cur == self.m {
                    return changed;
                }
                cur -= 1;
            }
            if cur < self.m {
                return changed;
            }
            let ci = buckets[cur].pop().unwrap() as usize;
            let g = gain_of(self, cands[ci]);
            if g >= cur {
                let cov = self.query_covered(cands[ci]);
                self.add_center(cands[ci], cov);
                changed = true;
            } else if g >= self.m {
                buckets[g].push(ci as u32);
            }
        }
    }

    /// LNS ruin-and-recreate around uncovered points. Windows are S2 cells
    /// (~6r edge) containing uncovered reps, densest first: remove the live
    /// centers inside, re-solve the local subproblem with the chunk solver
    /// (points covered by outside centers are fixed as pre-covered), and
    /// accept iff the local cost m·centers + uncovered strictly drops. This
    /// is the move that restructures whole neighborhoods where the greedy
    /// stranded sub-m point groups — single-center moves can't reach those.
    fn lns_pass(&mut self, radius: Precision, sweep_all: bool) -> bool {
        const MAX_WINDOWS: usize = 4096;

        // Window level: deepest S2 level whose average edge is ≥ ~6r.
        let mut level: u64 = 0;
        while level < 18 && 8_000_000.0 / (1u64 << (level + 1)) as Precision >= 6.0 * radius {
            level += 1;
        }

        // Live centers grouped by window cell (kept fresh on accepts).
        let mut center_cells: HashMap<u64, Vec<usize>> = HashMap::new();
        for i in 0..self.pos.len() {
            if self.live[i] {
                let cell = CellID::from(LatLng::from_degrees(self.pos[i][0], self.pos[i][1]))
                    .parent(level);
                center_cells.entry(cell.0).or_default().push(i);
            }
        }

        // Windows: cells with uncovered reps, densest first. Small inputs
        // (sweep_all) repack every center-occupied cell instead — the m≥2
        // packing structure can be suboptimal even where nothing is
        // uncovered, and small inputs can afford the full sweep.
        let mut win_count: HashMap<u64, u32> = HashMap::new();
        for (r, &c) in self.count.iter().enumerate() {
            if c == 0 {
                let p = self.reps[r];
                let cell = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(level);
                *win_count.entry(cell.0).or_insert(0) += 1;
            }
        }
        if sweep_all {
            for cell in center_cells.keys() {
                win_count.entry(*cell).or_insert(0);
            }
        }
        if win_count.is_empty() {
            return false;
        }
        let mut windows: Vec<(u32, u64)> = win_count.into_iter().map(|(c, n)| (n, c)).collect();
        windows.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        windows.truncate(MAX_WINDOWS);

        // Parallel propose: solve every window against the snapshot (gather,
        // pre-covered mask, 5 solver variants — all the expensive work).
        // Windows whose lost set has a 2·RHO-connected component beyond the
        // exact solver's cap descend into their four S2 children (split until
        // exact): disks never span disconnected lost components, so per-
        // component exact solves compose into the window optimum.
        let max_descent = level + 2;
        let this = &*self;
        let proposals: Vec<LnsProposal> = windows
            .par_iter()
            .flat_map_iter(|&(_, cell_raw)| {
                let base_removed: Vec<usize> = center_cells
                    .get(&cell_raw)
                    .map(|v| v.iter().copied().filter(|&i| this.live[i]).collect())
                    .unwrap_or_default();
                let mut out: Vec<LnsProposal> = Vec::new();
                if base_removed.is_empty() {
                    return out.into_iter();
                }
                let mut stack: Vec<(u64, Vec<usize>)> = vec![(cell_raw, base_removed)];
                while let Some((cw, removed)) = stack.pop() {
                    let cell = CellID(cw);
                    let allow_split = cell.level() < max_descent;
                    match this.lns_window(cw, &removed, radius, allow_split) {
                        LnsOutcome::Skip => {}
                        LnsOutcome::Proposal(p) => out.push(p),
                        LnsOutcome::Split => {
                            for child in cell.children() {
                                let child_removed: Vec<usize> = removed
                                    .iter()
                                    .copied()
                                    .filter(|&i| {
                                        crate::clustering::partition::contains_latlng(
                                            child,
                                            this.pos[i],
                                        )
                                    })
                                    .collect();
                                if !child_removed.is_empty() {
                                    stack.push((child.0, child_removed));
                                }
                            }
                        }
                    }
                }
                out.into_iter()
            })
            .collect();

        // Serial commit with exact revalidation against the current state
        // (overlapping windows may have shifted counts since the proposal).
        let mut changed = false;
        for prop in proposals {
            if prop.removed.iter().any(|&c| !self.live[c]) {
                continue;
            }
            let mut removed_cover: HashMap<u32, u32> = HashMap::new();
            for &c in &prop.removed {
                for &p in &self.covered[c] {
                    *removed_cover.entry(p).or_insert(0) += 1;
                }
            }
            let mut old_uncov = 0usize;
            let mut new_uncov = 0usize;
            for (idx, &p) in prop.local.iter().enumerate() {
                let cnt = self.count[p as usize];
                if cnt == 0 {
                    old_uncov += 1;
                }
                let rc = removed_cover.get(&p).copied().unwrap_or(0);
                if cnt <= rc && !prop.prop_mask[idx] {
                    new_uncov += 1;
                }
            }
            let old_cost = self.m * prop.removed.len() + old_uncov;
            let new_cost = self.m * prop.centers.len() + new_uncov;
            if new_cost >= old_cost + prop.slack {
                continue;
            }
            for &c in &prop.removed {
                self.kill_center(c);
            }
            for cand in prop.centers {
                let cov = self.query_covered(cand);
                self.add_center(cand, cov);
            }
            changed = true;
        }
        changed
    }

    /// Evaluate one LNS window (build + solve, read-only). Returns Split when
    /// the lost set has a component beyond the exact cap (the caller descends
    /// into child cells), a Proposal on strict improvement, or Skip.
    fn lns_window(
        &self,
        cell_raw: u64,
        removed: &[usize],
        radius: Precision,
        allow_split: bool,
    ) -> LnsOutcome {
        use super::frame::Grid;
        use super::solve::{RHO, SolveParams, solve_chunk};
        use crate::clustering::partition::cell_bbox_lat_lon;
        use crate::s2::ToPointArray;

        const MAX_WINDOW_POINTS: usize = 4000;

        // Local point set: reps in the expanded window bbox plus everything
        // the removed centers cover (so no coverage loss can hide outside).
        let bbox = cell_bbox_lat_lon(CellID(cell_raw));
        let margin = crate::clustering::candidates::meters_to_degrees(radius, bbox.center_lat());
        let expanded = bbox.expand(margin);
        let mut local: Vec<u32> = Vec::new();
        self.grid.for_each_in_bbox(
            self.reps,
            expanded.min_lat,
            expanded.max_lat,
            expanded.min_lon,
            expanded.max_lon,
            |i| local.push(i),
        );
        for &c in removed {
            local.extend(self.covered[c].iter().copied());
        }
        local.sort_unstable();
        local.dedup();
        if local.len() > MAX_WINDOW_POINTS {
            // At the descent floor an oversized window is skipped (the old
            // behavior); above it, descend.
            return if allow_split {
                LnsOutcome::Split
            } else {
                LnsOutcome::Skip
            };
        }

        // Coverage by removed centers, per local point.
        let mut removed_cover: HashMap<u32, u32> = HashMap::new();
        for &c in removed {
            for &p in &self.covered[c] {
                *removed_cover.entry(p).or_insert(0) += 1;
            }
        }
        let pre: Vec<bool> = local
            .iter()
            .map(|&p| {
                let rc = removed_cover.get(&p).copied().unwrap_or(0);
                self.count[p as usize] > rc
            })
            .collect();
        let old_uncov = local
            .iter()
            .filter(|&&p| self.count[p as usize] == 0)
            .count();
        let old_cost = self.m * removed.len() + old_uncov;

        let frame = LocalFrame::new(CellID(cell_raw).point_array());
        let pts_planar: Vec<[f64; 2]> = local
            .iter()
            .map(|&p| {
                let xy = frame.to_xy(self.reps[p as usize]);
                [xy[0] / radius, xy[1] / radius]
            })
            .collect();
        let lost: Vec<[f64; 2]> = pts_planar
            .iter()
            .zip(&pre)
            .filter(|(_, pre)| !**pre)
            .map(|(p, _)| *p)
            .collect();

        // Split-until-exact: if any 2·RHO component of the lost set exceeds
        // the exact cap, descend instead of settling for greedy here.
        let comps = if lost.len() > 512 {
            Vec::new() // certainly splitting; skip the O(n²) components
        } else {
            planar_components(&lost, 2.0 * RHO)
        };
        let too_big = lost.len() > 512
            || comps
                .iter()
                .any(|c| c.len() > super::exact::MAX_EXACT_POINTS);
        if too_big && allow_split {
            return LnsOutcome::Split;
        }
        // At the floor with an oversized lost set: greedy-only (no exact).

        // Local planar solve, best of the deterministic variants.
        let mut best: Option<(usize, Vec<[f64; 2]>, Vec<bool>)> = None;
        for v in 0..5u8 {
            let centers = solve_chunk(
                &pts_planar,
                &SolveParams {
                    m: self.m,
                    k_cap: 12,
                    variant: v,
                    pre_covered: &pre,
                },
            );
            let grid = Grid::build(&centers, 1.0);
            let mask: Vec<bool> = pts_planar
                .iter()
                .map(|p| {
                    let mut hit = false;
                    grid.for_each_within(&centers, *p, RHO, |_, _| hit = true);
                    hit
                })
                .collect();
            let uncov = mask
                .iter()
                .zip(&pre)
                .filter(|(hit, pre)| !**pre && !**hit)
                .count();
            let cost = self.m * centers.len() + uncov;
            if best.as_ref().is_none_or(|(c, _, _)| cost < *c) {
                best = Some((cost, centers, mask));
            }
        }
        let (mut new_cost, mut new_centers, mut prop_mask) = best.expect("variants ran");

        // Exact per-component re-solve (components are independent: no disk
        // spans two 2·RHO components). Falls back to greedy on node budget.
        if !lost.is_empty() && !too_big {
            let mut exact_total = 0usize;
            let mut exact_centers: Vec<[f64; 2]> = Vec::new();
            let mut complete = true;
            for comp in &comps {
                let comp_pts: Vec<[f64; 2]> = comp.iter().map(|&i| lost[i]).collect();
                match super::exact::solve_window_exact(&comp_pts, self.m, usize::MAX) {
                    Some((cost, centers)) => {
                        exact_total += cost;
                        exact_centers.extend(centers);
                    }
                    None => {
                        complete = false;
                        break;
                    }
                }
            }
            if complete && exact_total < new_cost {
                let r2 = RHO * RHO;
                prop_mask = pts_planar
                    .iter()
                    .map(|p| {
                        exact_centers
                            .iter()
                            .any(|c| (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r2)
                    })
                    .collect();
                new_cost = exact_total;
                new_centers = exact_centers;
            }
        }

        // Annealing (flag-gated): allow a bounded, deterministic
        // pseudo-random regression so windows can escape local optima; the
        // run-level best-state snapshot makes it safe.
        let slack = self.anneal_slack(cell_raw);
        if new_cost >= old_cost + slack {
            return LnsOutcome::Skip;
        }
        let centers: Vec<PointArray> = new_centers
            .into_iter()
            .map(|xy| frame.to_latlng([xy[0] * radius, xy[1] * radius]))
            .collect();
        LnsOutcome::Proposal(LnsProposal {
            removed: removed.to_vec(),
            local,
            prop_mask,
            centers,
            slack,
        })
    }

    /// Deterministic per-window annealing slack: 0 when not annealing, else
    /// floor(temp · hash01(cell, salt)). No RNG — same input, same schedule.
    fn anneal_slack(&self, cell_raw: u64) -> usize {
        match self.anneal_state {
            None => 0,
            Some((temp, salt)) => {
                let mut h = cell_raw ^ salt;
                h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                h ^= h >> 32;
                h = h.wrapping_mul(0xD6E8_FEB8_6659_FD93);
                let unit = (h >> 11) as f64 / (1u64 << 53) as f64;
                (temp * unit).floor() as usize
            }
        }
    }

    /// Current true cost (m·live centers + uncovered reps).
    fn total_cost(&self) -> usize {
        let live = self.live.iter().filter(|l| **l).count();
        let uncov = self.count.iter().filter(|c| **c == 0).count();
        self.m * live + uncov
    }

    /// Flag-gated (KOJI_AUTO_ANNEAL=1) annealing phase: a few LNS rounds that
    /// may accept bounded regressions (escaping local optima), each followed
    /// by a cheap-pass cleanup; the best state seen is kept. Deterministic.
    fn anneal(&mut self, radius: Precision, sweep_all: bool) {
        let snapshot = |s: &Self| {
            (
                s.pos.clone(),
                s.covered.clone(),
                s.live.clone(),
                s.count.clone(),
            )
        };
        let mut best_cost = self.total_cost();
        let mut best = snapshot(self);
        for (round, temp_scale) in [2.0, 1.5, 1.0, 0.5].into_iter().enumerate() {
            self.anneal_state = Some((temp_scale * self.m as Precision, 0x5EED + round as u64));
            let moved = self.lns_pass(radius, sweep_all);
            self.anneal_state = None;
            if !moved {
                continue;
            }
            // Consolidate, then run a strict LNS to claw back the slack.
            for _ in 0..2 {
                let mut changed = false;
                changed |= self.drop_pass();
                changed |= self.relocate_pass();
                changed |= self.merge_pass(radius);
                changed |= self.gapfill_pass();
                if !changed {
                    break;
                }
            }
            self.lns_pass(radius, sweep_all);
            let cost = self.total_cost();
            if cost < best_cost {
                best_cost = cost;
                best = snapshot(self);
            }
        }
        if self.total_cost() > best_cost {
            let (pos, covered, live, count) = best;
            self.pos = pos;
            self.covered = covered;
            self.live = live;
            self.count = count;
            // All scheduling state is stale after a restore.
            self.relocate_seen.clear();
            self.merge_seen.clear();
            self.swap_seen.clear();
        }
    }

    /// Debugging aid (env KOJI_AUTO_PARANOID=1): recompute `count` from the
    /// live covered lists and compare against the incremental bookkeeping;
    /// report uncovered totals. Identifies the pass that corrupts state.
    fn paranoid_check(&self, label: &str) {
        let mut recount = vec![0u32; self.reps.len()];
        for i in 0..self.pos.len() {
            if self.live[i] {
                for &r in &self.covered[i] {
                    recount[r as usize] += 1;
                }
            }
        }
        let drift = recount
            .iter()
            .zip(&self.count)
            .filter(|(a, b)| a != b)
            .count();
        let uncovered = recount.iter().filter(|&&c| c == 0).count();
        log::warn!("paranoid[{label}]: drift={drift} uncovered={uncovered}");
    }

    /// Run all passes for up to `max_rounds` rounds (early exit on a clean
    /// round) and return the surviving centers, sorted by S2 cell id.
    pub fn run(mut self, radius: Precision, max_rounds: usize) -> SingleVec {
        let paranoid = std::env::var("KOJI_AUTO_PARANOID").is_ok();
        let mut exhausted = true;
        for round in 0..max_rounds {
            let mut changed = false;
            changed |= self.drop_pass();
            if paranoid {
                self.paranoid_check(&format!("r{round}-drop"));
            }
            changed |= self.relocate_pass();
            if paranoid {
                self.paranoid_check(&format!("r{round}-relocate"));
            }
            changed |= self.merge_pass(radius);
            if paranoid {
                self.paranoid_check(&format!("r{round}-merge"));
            }
            changed |= self.gapfill_pass();
            if paranoid {
                self.paranoid_check(&format!("r{round}-gapfill"));
            }
            if !changed {
                // Cheap passes converged; pay for the expensive passes only
                // at stalls, cheapest tier first. A successful tier re-opens
                // the cheap passes (new disks usually enable drops/merges).
                let mut reopened = self.swap32_pass();
                if paranoid {
                    self.paranoid_check(&format!("r{round}-swap32"));
                }
                if !reopened {
                    reopened = self.swap_group_pass();
                    if paranoid {
                        self.paranoid_check(&format!("r{round}-swapgroup"));
                    }
                }
                if !reopened {
                    reopened = self.lns_pass(radius, self.reps.len() <= LNS_SWEEP_ALL_MAX);
                    if paranoid {
                        self.paranoid_check(&format!("r{round}-lns"));
                    }
                }
                if !reopened {
                    exhausted = false;
                    break;
                }
            }
        }
        // Round budget drained while the cheap passes were still churning:
        // the expensive passes never got their stall slot. Give them one
        // shot, then a cleanup sweep so their new disks get consolidated.
        if exhausted {
            let swapped = self.swap32_pass() | self.swap_group_pass();
            let filled = self.lns_pass(radius, self.reps.len() <= LNS_SWEEP_ALL_MAX);
            if paranoid {
                self.paranoid_check("post-exhaustion-expensive");
            }
            if swapped || filled {
                self.drop_pass();
                self.relocate_pass();
                self.merge_pass(radius);
                self.gapfill_pass();
                if paranoid {
                    self.paranoid_check("post-exhaustion-cleanup");
                }
            }
        }
        // Opt-in annealing phase: bounded deterministic regressions to
        // escape local optima, best state kept.
        if std::env::var("KOJI_AUTO_ANNEAL").is_ok_and(|v| v == "1") {
            self.anneal(radius, self.reps.len() <= LNS_SWEEP_ALL_MAX);
            if paranoid {
                self.paranoid_check("post-anneal");
            }
        }
        let mut out: Vec<(u64, PointArray)> = (0..self.pos.len())
            .filter(|&i| self.live[i])
            .map(|i| {
                let id =
                    CellID::from(LatLng::from_degrees(self.pos[i][0], self.pos[i][1])).parent(20);
                (id.0, self.pos[i])
            })
            .collect();
        out.sort_by_key(|(id, _)| *id);
        out.into_iter().map(|(_, p)| p).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ~meters offsets around a base coordinate for test construction.
    fn at(base: PointArray, dx_m: f64, dy_m: f64) -> PointArray {
        let frame = LocalFrame::new(base);
        frame.to_latlng([dx_m, dy_m])
    }

    #[test]
    fn drop_removes_redundant_center() {
        let base = [40.0, -74.0];
        let reps: SingleVec = vec![at(base, 0.0, 0.0), at(base, 30.0, 0.0)];
        // Two centers covering the same two points: one is redundant at m=1.
        let centers = vec![at(base, 10.0, 0.0), at(base, 20.0, 0.0)];
        let refiner = Refiner::new(centers, &reps, 70.0, 1);
        let out = refiner.run(70.0, 3);
        assert_eq!(out.len(), 1, "redundant center must be dropped");
    }

    #[test]
    fn merge_combines_mergeable_pair() {
        let base = [40.0, -74.0];
        // Two points 100 m apart, two centers each on top of one point.
        // One disk at the midpoint covers both (100 < 2·69.93).
        let reps: SingleVec = vec![at(base, 0.0, 0.0), at(base, 100.0, 0.0)];
        let centers = vec![at(base, 0.0, 0.0), at(base, 100.0, 0.0)];
        let refiner = Refiner::new(centers, &reps, 70.0, 1);
        let out = refiner.run(70.0, 3);
        assert_eq!(out.len(), 1, "pair must merge into one disk");
        for r in &reps {
            assert!(haversine_m(out[0], *r) <= 70.0 * MARGIN + 1e-6);
        }
    }

    #[test]
    fn gapfill_covers_uncovered_at_m1() {
        let base = [40.0, -74.0];
        let reps: SingleVec = vec![
            at(base, 0.0, 0.0),
            at(base, 500.0, 0.0),
            at(base, 1000.0, 0.0),
        ];
        // Start with no centers at all.
        let refiner = Refiner::new(vec![], &reps, 70.0, 1);
        let out = refiner.run(70.0, 3);
        assert_eq!(out.len(), 3, "every isolated point needs a center at m=1");
    }

    #[test]
    fn gapfill_respects_min_points() {
        let base = [40.0, -74.0];
        let reps: SingleVec = vec![at(base, 0.0, 0.0), at(base, 500.0, 0.0)];
        let refiner = Refiner::new(vec![], &reps, 70.0, 3);
        let out = refiner.run(70.0, 3);
        assert!(
            out.is_empty(),
            "isolated points must stay uncovered at m=3 (cheaper than clusters)"
        );
    }
}
