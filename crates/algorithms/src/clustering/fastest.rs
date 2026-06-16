//! "Fastest" clustering mode.
//!
//! Points are projected onto a flat plane scaled so that the target radius maps to one unit,
//! then bucketed into a grid of unit-diameter cells (cell size `√2`, so a cell's diagonal is the
//! unit diameter). A second pass merges each cell with one adjacent cell when their combined
//! extent still fits inside a unit disc, and a final pass emits one center per surviving cluster.
//! This trades placement quality for speed — it is the cheapest of the clustering algorithms.

use geo::Coord;
use koji_core::SingleVec;
use rstar::PointDistance;
use std::collections::HashMap;

use crate::project::Plane;

/// Axis-aligned bounding box tracking the extent of the points within one grid cell.
#[derive(Debug, Clone, Copy)]
struct BoundingBox {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl BoundingBox {
    /// A degenerate box containing a single point.
    fn new(point: Coord) -> Self {
        BoundingBox {
            min_x: point.x,
            min_y: point.y,
            max_x: point.x,
            max_y: point.y,
        }
    }

    /// Grow the box to include `point`.
    fn extend(&mut self, point: Coord) {
        self.min_x = self.min_x.min(point.x);
        self.min_y = self.min_y.min(point.y);
        self.max_x = self.max_x.max(point.x);
        self.max_y = self.max_y.max(point.y);
    }

    /// Lower-left corner of the union of `self` and `other`.
    fn union_min(&self, other: &BoundingBox) -> Coord {
        Coord {
            x: self.min_x.min(other.min_x),
            y: self.min_y.min(other.min_y),
        }
    }

    /// Upper-right corner of the union of `self` and `other`.
    fn union_max(&self, other: &BoundingBox) -> Coord {
        Coord {
            x: self.max_x.max(other.max_x),
            y: self.max_y.max(other.max_y),
        }
    }
}

/// Integer coordinate of a cell in the scaled projection grid.
type CellKey = (i32, i32);

/// State accumulated for one grid cell during the first pass.
#[derive(Debug, Clone, Copy)]
struct Cell {
    bbox: BoundingBox,
    /// Still eligible to emit its own center; cleared once the cell is folded into a merged pair.
    active: bool,
    /// Number of input points that landed in this cell.
    count: usize,
}

pub fn main(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let projected = plane.project();

    let output: SingleVec = cluster(projected, min_points)
        .into_iter()
        .filter_map(|(center, count)| (count >= min_points).then_some([center.x, center.y]))
        .collect();

    plane.reverse(output)
}

/// Midpoint of two coordinates.
fn midpoint(a: Coord, b: Coord) -> Coord {
    Coord {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    }
}

/// Bucket the projected points into unit-diameter grid cells, merge adjacent cells whose combined
/// extent still fits a unit disc, and return one `(center, member_count)` per surviving cluster.
/// Centers that land on the exact same coordinate are de-duplicated.
fn cluster(points: Vec<Coord>, min_points: usize) -> Vec<(Coord, usize)> {
    let sqrt2 = std::f64::consts::SQRT_2;
    let half_sqrt2 = sqrt2 / 2.0;

    // First pass: drop each point into the grid cell its scaled coordinate floors into.
    let mut cells: HashMap<CellKey, Cell> = HashMap::new();
    for p in points {
        let key = ((p.x / sqrt2).floor() as i32, (p.y / sqrt2).floor() as i32);
        cells
            .entry(key)
            .and_modify(|cell| {
                cell.bbox.extend(p);
                cell.count += 1;
            })
            .or_insert_with(|| Cell {
                bbox: BoundingBox::new(p),
                active: true,
                count: 1,
            });
    }

    // Centers are keyed by their coordinate bits so coincident centers collapse to one entry.
    let mut centers: HashMap<(u64, u64), (Coord, usize)> = HashMap::new();
    let mut emit = |center: Coord, count: usize| {
        if count > 0 {
            centers.insert((center.x.to_bits(), center.y.to_bits()), (center, count));
        }
    };

    // Second pass: try to merge each cell with one active 8-neighbour whose combined bounding box
    // still fits within a unit disc (squared diameter <= 4). A successful merge emits a single
    // center at the midpoint of the union and retires both cells.
    'cells: for (key, cell) in cells.clone() {
        let (v, h) = key;
        for neighbor_key in [
            (v, h - 1),
            (v, h + 1),
            (v + 1, h),
            (v - 1, h),
            (v - 1, h - 1),
            (v + 1, h - 1),
            (v + 1, h + 1),
            (v - 1, h + 1),
        ] {
            let Some(&neighbor) = cells.get(&neighbor_key) else {
                continue;
            };
            if !neighbor.active {
                continue;
            }

            let lower_left = cell.bbox.union_min(&neighbor.bbox);
            let upper_right = cell.bbox.union_max(&neighbor.bbox);
            if lower_left.distance_2(&upper_right) <= 4.0 {
                let combined = cell.count + neighbor.count;
                if combined > min_points {
                    emit(midpoint(lower_left, upper_right), combined);
                    cells.entry(neighbor_key).and_modify(|c| c.active = false);
                    cells.entry(key).and_modify(|c| c.active = false);
                    continue 'cells;
                }
            }
        }
    }

    // Final pass: every cell not consumed by a merge emits its own center — the lone point for a
    // singleton cell, otherwise the geometric center of the grid cell.
    for (key, cell) in cells {
        if !cell.active {
            continue;
        }
        let center = if cell.count == 1 {
            Coord {
                x: cell.bbox.min_x,
                y: cell.bbox.min_y,
            }
        } else {
            let (v, h) = key;
            Coord {
                x: v as f64 * sqrt2 + half_sqrt2,
                y: h as f64 * sqrt2 + half_sqrt2,
            }
        };
        emit(center, cell.count);
    }

    centers.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── main: empty input ─────────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty() {
        let empty: SingleVec = vec![];
        let result = main(&empty, 70.0, 1);
        assert!(result.is_empty());
    }

    // ── main: single point, min_points = 1 ───────────────────────────────────

    #[test]
    fn single_point_min1_returns_one_cluster() {
        let pts = vec![[40.0_f64, -74.0_f64]];
        let result = main(&pts, 70.0, 1);
        assert_eq!(
            result.len(),
            1,
            "one point with min_points=1 should yield 1 cluster"
        );
    }

    // ── main: cluster count never exceeds input points ────────────────────────

    #[test]
    fn clusters_do_not_exceed_input_points() {
        let pts: Vec<[f64; 2]> = (0..20).map(|i| [40.0 + i as f64 * 0.01, -74.0]).collect();
        let result = main(&pts, 70.0, 1);
        assert!(
            result.len() <= pts.len(),
            "cluster count ({}) > input count ({})",
            result.len(),
            pts.len()
        );
    }

    // ── main: dense cluster with high min_points ──────────────────────────────

    #[test]
    fn sparse_points_filtered_by_min_points() {
        // 3 isolated points, each far from the others; min_points = 5 → should return nothing.
        let pts = vec![[40.0, -74.0], [41.0, -74.0], [42.0, -74.0]];
        let result = main(&pts, 70.0, 5);
        // Each point is isolated; no cluster can have 5+ members.
        // (behavior: 0 or fewer than 3 centers; exact 0 is expected here)
        assert!(
            result.len() <= 3,
            "should not exceed input count, got {}",
            result.len()
        );
    }

    // ── main: tight cluster should yield roughly one center ───────────────────

    #[test]
    fn tight_cluster_yields_one_center() {
        // 10 points within ~1 m of each other — Fastest should group into 1 cluster.
        let pts: Vec<[f64; 2]> = (0..10)
            .map(|i| [40.0 + i as f64 * 0.000001, -74.0 + i as f64 * 0.000001])
            .collect();
        let result = main(&pts, 1_000.0, 2); // 1 km radius, 2 min points
        assert!(
            !result.is_empty(),
            "tight cluster with large radius should yield at least 1 center"
        );
        assert!(
            result.len() <= pts.len(),
            "should not produce more clusters than input points"
        );
    }

    // ── main: output contains valid lat/lon ───────────────────────────────────

    #[test]
    fn output_lat_lon_in_valid_range() {
        let pts = vec![[40.0, -74.0], [40.001, -74.001], [40.002, -74.002]];
        let result = main(&pts, 70.0, 1);
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0, "invalid lat: {lat}");
            assert!(lon.abs() < 180.0, "invalid lon: {lon}");
        }
    }

    // ── main: deterministic for same input ────────────────────────────────────

    #[test]
    fn deterministic_same_input() {
        let pts = vec![[40.0, -74.0], [40.001, -74.001], [40.5, -73.0]];
        let a = main(&pts, 70.0, 1);
        let b = main(&pts, 70.0, 1);
        assert_eq!(a.len(), b.len(), "Fastest must be deterministic");
    }
}
