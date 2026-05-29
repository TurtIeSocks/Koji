//! S2 grid bucketing utilities shared across the workspace.
//!
//! `create_cell_map` buckets a set of `[lat, lon]` points by their S2 cell at a
//! requested `split_level`. It is used by the adaptive clustering partitioner
//! (`algorithms`) and by the external plugin runner (`koji-plugins`) to fan a
//! point set out over parallel S2 cells. It lives in `koji-core` so both of
//! those crates can depend on it without forming a cycle.

use std::collections::HashMap;

use s2::cellid::CellID;
use s2::latlng::LatLng;

use crate::{PointArray, SingleVec};

/// Map a `[lat, lon]` point to its S2 `CellID` at `parent_level`.
fn from_array_to_cell_id(point: &PointArray, parent_level: u64) -> CellID {
    CellID::from(LatLng::from_degrees(point[0], point[1])).parent(parent_level)
}

/// Bucket `points` by their S2 cell at `split_level`.
///
/// Each point is first resolved to its level-20 leaf cell, then grouped by that
/// leaf's ancestor at `split_level`. The returned map is keyed by the raw
/// `CellID` (`u64`) of the `split_level` ancestor; the value is the subset of
/// the original points that fall inside it. Preserves every input point exactly
/// once across the buckets.
pub fn create_cell_map(points: &SingleVec, split_level: u64) -> HashMap<u64, SingleVec> {
    let s20cells: Vec<CellID> = points
        .iter()
        .map(|point| from_array_to_cell_id(point, 20))
        .collect();
    let mut cell_maps = HashMap::new();
    for (i, cell) in s20cells.into_iter().enumerate() {
        let handler = cell_maps
            .entry(cell.parent(split_level).0)
            .or_insert(Vec::new());
        handler.push(points[i]);
    }
    cell_maps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_preserve_all_points() {
        let points: SingleVec = vec![
            [37.7749, -122.4194],
            [37.7750, -122.4195],
            [40.7128, -74.0060],
        ];
        let map = create_cell_map(&points, 6);
        let total: usize = map.values().map(|v| v.len()).sum();
        assert_eq!(total, points.len());
    }

    #[test]
    fn nearby_points_share_a_coarse_cell() {
        // Two points a few meters apart land in the same low-level (coarse) cell.
        let points: SingleVec = vec![[37.7749, -122.4194], [37.77491, -122.41941]];
        let map = create_cell_map(&points, 6);
        assert_eq!(map.len(), 1, "adjacent points should share one L6 cell");
    }

    #[test]
    fn distant_points_split_into_separate_cells() {
        let points: SingleVec = vec![[37.7749, -122.4194], [40.7128, -74.0060]];
        let map = create_cell_map(&points, 6);
        assert_eq!(map.len(), 2, "SF and NYC should not share an L6 cell");
    }

    #[test]
    fn empty_input_yields_empty_map() {
        let points: SingleVec = vec![];
        assert!(create_cell_map(&points, 10).is_empty());
    }
}
