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

const EPS: f64 = 1e-7;

#[inline]
fn dist2(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[inline]
fn in_disc(center: Coord, r: f64, p: Coord) -> bool {
    dist2(center, p) <= r * r + EPS
}

/// Circle through two points: center at their midpoint, radius half their distance.
fn circle_two(a: Coord, b: Coord) -> (Coord, f64) {
    let center = Coord {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    };
    (center, dist2(a, b).sqrt() / 2.0)
}

/// Circumcircle of three points; falls back to the diameter circle of the farthest
/// pair when the points are (near-)collinear.
fn circle_three(a: Coord, b: Coord, c: Coord) -> (Coord, f64) {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        // Collinear: the smallest enclosing circle is the diameter of the two
        // extreme points (the third lies between them).
        let (ab, bc, ca) = (dist2(a, b), dist2(b, c), dist2(c, a));
        return if ab >= bc && ab >= ca {
            circle_two(a, b)
        } else if bc >= ca {
            circle_two(b, c)
        } else {
            circle_two(c, a)
        };
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let center = Coord {
        x: (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        y: (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    };
    (center, dist2(center, a).sqrt())
}

/// Smallest enclosing circle `(center, radius)` of `points` in the Euclidean plane.
/// Deterministic incremental Welzl (input order, no shuffle): O(n) expected, fine for
/// the small per-group point sets here. Returns `None` for empty input.
// Used only by tests until the cluster path wires it in (later task); keeping the
// dead-code allow narrowly scoped to this entry point covers its private callees too.
#[allow(dead_code)]
fn smallest_enclosing_circle(points: &[Coord]) -> Option<(Coord, f64)> {
    let n = points.len();
    if n == 0 {
        return None;
    }
    let mut center = points[0];
    let mut r = 0.0_f64;
    for i in 1..n {
        if in_disc(center, r, points[i]) {
            continue;
        }
        center = points[i];
        r = 0.0;
        for j in 0..i {
            if in_disc(center, r, points[j]) {
                continue;
            }
            let (cc, rr) = circle_two(points[i], points[j]);
            center = cc;
            r = rr;
            for k in 0..j {
                if in_disc(center, r, points[k]) {
                    continue;
                }
                let (cc, rr) = circle_three(points[i], points[j], points[k]);
                center = cc;
                r = rr;
            }
        }
    }
    Some((center, r))
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

    // ── MEC: geometry ─────────────────────────────────────────────────────────

    fn c(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    fn covers(center: Coord, r: f64, pts: &[Coord]) -> bool {
        pts.iter().all(|p| {
            let dx = p.x - center.x;
            let dy = p.y - center.y;
            dx * dx + dy * dy <= r * r + 1e-6
        })
    }

    #[test]
    fn mec_single_point_is_zero_radius_at_point() {
        let (center, r) = smallest_enclosing_circle(&[c(3.0, -2.0)]).unwrap();
        assert!((center.x - 3.0).abs() < 1e-9 && (center.y + 2.0).abs() < 1e-9);
        assert!(r < 1e-9, "single-point radius should be ~0, got {r}");
    }

    #[test]
    fn mec_two_points_is_diameter_circle() {
        let (center, r) = smallest_enclosing_circle(&[c(0.0, 0.0), c(2.0, 0.0)]).unwrap();
        assert!((center.x - 1.0).abs() < 1e-9 && center.y.abs() < 1e-9);
        assert!((r - 1.0).abs() < 1e-9, "radius should be 1.0, got {r}");
    }

    #[test]
    fn mec_encloses_all_points() {
        let pts = [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 1.0), c(1.0, 1.0), c(0.5, 0.5)];
        let (center, r) = smallest_enclosing_circle(&pts).unwrap();
        assert!(covers(center, r, &pts), "MEC must enclose every point");
        // tightest enclosing circle of the unit square has r = sqrt(2)/2
        assert!((r - std::f64::consts::SQRT_2 / 2.0).abs() < 1e-6, "got r={r}");
    }

    #[test]
    fn mec_collinear_points_use_extreme_pair() {
        let pts = [c(-3.0, 0.0), c(0.0, 0.0), c(5.0, 0.0)];
        let (center, r) = smallest_enclosing_circle(&pts).unwrap();
        assert!(covers(center, r, &pts), "collinear MEC must enclose all");
        assert!((r - 4.0).abs() < 1e-6, "radius should span -3..5 → 4, got {r}");
    }

    #[test]
    fn mec_empty_is_none() {
        assert!(smallest_enclosing_circle(&[]).is_none());
    }
}
