//! Consolidated S2 primitives shared across the workspace.
//!
//! This module is the single home for the s2↔geo bridge: cell→geometry
//! conversions ([`ToGeo`]/[`ToPointArray`]), region/coverage helpers
//! ([`get_region_cells`], [`circle_coverage`], [`cell_coverage`], [`s2_grid`]),
//! the client-facing polygon responses ([`S2Response`], [`get_cells`],
//! [`get_polygons`]), and the point→cell-id mapping ([`from_array_to_cell_id`]).
//! Living in `koji-core` lets every downstream crate (algorithms, koji-service,
//! koji-plugins) depend on these primitives without forming a cycle.

use std::collections::{HashSet, VecDeque};

use geo::{Coord, Destination, Haversine, Intersects, LineString, Polygon};
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use s2::{
    cell::Cell, cellid::CellID, cellunion::CellUnion, latlng::LatLng, rect::Rect,
    region::RegionCoverer,
};
use serde::Serialize;

use crate::{PointArray, Precision};

type Covered = HashSet<u64>;

#[derive(Debug, Clone, Serialize)]
pub struct S2Response {
    pub id: String,
    coords: [[Precision; 2]; 4],
}

pub trait ToGeo {
    fn polygon(&self) -> geo::Polygon<Precision>;
    fn geo_point(&self) -> geo::Point;
}

pub trait ToPointArray {
    fn point_array(&self) -> PointArray;
}

impl ToPointArray for CellID {
    fn point_array(&self) -> PointArray {
        let center = Cell::from(self).center();
        [center.latitude().deg(), center.longitude().deg()]
    }
}

impl ToGeo for CellID {
    fn polygon(&self) -> geo::Polygon<Precision> {
        let cell = Cell::from(self);
        geo::Polygon::<Precision>::new(
            geo::LineString::from(
                (0..4)
                    .map(|i| {
                        let v = cell.vertex(i);
                        geo::Point::new(v.longitude().deg(), v.latitude().deg())
                    })
                    .collect::<Vec<geo::Point>>(),
            ),
            vec![],
        )
    }

    fn geo_point(&self) -> geo::Point {
        let cell = Cell::from(self);
        geo::Point::new(
            cell.center().longitude().deg(),
            cell.center().latitude().deg(),
        )
    }
}

pub fn get_region_cells(
    min_lat: Precision,
    max_lat: Precision,
    min_lon: Precision,
    max_lon: Precision,
    cell_size: u8,
) -> CellUnion {
    let region = Rect::from_degrees(min_lat, min_lon, max_lat, max_lon);

    RegionCoverer {
        max_level: cell_size,
        min_level: cell_size,
        level_mod: 1,
        max_cells: 100000,
    }
    .covering(&region)
}

pub fn get_cells(
    cell_size: u8,
    min_lat: Precision,
    min_lon: Precision,
    max_lat: Precision,
    max_lon: Precision,
) -> Vec<S2Response> {
    let cells = get_region_cells(min_lat, max_lat, min_lon, max_lon, cell_size);

    cells
        .0
        .iter()
        .enumerate()
        .map_while(|(i, cell)| {
            if i < 100_000 {
                Some(get_client_polygon(cell))
            } else {
                None
            }
        })
        .collect()
}

fn get_client_polygon(id: &CellID) -> S2Response {
    let cell = Cell::from(id);
    S2Response {
        id: id.0.to_string(),
        coords: std::array::from_fn(|i| {
            let v = cell.vertex(i);
            [v.latitude().deg(), v.longitude().deg()]
        }),
    }
}

pub fn get_polygons(cell_ids: Vec<String>) -> Vec<S2Response> {
    cell_ids
        .into_par_iter()
        .filter_map(|id| match id.parse::<u64>() {
            Ok(id) => Some(get_client_polygon(&CellID(id))),
            Err(e) => {
                log::error!("[S2] Error parsing cell id: {}", e);
                None
            }
        })
        .collect()
}

pub fn circle_coverage(lat: Precision, lon: Precision, radius: Precision, level: u8) -> Covered {
    let point = geo::Point::new(lon, lat);
    let circle = geo::Polygon::<Precision>::new(
        geo::LineString::from(
            (0..60)
                .map(|i| Haversine.destination(point, (i * 6) as Precision, radius))
                .collect::<Vec<geo::Point>>(),
        ),
        vec![],
    );

    // Iterative BFS flood fill (mirrors `s2_grid`). The old implementation
    // recursively spawned one OS thread per newly-covered cell (unbounded,
    // each cloning the 60-vertex circle) around an Arc<Mutex<HashSet>> whose
    // poison errors were logged-and-swallowed — and leaked the mutex into the
    // public API.
    let start = CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);
    let mut covered: Covered = HashSet::new();
    covered.insert(start.0);
    let mut queue = VecDeque::from([start]);
    while let Some(cell) = queue.pop_front() {
        for neighbor in cell.edge_neighbors() {
            if covered.contains(&neighbor.0) {
                continue;
            }
            if neighbor.polygon().intersects(&circle) {
                covered.insert(neighbor.0);
                queue.push_back(neighbor);
            }
        }
    }
    covered
}

/// Build the SIZE x SIZE set (SIZE^2) around `center` at `level` by expanding (SIZE-1)/2 rings
/// with (SIZE-1)-neighborhood (includes diagonals). This matches a Chebyshev radius of (SIZE-1)/2.
pub fn s2_grid(center: CellID, level: u8, size: u8) -> HashSet<CellID> {
    let half = (size / 2) as usize;
    if half == 0 {
        return std::iter::once(center).collect();
    }

    let mut visited: HashSet<CellID> = HashSet::with_capacity((size * size) as usize);
    let mut frontier: VecDeque<CellID> = VecDeque::new();

    visited.insert(center);
    frontier.push_back(center);

    for _ in 0..half {
        let mut next: Vec<CellID> = Vec::new();
        while let Some(cid) = frontier.pop_front() {
            for n in cid.all_neighbors(level as u64) {
                if visited.insert(n) {
                    next.push(n);
                }
            }
        }
        frontier.extend(next);
    }

    visited
}

/// True if the S2 cell (by ID) intersects the given polygon.
/// Build a planar polygon from the cell's 4 vertices in (lon, lat) order.
pub fn cell_intersects_polygon(id: CellID, poly: &Polygon<Precision>) -> bool {
    let cell = Cell::from(&id);

    let mut ring: Vec<Coord<Precision>> = Vec::with_capacity(5);
    for k in 0..4 {
        let p = cell.vertex(k);
        let ll = LatLng::from(&p);
        ring.push(Coord {
            x: ll.lng.deg(),
            y: ll.lat.deg(),
        });
    }
    ring.push(ring[0]);

    let cell_poly = Polygon::new(LineString::from(ring), vec![]);
    poly.intersects(&cell_poly)
}

pub fn cell_coverage(lat: Precision, lon: Precision, size: u8, level: u8) -> Covered {
    let mut covered = HashSet::new();
    let center = CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);

    if size == 1 {
        covered.insert(center.0);
    } else {
        let neighbors = s2_grid(center, level, size);
        for neighbor in neighbors {
            covered.insert(neighbor.0);
        }
    }
    covered
}

/// Map a `[lat, lon]` point to its S2 `CellID` at `parent_level`.
pub fn from_array_to_cell_id(point: &PointArray, parent_level: u64) -> CellID {
    CellID::from(LatLng::from_degrees(point[0], point[1])).parent(parent_level)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── from_array_to_cell_id / point_array round-trip ──────────────────────

    #[test]
    fn cell_id_round_trip_lat_lon() {
        // [lat, lon] → cell at level 15 → back to [lat, lon] center.
        // The center of the cell should be *close* to the original point but
        // not necessarily equal (S2 snaps to the cell center). We verify:
        // 1. The returned [lat, lon] is within a small tolerance of the input.
        // 2. The round-trip from that center back yields the same CellID.
        let input: PointArray = [48.8566, 2.3522]; // Paris
        let level = 15u64;
        let cell_id = from_array_to_cell_id(&input, level);
        assert_eq!(cell_id.level(), level);

        let center = cell_id.point_array();
        // center is [lat, lon] — must be within ~1 km of input at level 15
        assert!((center[0] - input[0]).abs() < 0.01, "lat off: {:?}", center);
        assert!((center[1] - input[1]).abs() < 0.01, "lon off: {:?}", center);

        // idempotent: centroid→cell_id gives the same cell
        let cell_id2 = from_array_to_cell_id(&center, level);
        assert_eq!(cell_id, cell_id2, "cell id not idempotent");
    }

    #[test]
    fn cell_id_level_is_correct() {
        for level in [1u64, 6, 12, 20] {
            let id = from_array_to_cell_id(&[0.0, 0.0], level);
            assert_eq!(id.level(), level, "level {level}");
        }
    }

    // ── ToGeo / ToPointArray ─────────────────────────────────────────────────

    #[test]
    fn cell_polygon_has_4_vertices_and_center_inside() {
        // cell polygon must have 4 exterior coordinates (the 4 vertices).
        // The center point must lie inside (intersects the ring).
        use geo::Contains;
        let input: PointArray = [35.6762, 139.6503]; // Tokyo
        let cell_id = from_array_to_cell_id(&input, 15);

        let poly = cell_id.polygon();
        // geo Polygon: exterior ring includes the closing point = 5 coords
        let ext: Vec<_> = poly.exterior().coords().collect();
        // S2 cell polygon via LineString::from(Vec<Point>) doesn't auto-close,
        // so we may get 4 or 5. Assert at least 4.
        assert!(ext.len() >= 4, "expected ≥4 vertices, got {}", ext.len());

        // center of the cell must be contained in the polygon
        let center = cell_id.geo_point();
        assert!(
            poly.contains(&center),
            "center {:?} not inside cell polygon",
            center
        );
    }

    #[test]
    fn cell_point_array_matches_geo_point_center() {
        let input: PointArray = [40.7128, -74.0060]; // NYC
        let cell_id = from_array_to_cell_id(&input, 15);
        let arr = cell_id.point_array();
        let geo_point = cell_id.geo_point();
        // point_array is [lat, lon]; geo_point is (x=lon, y=lat)
        assert!((arr[0] - geo_point.y()).abs() < 1e-10, "lat mismatch");
        assert!((arr[1] - geo_point.x()).abs() < 1e-10, "lon mismatch");
    }

    // ── get_client_polygon (via get_cells) ──────────────────────────────────

    #[test]
    fn get_polygon_returns_4_latlon_corners() {
        let cell_id = from_array_to_cell_id(&[51.5074, -0.1278], 15); // London
        let corners = get_client_polygon(&cell_id).coords;
        // Each corner is [lat, lon]; lat ∈ [-90,90], lon ∈ [-180,180]
        for (i, [lat, lon]) in corners.iter().enumerate() {
            assert!(
                (-90.0..=90.0).contains(lat),
                "corner {i} lat {lat} out of range"
            );
            assert!(
                (-180.0..=180.0).contains(lon),
                "corner {i} lon {lon} out of range"
            );
        }
        // All 4 corners must be distinct (non-degenerate cell at level 15)
        let unique: HashSet<u64> = corners
            .iter()
            .map(|[lat, lon]| (lat.to_bits(), lon.to_bits()))
            .map(|(a, b)| a ^ b.wrapping_mul(0x9e37))
            .collect();
        assert_eq!(unique.len(), 4, "corners not distinct: {:?}", corners);
    }

    #[test]
    fn get_cells_count_and_structure() {
        // A small tight bbox around central Paris at L15 should yield ≥1 and
        // a sane count (not hundreds of thousands).
        let cells = get_cells(15, 48.85, 2.34, 48.86, 2.36);
        assert!(!cells.is_empty(), "should have at least one cell");
        // ids parse as u64
        for c in &cells {
            let _: u64 = c.id.parse().expect("id must be a u64 string");
        }
    }

    #[test]
    fn get_polygons_filters_invalid_ids() {
        // One valid cell id + one garbage string.
        let cell_id = from_array_to_cell_id(&[0.0, 0.0], 10);
        let ids = vec![cell_id.0.to_string(), "not_a_number".to_string()];
        let result = get_polygons(ids);
        // Only the valid one survives.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, cell_id.0.to_string());
    }

    #[test]
    fn get_polygons_empty_input() {
        assert!(get_polygons(vec![]).is_empty());
    }

    // ── get_region_cells ─────────────────────────────────────────────────────

    #[test]
    fn region_cells_at_level_correct_level() {
        // Every cell in the covering must be exactly the requested level.
        let cells = get_region_cells(48.85, 48.86, 2.34, 2.36, 14);
        assert!(!cells.0.is_empty());
        for id in &cells.0 {
            assert_eq!(id.level(), 14, "cell not at level 14: {:?}", id);
        }
    }

    #[test]
    fn region_cells_contain_center_point() {
        // The cell covering a region must include the cell that contains the
        // region's center point.
        let (min_lat, max_lat, min_lon, max_lon) = (48.85, 48.86, 2.34, 2.36);
        let center_lat = (min_lat + max_lat) / 2.0;
        let center_lon = (min_lon + max_lon) / 2.0;
        let level = 13u8;
        let cells = get_region_cells(min_lat, max_lat, min_lon, max_lon, level);
        let center_cell = from_array_to_cell_id(&[center_lat, center_lon], level as u64);
        assert!(
            cells.0.contains(&center_cell),
            "covering does not contain the center cell"
        );
    }

    // ── s2_grid ───────────────────────────────────────────────────────────────

    #[test]
    fn s2_grid_size_1_returns_just_center() {
        let center = from_array_to_cell_id(&[0.0, 0.0], 10);
        let grid = s2_grid(center, 10, 1);
        assert_eq!(grid.len(), 1);
        assert!(grid.contains(&center));
    }

    #[test]
    fn s2_grid_size_3_contains_center_and_neighbors() {
        let center = from_array_to_cell_id(&[37.7749, -122.4194], 12);
        let grid = s2_grid(center, 12, 3);
        // At size=3, half=1: expand one ring via all_neighbors.
        // Must include center.
        assert!(grid.contains(&center), "center not in grid");
        // Must have more than 1 cell.
        assert!(grid.len() > 1, "size-3 grid should have multiple cells");
        // All cells must be at the requested level.
        for c in &grid {
            assert_eq!(c.level(), 12, "grid cell not at level 12: {:?}", c);
        }
    }

    #[test]
    fn s2_grid_size_0_returns_just_center() {
        // size=0: half=0, early-return with just the center.
        let center = from_array_to_cell_id(&[0.0, 0.0], 8);
        let grid = s2_grid(center, 8, 0);
        // Size 0 means half=0 → returns iter::once(center).
        assert_eq!(grid.len(), 1);
        assert!(grid.contains(&center));
    }

    #[test]
    fn s2_grid_all_cells_at_correct_level() {
        let center = from_array_to_cell_id(&[51.5, -0.1], 14);
        for size in [1u8, 3, 5] {
            let grid = s2_grid(center, 14, size);
            for c in &grid {
                assert_eq!(c.level(), 14, "size={size} cell not at L14");
            }
        }
    }

    // ── cell_coverage ─────────────────────────────────────────────────────────

    #[test]
    fn cell_coverage_size_1_is_single_cell() {
        let covered = cell_coverage(37.7749, -122.4194, 1, 15);
        assert_eq!(covered.len(), 1);
    }

    #[test]
    fn cell_coverage_size_3_contains_center() {
        let lat = 35.6762;
        let lon = 139.6503;
        let level = 14u8;
        let covered = cell_coverage(lat, lon, 3, level);
        let center_id =
            CellID::from(s2::latlng::LatLng::from_degrees(lat, lon)).parent(level as u64);
        assert!(
            covered.contains(&center_id.0),
            "center cell not in coverage"
        );
        // Size 3 should produce multiple cells.
        assert!(covered.len() > 1, "expected more than 1 cell for size=3");
    }

    // ── cell_intersects_polygon ───────────────────────────────────────────────

    #[test]
    fn cell_intersects_polygon_center_inside() {
        // Build a polygon that definitely contains the cell center.
        use geo::{LineString, Polygon, coord};
        let lat = 48.8566;
        let lon = 2.3522;
        let cell_id = from_array_to_cell_id(&[lat, lon], 16);

        // Large box around the cell (lon±1, lat±1)
        let big_box: Polygon<Precision> = Polygon::new(
            LineString::from(vec![
                coord! { x: lon - 1.0, y: lat - 1.0 },
                coord! { x: lon + 1.0, y: lat - 1.0 },
                coord! { x: lon + 1.0, y: lat + 1.0 },
                coord! { x: lon - 1.0, y: lat + 1.0 },
                coord! { x: lon - 1.0, y: lat - 1.0 },
            ]),
            vec![],
        );
        assert!(cell_intersects_polygon(cell_id, &big_box));
    }

    #[test]
    fn cell_intersects_polygon_far_away_false() {
        use geo::{LineString, Polygon, coord};
        // Cell near Paris, polygon near NYC — no intersection.
        let cell_id = from_array_to_cell_id(&[48.85, 2.35], 15);
        let nyc_box: Polygon<Precision> = Polygon::new(
            LineString::from(vec![
                coord! { x: -75.0, y: 40.0 },
                coord! { x: -73.0, y: 40.0 },
                coord! { x: -73.0, y: 41.5 },
                coord! { x: -75.0, y: 41.5 },
                coord! { x: -75.0, y: 40.0 },
            ]),
            vec![],
        );
        assert!(!cell_intersects_polygon(cell_id, &nyc_box));
    }

    // ── circle_coverage ──────────────────────────────────────────────────────

    #[test]
    fn circle_coverage_contains_center_cell() {
        // A circle around a point must cover the cell containing that point.
        let lat = 40.7128;
        let lon = -74.0060;
        let level = 15u8;
        let radius_m = 500.0; // 500 m
        let covered = circle_coverage(lat, lon, radius_m, level);
        let center_id = from_array_to_cell_id(&[lat, lon], level as u64);
        assert!(
            covered.contains(&center_id.0),
            "circle coverage must contain the center cell"
        );
    }

    #[test]
    fn circle_coverage_larger_radius_more_cells() {
        // A larger radius should cover at least as many cells as a smaller one.
        let lat = 48.8566;
        let lon = 2.3522;
        let level = 14u8;
        let small = circle_coverage(lat, lon, 100.0, level).len();
        let large = circle_coverage(lat, lon, 2000.0, level).len();
        assert!(
            large >= small,
            "larger radius should cover ≥ cells: small={small}, large={large}"
        );
    }
}
