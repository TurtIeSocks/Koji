//! Input dedup + exact connectivity decomposition.
//!
//! Two points can share a disk only if they are ≤ 2r apart, so the connected
//! components of the 2r-proximity graph can be solved independently with no
//! quality loss. Components are computed conservatively (over-merging is safe,
//! under-merging is not) by unioning S2 cells whose centers are within
//! `2·cell_diag + 2r` of each other.

use hashbrown::HashMap;
use s2::{cellid::CellID, latlng::LatLng};

use koji_core::{Precision, SingleVec};

use crate::rtree;
use crate::s2::ToPointArray;

/// Deduplicate points at S2 level 20 (the scorer's identity level). The first
/// instance in input order is the representative; covering a representative
/// within the effective radius implies the scorer counts its cell covered.
pub fn dedupe(points: &SingleVec) -> (SingleVec, Vec<CellID>) {
    let mut seen: HashMap<u64, ()> = HashMap::with_capacity(points.len());
    let mut reps: SingleVec = Vec::with_capacity(points.len());
    let mut cells: Vec<CellID> = Vec::with_capacity(points.len());
    for p in points {
        let cell = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
        if seen.insert(cell.0, ()).is_none() {
            reps.push(*p);
            cells.push(cell);
        }
    }
    (reps, cells)
}

struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n as u32).collect(),
        }
    }
    fn find(&mut self, i: u32) -> u32 {
        let mut root = i;
        while self.parent[root as usize] != root {
            root = self.parent[root as usize];
        }
        let mut cur = i;
        while self.parent[cur as usize] != root {
            let next = self.parent[cur as usize];
            self.parent[cur as usize] = root;
            cur = next;
        }
        root
    }
    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            // Deterministic: smaller root wins.
            let (lo, hi) = if ra < rb { (ra, rb) } else { (rb, ra) };
            self.parent[hi as usize] = lo;
        }
    }
}

/// Approximate average S2 cell edge length in meters at `level`.
fn avg_edge_meters(level: u64) -> Precision {
    8_000_000.0 / (1u64 << level) as Precision
}

/// Pick the grouping level: the deepest level whose average edge is still
/// ≥ 2·(2r), so points ≤ 2r apart always land in the same or adjacent cells
/// (with margin for min-vs-avg edge variation).
fn grouping_level(radius: Precision) -> u64 {
    let target = 4.0 * radius.max(1.0);
    let mut level: u64 = 0;
    while level < 18 && avg_edge_meters(level + 1) >= target {
        level += 1;
    }
    level
}

/// Decompose `reps` into 2r-connectivity components (conservatively merged).
/// Returns components as sorted index lists, ordered by their smallest cell id
/// — fully deterministic.
pub fn components(reps: &SingleVec, radius: Precision) -> Vec<Vec<u32>> {
    if reps.is_empty() {
        return vec![];
    }
    let level = grouping_level(radius);

    // Group representative indices by cell.
    let mut cell_members: HashMap<u64, Vec<u32>> = HashMap::new();
    for (i, p) in reps.iter().enumerate() {
        let cell = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(level);
        cell_members.entry(cell.0).or_default().push(i as u32);
    }

    // Deterministic cell ordering.
    let mut cell_ids: Vec<u64> = cell_members.keys().copied().collect();
    cell_ids.sort_unstable();

    // Union occupied cells whose centers are within 2·diag + 2r. The rtree
    // points carry that radius so locate_all_at_point is the exact predicate.
    let union_radius = 2.0 * avg_edge_meters(level) * 1.5 + 2.0 * radius;
    let cell_centers: SingleVec = cell_ids
        .iter()
        .map(|id| CellID(*id).point_array())
        .collect();
    let tree = rtree::spawn(union_radius, &cell_centers);
    let center_to_cell_idx: HashMap<u64, u32> = cell_centers
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let id20 = CellID::from(LatLng::from_degrees(c[0], c[1])).parent(20);
            (id20.0, i as u32)
        })
        .collect();

    let mut uf = UnionFind::new(cell_ids.len());
    for (i, center) in cell_centers.iter().enumerate() {
        for nb in tree.locate_all_at_point(center) {
            if let Some(&j) = center_to_cell_idx.get(&nb.cell_id.0) {
                uf.union(i as u32, j);
            }
        }
    }

    // Gather components: root cell idx → rep indices.
    let mut comp_map: HashMap<u32, Vec<u32>> = HashMap::new();
    for (i, id) in cell_ids.iter().enumerate() {
        let root = uf.find(i as u32);
        comp_map
            .entry(root)
            .or_default()
            .extend(cell_members.remove(id).unwrap());
    }
    let mut comps: Vec<Vec<u32>> = comp_map.into_values().collect();
    for c in comps.iter_mut() {
        c.sort_unstable();
    }
    comps.sort_by_key(|c| c[0]);
    comps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupe_collapses_same_cell() {
        let p = [40.0, -74.0];
        let q = [40.001, -74.0]; // ~110 m away: distinct cell
        let (reps, _) = dedupe(&vec![p, p, q, p]);
        assert_eq!(reps.len(), 2);
        assert_eq!(reps[0], p);
        assert_eq!(reps[1], q);
    }

    #[test]
    fn far_groups_are_separate_components() {
        // Two tight clumps ~20 km apart, r = 70 m.
        let mut pts: SingleVec = vec![];
        for i in 0..5 {
            pts.push([40.0 + i as f64 * 0.0003, -74.0]);
            pts.push([40.18 + i as f64 * 0.0003, -74.0]);
        }
        let (reps, _) = dedupe(&pts);
        let comps = components(&reps, 70.0);
        assert_eq!(comps.len(), 2, "expected 2 components, got {comps:?}");
        assert_eq!(comps.iter().map(|c| c.len()).sum::<usize>(), reps.len());
    }

    #[test]
    fn chain_within_2r_is_one_component() {
        // Points spaced ~130 m (< 2r = 140 m) form a chain → one component,
        // even across grouping-cell boundaries.
        let mut pts: SingleVec = vec![];
        for i in 0..60 {
            pts.push([40.0 + i as f64 * 0.00117, -74.0]);
        }
        let (reps, _) = dedupe(&pts);
        let comps = components(&reps, 70.0);
        assert_eq!(comps.len(), 1, "chain must stay one component");
    }
}
