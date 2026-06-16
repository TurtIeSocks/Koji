use ::s2::cell::Cell;
use ::s2::cellid::CellID;
use ::s2::latlng::LatLng;
use koji_core::KojiBbox;
use koji_core::PointArray;
use koji_core::Precision;

use rstar::{AABB, RTree};

use crate::clustering::candidates;
use crate::clustering::rtree::point::Point;

/// Shared 1024 constant used as both a unit byte size (1 KiB) and a candidate
/// grid-density base in greedy.rs. Centralized here to keep all `1024` magic
/// numbers in clustering on a single source of truth.
pub(crate) const BYTE: usize = 1024;

pub(crate) fn cell_bbox_lat_lon(cell: CellID) -> KojiBbox {
    let c = Cell::from(&cell);
    // Use vertex extremes; avoids rect_bound() API churn between s2 versions.
    let mut bb = KojiBbox {
        min_lat: Precision::INFINITY,
        max_lat: Precision::NEG_INFINITY,
        min_lon: Precision::INFINITY,
        max_lon: Precision::NEG_INFINITY,
    };
    for i in 0..4 {
        let v = c.vertex(i);
        let lat = v.latitude().deg();
        let lon = v.longitude().deg();
        bb.min_lat = bb.min_lat.min(lat);
        bb.max_lat = bb.max_lat.max(lat);
        bb.min_lon = bb.min_lon.min(lon);
        bb.max_lon = bb.max_lon.max(lon);
    }
    bb
}

/// Ownership predicate: does `p` belong to `cell` at the cell's own level?
///
/// Used by crucible during the post-solve ownership filter. The deterministic
/// `LatLng → parent(level)` rule ensures a cluster center is owned by exactly
/// one chunk.
pub(crate) fn contains_latlng(cell: CellID, p: PointArray) -> bool {
    let derived = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(cell.level());
    derived == cell
}

/// Collect halo points: those that intersect `cell`'s bbox expanded by `radius_meters`
/// but do NOT belong to `cell`. Halo points participate in a chunk's solve as
/// coverage targets but never own cluster centers, so adjacent chunks can place
/// edge clusters fairly without producing duplicates.
pub(crate) fn gather_halo(
    cell: CellID,
    all_points_tree: &RTree<Point>,
    radius_meters: Precision,
) -> Vec<Point> {
    let bbox = cell_bbox_lat_lon(cell);
    let radius_deg = candidates::meters_to_degrees(radius_meters, bbox.center_lat());
    let expanded = bbox.expand(radius_deg);

    let envelope = AABB::from_corners(
        [expanded.min_lat, expanded.min_lon],
        [expanded.max_lat, expanded.max_lon],
    );

    all_points_tree
        .locate_in_envelope_intersecting(&envelope)
        .filter(|p| !contains_latlng(cell, p.center))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::{Precision, SingleVec};
    use rand::{Rng, SeedableRng, rngs::SmallRng};

    pub(super) fn random_points_in_bbox(n: usize, bbox: [Precision; 4], seed: u64) -> SingleVec {
        let [min_lat, min_lon, max_lat, max_lon] = bbox;
        let mut rng = SmallRng::seed_from_u64(seed);
        (0..n)
            .map(|_| {
                let lat = rng.random_range(min_lat..max_lat);
                let lon = rng.random_range(min_lon..max_lon);
                [lat, lon]
            })
            .collect()
    }

    pub(super) fn dense_cluster(
        center: [Precision; 2],
        n: usize,
        radius_m: Precision,
        seed: u64,
    ) -> SingleVec {
        let mut rng = SmallRng::seed_from_u64(seed);
        // Approx 1 deg lat ≈ 111_000 m; ignore lon shrinkage for tiny test radii
        let radius_deg = radius_m / 111_000.0;
        (0..n)
            .map(|_| {
                let dlat = rng.random_range(-radius_deg..radius_deg);
                let dlon = rng.random_range(-radius_deg..radius_deg);
                [center[0] + dlat, center[1] + dlon]
            })
            .collect()
    }

    pub(super) fn sparse_grid(rows: usize, cols: usize, bbox: [Precision; 4]) -> SingleVec {
        let [min_lat, min_lon, max_lat, max_lon] = bbox;
        let lat_step = (max_lat - min_lat) / rows as Precision;
        let lon_step = (max_lon - min_lon) / cols as Precision;
        (0..rows)
            .flat_map(|r| {
                (0..cols).map(move |c| {
                    [
                        min_lat + (r as Precision + 0.5) * lat_step,
                        min_lon + (c as Precision + 0.5) * lon_step,
                    ]
                })
            })
            .collect()
    }

    #[test]
    fn helpers_produce_expected_shapes() {
        let pts = random_points_in_bbox(100, [0., 0., 1., 1.], 42);
        assert_eq!(pts.len(), 100);
        for p in &pts {
            assert!(p[0] >= 0.0 && p[0] < 1.0);
            assert!(p[1] >= 0.0 && p[1] < 1.0);
        }

        let cluster = dense_cluster([10., 20.], 50, 100.0, 7);
        assert_eq!(cluster.len(), 50);

        let grid = sparse_grid(5, 5, [0., 0., 10., 10.]);
        assert_eq!(grid.len(), 25);
    }

    #[test]
    fn contains_latlng_owns_only_its_cell() {
        // Pick a stable lat/lon, get its level-10 parent.
        let center = LatLng::from_degrees(37.7749, -122.4194); // SF
        let owning_cell = CellID::from(center).parent(10);

        // The lat/lon itself must be contained.
        assert!(contains_latlng(owning_cell, [37.7749, -122.4194]));

        // A point on the opposite side of the world is not contained.
        assert!(!contains_latlng(owning_cell, [-37.7749, 57.5806]));
    }

    #[test]
    fn cell_bbox_lat_lon_is_finite_and_ordered() {
        let center = LatLng::from_degrees(0., 0.);
        let cell = CellID::from(center).parent(8);
        let bb = cell_bbox_lat_lon(cell);
        assert!(bb.min_lat.is_finite() && bb.max_lat.is_finite());
        assert!(bb.min_lon.is_finite() && bb.max_lon.is_finite());
        assert!(bb.min_lat <= bb.max_lat);
        assert!(bb.min_lon <= bb.max_lon);
    }

    #[test]
    fn gather_halo_includes_nearby_points_outside_cell() {
        use crate::clustering::rtree::point::Point;
        use crate::rtree;

        // Pick a level-12 cell; place one point inside, one just outside (within radius).
        let inside = [37.7749, -122.4194];
        let cell = CellID::from(LatLng::from_degrees(inside[0], inside[1])).parent(12);
        let bbox = cell_bbox_lat_lon(cell);

        // Point just past the east edge (10 meters past, well under default 70m radius).
        let outside = [bbox.max_lat - 0.00001, bbox.max_lon + 0.00005];

        let all_points: SingleVec = vec![inside, outside];
        let tree = rtree::spawn(70.0, &all_points);

        let halo = gather_halo(cell, &tree, 70.0);
        assert!(
            halo.iter()
                .any(|p: &Point| (p.center[1] - outside[1]).abs() < 1e-6),
            "halo should include the just-outside point"
        );
        assert!(
            !halo
                .iter()
                .any(|p: &Point| (p.center[1] - inside[1]).abs() < 1e-6),
            "halo should NOT include the inside point"
        );
    }

    // ── contains_latlng: level edge cases ────────────────────────────────────

    #[test]
    fn contains_latlng_level0_contains_everything() {
        // Level-0 cell = one full face; any point on that face is contained.
        // Use a level-6 cell and check its center.
        use s2::latlng::LatLng;
        let center = LatLng::from_degrees(40.0, -74.0);
        let cell = CellID::from(center).parent(6);
        assert!(
            contains_latlng(cell, [40.0, -74.0]),
            "a cell should contain its own center point"
        );
    }

    #[test]
    fn contains_latlng_far_point_not_in_local_cell() {
        // A cell at one lat/lon should not contain a point on the opposite side of the globe.
        use s2::latlng::LatLng;
        let local_cell = CellID::from(LatLng::from_degrees(40.0, -74.0)).parent(10);
        // Antipodal-ish point is definitely not in this local cell.
        assert!(
            !contains_latlng(local_cell, [-40.0, 106.0]),
            "antipodal point should not be contained"
        );
    }

    // ── cell_bbox_lat_lon: non-equatorial cells ────────────────────────────────

    #[test]
    fn cell_bbox_at_high_latitude() {
        use s2::latlng::LatLng;
        let cell = CellID::from(LatLng::from_degrees(80.0, 0.0)).parent(8);
        let bb = cell_bbox_lat_lon(cell);
        assert!(bb.min_lat >= 0.0, "min_lat should be positive at high N latitude");
        assert!(bb.max_lat <= 90.0);
        assert!(bb.min_lat <= bb.max_lat);
    }

    #[test]
    fn cell_bbox_south_hemisphere() {
        use s2::latlng::LatLng;
        let cell = CellID::from(LatLng::from_degrees(-45.0, 150.0)).parent(8);
        let bb = cell_bbox_lat_lon(cell);
        assert!(bb.max_lat <= 0.0, "max_lat should be ≤ 0 for southern hemisphere cell at -45°");
        assert!(bb.min_lat <= bb.max_lat);
        assert!(bb.min_lon <= bb.max_lon);
    }
}
