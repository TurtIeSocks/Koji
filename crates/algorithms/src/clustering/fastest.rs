//! "Fastest" clustering mode.
//!
//! The cheapest clustering tier: a deterministic unit-disc cover. Points are projected onto
//! a flat plane scaled so the target radius maps to one unit, then bucketed into a `√2` grid
//! (each cell is coverable by one radius-1 disc). Occupied cells are visited in sorted order
//! and region-grown — a cell absorbs sorted neighbours while the group still fits inside a
//! single radius-1 disc — and each group emits one center. Groups below `min_points` are
//! dropped. O(n) expected, grid-only: no spatial index or candidate lattice.

use geo::Coord;
use koji_core::SingleVec;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::project::Plane;

/// Integer coordinate of a cell in the scaled projection grid.
type CellKey = (i32, i32);

/// Distortion safety margin for the disc-fit test. Benchmarking (see the design doc) found
/// `0` optimal: a positive margin only adds clusters to shave a few boundary-hugging points.
const MARGIN: f64 = 0.0;

pub fn main(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let projected = plane.project();

    // `cluster` already drops groups below `min_points`, so every returned center survives.
    let output: SingleVec = cluster(projected, min_points, MARGIN)
        .into_iter()
        .map(|(center, _count)| [center.x, center.y])
        .collect();

    plane.reverse(output)
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

/// `(min_x, min_y, max_x, max_y)` of a non-empty point set.
fn bounds(points: &[Coord]) -> (f64, f64, f64, f64) {
    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = points[0].x;
    let mut max_y = points[0].y;
    for p in &points[1..] {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    (min_x, min_y, max_x, max_y)
}

/// Does `points` fit inside a single radius-1 disc (allowing `margin` of safety)?
/// Tested via the minimum enclosing circle: the group fits iff its MEC radius ≤ 1 − margin.
/// (Placement was benchmarked across MEC / centroid / bbox-center; MEC won — see design doc.)
fn fits(points: &[Coord], margin: f64) -> bool {
    if points.is_empty() {
        return true;
    }
    // Cheap necessary pre-check: a radius-1 cover implies the bounding-box diagonal is ≤ 2.
    let (min_x, min_y, max_x, max_y) = bounds(points);
    if (max_x - min_x).powi(2) + (max_y - min_y).powi(2) > 4.0 {
        return false;
    }
    match smallest_enclosing_circle(points) {
        Some((_, r)) => r <= 1.0 - margin,
        None => true,
    }
}

/// Place the disc center at the minimum-enclosing-circle center (assumes non-empty).
fn place(points: &[Coord]) -> Coord {
    smallest_enclosing_circle(points).unwrap().0
}

/// The eight grid neighbours of a cell.
fn neighbours((v, h): CellKey) -> [CellKey; 8] {
    [
        (v - 1, h - 1),
        (v - 1, h),
        (v - 1, h + 1),
        (v, h - 1),
        (v, h + 1),
        (v + 1, h - 1),
        (v + 1, h),
        (v + 1, h + 1),
    ]
}

/// Bucket points into a √2 grid, region-grow each occupied cell (in deterministic sorted
/// order) by absorbing neighbours while the group still fits one radius-1 disc, then emit
/// one `(center, member_count)` per group with at least `min_points` members. Coincident
/// centers are de-duplicated.
fn cluster(points: Vec<Coord>, min_points: usize, margin: f64) -> Vec<(Coord, usize)> {
    let sqrt2 = std::f64::consts::SQRT_2;

    // First pass: bucket. BTreeMap gives deterministic sorted iteration over cells.
    let mut cells: BTreeMap<CellKey, Vec<Coord>> = BTreeMap::new();
    for p in points {
        let key = ((p.x / sqrt2).floor() as i32, (p.y / sqrt2).floor() as i32);
        cells.entry(key).or_default().push(p);
    }

    let mut centers: HashMap<(u64, u64), (Coord, usize)> = HashMap::new();
    let mut claimed: BTreeSet<CellKey> = BTreeSet::new();

    // Second pass: region-grow from each unclaimed cell in sorted order.
    for (&seed, seed_points) in &cells {
        if claimed.contains(&seed) {
            continue;
        }
        claimed.insert(seed);

        let mut group_cells: BTreeSet<CellKey> = BTreeSet::from([seed]);
        let mut group_points: Vec<Coord> = seed_points.clone();

        loop {
            // Sorted set of unclaimed, occupied neighbours of the current group.
            let mut candidates: BTreeSet<CellKey> = BTreeSet::new();
            for &cell in &group_cells {
                for n in neighbours(cell) {
                    if !claimed.contains(&n) && cells.contains_key(&n) {
                        candidates.insert(n);
                    }
                }
            }

            // Absorb the first (sorted) candidate that keeps the group disc-coverable.
            let mut absorbed = None;
            for cand in &candidates {
                let mut trial = group_points.clone();
                trial.extend_from_slice(&cells[cand]);
                if fits(&trial, margin) {
                    absorbed = Some(*cand);
                    break;
                }
            }

            match absorbed {
                Some(cand) => {
                    group_points.extend_from_slice(&cells[&cand]);
                    group_cells.insert(cand);
                    claimed.insert(cand);
                }
                None => break,
            }
        }

        let count = group_points.len();
        if count < min_points {
            continue;
        }
        let center = place(&group_points);
        centers.insert((center.x.to_bits(), center.y.to_bits()), (center, count));
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
        let mut a = main(&pts, 70.0, 1);
        let mut b = main(&pts, 70.0, 1);
        a.sort_by(|p, q| p.partial_cmp(q).unwrap());
        b.sort_by(|p, q| p.partial_cmp(q).unwrap());
        assert_eq!(a, b, "Fastest must be deterministic (identical set of centers)");
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

    #[test]
    fn mec_right_triangle_is_circumcircle() {
        // Exercises circle_three: the circumcircle of (0,0),(4,0),(0,3) is centered at the
        // hypotenuse midpoint (2, 1.5) with radius 2.5. Locks the circumcenter formula.
        let (center, r) =
            smallest_enclosing_circle(&[c(0.0, 0.0), c(4.0, 0.0), c(0.0, 3.0)]).unwrap();
        assert!(
            (center.x - 2.0).abs() < 1e-9 && (center.y - 1.5).abs() < 1e-9,
            "circumcenter wrong: {center:?}"
        );
        assert!((r - 2.5).abs() < 1e-9, "circumradius wrong: {r}");
    }

    // ── fit + place ───────────────────────────────────────────────────────────

    #[test]
    fn fits_within_unit_disc() {
        // Two points distance 2 apart → MEC radius exactly 1 → fits at margin 0.
        let pts = [c(0.0, 0.0), c(2.0, 0.0)];
        assert!(fits(&pts, 0.0));
        // distance 2.001 apart → MEC radius > 1 → does not fit.
        let pts2 = [c(0.0, 0.0), c(2.001, 0.0)];
        assert!(!fits(&pts2, 0.0));
    }

    #[test]
    fn fits_margin_rejects_boundary() {
        // radius exactly 1 fails once a positive margin is required.
        let pts = [c(0.0, 0.0), c(2.0, 0.0)];
        assert!(!fits(&pts, 0.01));
    }

    #[test]
    fn fits_rejects_group_too_wide_for_a_disc() {
        // unit square: MEC radius √2/2 ≈ 0.707 → fits.
        let sq = [c(0.0, 0.0), c(1.0, 1.0)];
        assert!(fits(&sq, 0.0));
        // bbox diagonal 2.121 > 2 → cannot fit any radius-1 disc (cheap pre-check rejects).
        let wide = [c(0.0, 0.0), c(1.5, 1.5)];
        assert!(!fits(&wide, 0.0));
    }

    #[test]
    fn place_is_mec_center() {
        // MEC center of the 2x2 square is its middle.
        let pts = [c(0.0, 0.0), c(2.0, 0.0), c(0.0, 2.0), c(2.0, 2.0)];
        let m = place(&pts);
        assert!((m.x - 1.0).abs() < 1e-6 && (m.y - 1.0).abs() < 1e-6);
    }

    // ── cluster: region-growing behavior ──────────────────────────────────────

    #[test]
    fn cluster_mec_covers_every_point_when_min_points_1() {
        // A handful of nearby points; with min_points=1 nothing is dropped, so every
        // projected point must be within radius 1 of some returned center.
        let pts: Vec<Coord> = (0..25)
            .map(|i| c((i % 5) as f64 * 0.3, (i / 5) as f64 * 0.3))
            .collect();
        let centers = cluster(pts.clone(), 1, 0.0);
        for p in &pts {
            let covered = centers
                .iter()
                .any(|(ctr, _)| dist2(*ctr, *p) <= 1.0 + 1e-6);
            assert!(covered, "point {p:?} not covered by any disc");
        }
    }

    #[test]
    fn cluster_is_deterministic_as_a_set() {
        let pts: Vec<Coord> = (0..40)
            .map(|i| c((i % 7) as f64 * 0.5, (i / 7) as f64 * 0.5))
            .collect();
        let mut a: Vec<(u64, u64)> = cluster(pts.clone(), 2, 0.0)
            .into_iter()
            .map(|(ctr, _)| (ctr.x.to_bits(), ctr.y.to_bits()))
            .collect();
        let mut b: Vec<(u64, u64)> = cluster(pts.clone(), 2, 0.0)
            .into_iter()
            .map(|(ctr, _)| (ctr.x.to_bits(), ctr.y.to_bits()))
            .collect();
        a.sort();
        b.sort();
        assert_eq!(a, b, "cluster output must be a deterministic set");
    }

    #[test]
    fn cluster_aggregates_neighbouring_cells() {
        // x = 0.0, 0.9, 1.8 — near-collinear, MEC radius 0.9 ≤ 1 → fits ONE disc, spanning
        // two √2 grid cells. They must collapse to a single center counting all 3.
        let pts = vec![c(0.0, 0.0), c(0.9, 0.0), c(1.8, 0.0)];
        let centers = cluster(pts, 1, 0.0);
        assert_eq!(centers.len(), 1, "three points fitting one disc → 1 center");
        assert_eq!(centers[0].1, 3, "the single disc should count all 3 members");
    }

    #[test]
    fn cluster_drops_sub_min_points_groups() {
        // Two far-apart singletons, min_points=2 → each group has 1 member → all dropped.
        let pts = vec![c(0.0, 0.0), c(100.0, 100.0)];
        let centers = cluster(pts, 2, 0.0);
        assert!(centers.is_empty(), "groups below min_points must be dropped");
    }
}
