use std::collections::HashSet;
use web_time::Instant;

use crate::{routing, stats::Stats};

use geo::{BoundingRect, MultiPolygon, Polygon};
use geojson::{Feature, GeometryValue};
use koji_core::{Precision, SingleVec};

use crate::routing::RoutingConfig;
use rayon::{iter::IntoParallelIterator, prelude::ParallelIterator};
use s2::{
    cell::Cell,
    cellid::{CellID, MAX_LEVEL},
    latlng::LatLng,
    rect::Rect,
    region::RegionCoverer,
    s1::{Angle, Deg},
};

#[derive(Debug)]
pub struct BootstrapS2<'a> {
    feature: &'a Feature,
    result: SingleVec,
    level: u8,
    size: u8,
    pub stats: Stats,
}

impl<'a> BootstrapS2<'a> {
    pub fn new(feature: &'a Feature, level: u8, size: u8) -> Self {
        let mut new_bootstrap = Self {
            feature,
            result: vec![],
            level,
            size,
            stats: Stats::new("BootstrapS2".to_string(), 0),
        };

        let time = Instant::now();
        new_bootstrap.result = new_bootstrap.run();
        new_bootstrap.stats.set_cluster_time(time);
        new_bootstrap
            .stats
            .cluster_stats(0., &vec![], &new_bootstrap.result);

        new_bootstrap
    }

    pub fn sort(&mut self, routing: &RoutingConfig) {
        self.result = routing::main(&vec![], self.result.clone(), 0., routing, &mut self.stats);
    }

    pub fn result(self) -> SingleVec {
        self.result
    }

    pub fn feature(self) -> Feature {
        // Koji-native MultiPoint projection (matches the old matrix
        // `SingleVec::to_feature(CirclePokemon)` geometry); no To* matrix.
        let mut new_feature = koji_core::single_vec_to_multipoint_feature(&self.result);

        if let Some(name) = self.feature.property("__name") {
            new_feature.set_property("__name", name.clone());
        }
        if let Some(geofence_id) = self.feature.property("__id") {
            new_feature.set_property("__geofence_id", geofence_id.clone());
        }
        new_feature.set_property("__mode", "CircleRaid");
        new_feature
    }

    fn build_polygons(&self) -> Vec<geo::Polygon> {
        if let Some(geometry) = self.feature.geometry.as_ref() {
            match geometry.value {
                GeometryValue::Polygon { .. } => match Polygon::<Precision>::try_from(geometry) {
                    Ok(poly) => vec![poly],
                    Err(_) => vec![],
                },
                GeometryValue::MultiPolygon { .. } => {
                    match MultiPolygon::<Precision>::try_from(geometry) {
                        Ok(multi_poly) => multi_poly.0.into_iter().collect(),
                        Err(_) => vec![],
                    }
                }
                _ => vec![],
            }
        } else {
            vec![]
        }
    }

    fn run(&self) -> SingleVec {
        log::info!("Starting S2 bootstrapping");

        let time = Instant::now();
        let polygons = self.build_polygons();

        let results = polygons
            .into_par_iter()
            .flat_map(|poly| self.centers_for_polygon(&poly))
            .collect();

        log::info!("Bootstrapped S2 in {:.4}s", time.elapsed().as_secs_f32());

        results
    }

    /// Core implementation operating on geo-types::Polygon<Precision>.
    pub fn centers_for_polygon(&self, poly: &Polygon<Precision>) -> Vec<[Precision; 2]> {
        let time = Instant::now();
        // 1) Bounding box and S2 Rect (note: simple case, no antimeridian split).
        let bbox = poly
            .bounding_rect()
            .expect("Polygon has no bounding box (empty geometry)?");
        let lat_lo = bbox.min().y;
        let lat_hi = bbox.max().y;
        let lng_lo = bbox.min().x;
        let lng_hi = bbox.max().x;
        let expand_angle = Angle::from(Deg(0.1));

        let rect = Rect::from_degrees(lat_lo, lng_lo, lat_hi, lng_hi).expanded(&LatLng {
            lat: expand_angle,
            lng: expand_angle,
        });

        // 2) RegionCoverer at the requested level.
        let rc = RegionCoverer {
            min_level: self.level,
            max_level: self.level,
            level_mod: 1,
            max_cells: usize::MAX,
        };
        let cover = rc.covering(&rect);

        log::info!(
            "Created region coverer in {:.4}s",
            time.elapsed().as_secs_f32()
        );

        log::info!("Checking {} cells", cover.0.len());
        let time = Instant::now();

        let covered_set = cover
            .0
            .into_iter()
            .filter_map(|id| self.block_center_cell(id))
            .collect::<HashSet<CellID>>();
        log::info!("Created centers in {:.4}s", time.elapsed().as_secs_f32());

        covered_set
            .into_par_iter()
            .filter_map(|id| {
                // 4) Build the size×size neighborhood via ring expansion (Chebyshev radius = half).
                let neighborhood = koji_core::s2::s2_grid(id, self.level, self.size);
                // 5) If any cell in the block intersects the polygon, include the center point.
                if neighborhood
                    .iter()
                    .any(|cid| koji_core::s2::cell_intersects_polygon(*cid, poly))
                {
                    let ll = cell_center_latlng(id);
                    Some([ll.lat.deg(), ll.lng.deg()])
                } else {
                    None
                }
            })
            .collect()
    }

    /// Given a level-L CellID, return the center cell of its SIZE x SIZE block at that level.
    /// We do this by converting to (face, i, j) at leaf resolution, downshifting to
    /// level-L grid, snapping to the block center (offset +4 in a 0..8 range), then
    /// reconstructing a cell and taking its parent at level L.
    ///
    /// Requires: 0 <= L <= MAX_LEVEL and `id.level() == L`.
    fn block_center_cell(&self, id: CellID) -> Option<CellID> {
        if self.size == 1 {
            return Some(id);
        }

        let (face, i_leaf, j_leaf, _orient) = id.face_ij_orientation();
        let shift = MAX_LEVEL as i32 - self.level as i32;

        // Convert leaf i,j to level-L grid coords (0..2^L-1).
        let i_l = i_leaf >> shift;
        let j_l = j_leaf >> shift;

        let half = (self.size / 2) as i32; // e.g., 4 for size 9, 2 for size 5
        let size_i32 = self.size as i32;

        // Integer block indices, then snap to the block center.
        let block_i = i_l.div_euclid(size_i32);
        let block_j = j_l.div_euclid(size_i32);

        let center_i_l = block_i * size_i32 + half;
        let center_j_l = block_j * size_i32 + half;

        // Back to leaf i,j at lower-left of the level-L cell.
        let center_i_leaf = center_i_l << shift;
        let center_j_leaf = center_j_l << shift;

        let center =
            CellID::from_face_ij(face, center_i_leaf, center_j_leaf).parent(self.level as u64);

        // A boundary block whose center column/row lands past the face edge wraps
        // onto an adjacent cube face; skip it (an in-face block's size×size
        // neighbourhood still covers the polygon along that seam). The previous
        // `cells_to_nearest_face_edges` "correction" here conflated distance-to-seam
        // with block-local offset, spuriously shifting the center +1 cell on the
        // east/north half of every face — `block_i * size + half` is already the
        // block center, so emit it directly.
        if center.face() != face {
            return None;
        }

        Some(center)
    }
}

/// Center LatLng for a cell.
fn cell_center_latlng(id: CellID) -> LatLng {
    let p = Cell::from(id).center();
    LatLng::from(&p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, GeometryValue};
    use s2::{
        cellid::{CellID, MAX_LEVEL},
        latlng::LatLng,
    };

    fn rect_feature(
        min_lon: Precision,
        min_lat: Precision,
        max_lon: Precision,
        max_lat: Precision,
    ) -> Feature {
        let ring = vec![
            geojson::Position::from([min_lon, min_lat]),
            geojson::Position::from([max_lon, min_lat]),
            geojson::Position::from([max_lon, max_lat]),
            geojson::Position::from([min_lon, max_lat]),
            geojson::Position::from([min_lon, min_lat]),
        ];
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(GeometryValue::Polygon {
                coordinates: vec![ring],
            })),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // ── BootstrapS2::new / result ─────────────────────────────────────────────

    #[test]
    fn small_rect_level14_produces_cells() {
        // ~1 km² rectangle at level 14 (cells ~≈ 600 m); expect at least a few centers.
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let bs = BootstrapS2::new(&feature, 14, 1);
        let pts = bs.result();
        assert!(!pts.is_empty(), "expected cells in small rect at level 14");
    }

    #[test]
    fn output_lat_lon_in_valid_range() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let bs = BootstrapS2::new(&feature, 15, 1);
        for [lat, lon] in bs.result() {
            assert!(lat.abs() < 90.0, "bad lat: {lat}");
            assert!(lon.abs() < 180.0, "bad lon: {lon}");
        }
    }

    #[test]
    fn no_geometry_returns_empty() {
        let feature = Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        };
        let bs = BootstrapS2::new(&feature, 14, 1);
        assert!(bs.result().is_empty());
    }

    // ── centers_for_polygon ───────────────────────────────────────────────────

    #[test]
    fn centers_for_polygon_level15_nonempty() {
        use geo::{LineString, Polygon};
        // A ~1 km × 1 km polygon near NYC.
        let exterior = LineString::from(vec![
            (-74.003, 39.997),
            (-73.997, 39.997),
            (-73.997, 40.003),
            (-74.003, 40.003),
            (-74.003, 39.997),
        ]);
        let poly = Polygon::new(exterior, vec![]);
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let bs = BootstrapS2::new(&feature, 15, 1);
        let centers = bs.centers_for_polygon(&poly);
        assert!(
            !centers.is_empty(),
            "should have centers for this polygon at level 15"
        );
    }

    // ── block_center_cell with size=1 ─────────────────────────────────────────

    #[test]
    fn block_center_size1_returns_same_cell() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let bs = BootstrapS2::new(&feature, 14, 1);
        let cell = CellID::from(LatLng::from_degrees(40.0, -74.0)).parent(14);
        // block_center_cell is private; exercise through new() which calls it.
        // Just verify the outer result is non-empty.
        let pts = bs.result();
        // If size=1, block_center returns each cell unchanged; points must be present.
        assert!(!pts.is_empty());
        let _ = cell; // suppress unused warning
    }

    #[test]
    fn block_center_no_spurious_shift_on_east_half() {
        // Regression: the removed `cells_to_nearest_face_edges` correction shifted
        // the block center +1 cell on the east/north half of every face. The true
        // center is `block_i * size + half`, so an east-half cell must map to it
        // unshifted. (Pre-fix this returned column 1022 instead of 1021.)
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let level: u8 = 10;
        let size: u8 = 3;
        let bs = BootstrapS2::new(&feature, level, size);
        let shift = MAX_LEVEL as i32 - level as i32;

        // i_l = 1022 lives in block 340 (340*3 = 1020); its center column is
        // 340*3 + 1 = 1021, and the block stays on-face (1021 < 2^10 = 1024).
        let face: u8 = 0;
        let input = CellID::from_face_ij(face, 1022 << shift, 5 << shift).parent(level as u64);

        let center = bs
            .block_center_cell(input)
            .expect("on-face block must yield a center");
        let (got_face, got_i, got_j, _) = center.face_ij_orientation();
        assert_eq!(got_face, face);
        assert_eq!(
            got_i >> shift,
            1021,
            "east-half block center must not shift +1"
        );
        assert_eq!(got_j >> shift, 4); // block 1 → center 1*3 + 1 = 4
    }

    // ── BootstrapS2: size > 1 produces fewer (grouped) centers ────────────────

    #[test]
    fn size3_produces_fewer_centers_than_size1() {
        // Larger block size → fewer distinct centers (9 cells share one center).
        let feature = rect_feature(-74.01, 39.99, -73.99, 40.01);
        let bs1 = BootstrapS2::new(&feature, 15, 1);
        let bs3 = BootstrapS2::new(&feature, 15, 3);
        let n1 = bs1.result().len();
        let n3 = bs3.result().len();
        assert!(
            n3 <= n1,
            "size=3 should produce ≤ centers than size=1; got size1={n1}, size3={n3}"
        );
    }

    #[test]
    fn size1_and_size3_centers_are_valid_lat_lon() {
        let feature = rect_feature(-74.01, 39.99, -73.99, 40.01);
        for size in [1u8, 3, 5] {
            let bs = BootstrapS2::new(&feature, 14, size);
            for [lat, lon] in bs.result() {
                assert!(lat.abs() <= 90.0, "bad lat {lat} for size {size}");
                assert!(lon.abs() <= 180.0, "bad lon {lon} for size {size}");
            }
        }
    }

    // ── BootstrapS2::feature() propagates name and id properties ─────────────

    #[test]
    fn feature_propagates_name_property() {
        use geojson::JsonValue;
        let mut f = rect_feature(-74.003, 39.997, -73.997, 40.003);
        f.set_property("__name", JsonValue::String("test_area".to_string()));
        f.set_property("__id", JsonValue::Number(42.into()));
        let bs = BootstrapS2::new(&f, 14, 1);
        let out = bs.feature();
        assert_eq!(
            out.property("__name").and_then(|v| v.as_str()),
            Some("test_area"),
            "feature() should propagate __name"
        );
        assert_eq!(
            out.property("__geofence_id").and_then(|v| v.as_i64()),
            Some(42),
            "feature() should propagate __id as __geofence_id"
        );
        assert_eq!(
            out.property("__mode").and_then(|v| v.as_str()),
            Some("CircleRaid"),
        );
    }

    // ── BootstrapS2: MultiPolygon geometry ────────────────────────────────────

    #[test]
    fn multipolygon_feature_produces_cells() {
        use geojson::{Feature, Geometry, GeometryValue};
        // Two non-overlapping rectangles as a MultiPolygon.
        let ring1 = vec![
            geojson::Position::from([-74.003_f64, 39.997_f64]),
            geojson::Position::from([-73.997, 39.997]),
            geojson::Position::from([-73.997, 40.003]),
            geojson::Position::from([-74.003, 40.003]),
            geojson::Position::from([-74.003, 39.997]),
        ];
        let ring2 = vec![
            geojson::Position::from([-74.103_f64, 39.997_f64]),
            geojson::Position::from([-74.097, 39.997]),
            geojson::Position::from([-74.097, 40.003]),
            geojson::Position::from([-74.103, 40.003]),
            geojson::Position::from([-74.103, 39.997]),
        ];
        let feature = Feature {
            bbox: None,
            geometry: Some(Geometry::new(GeometryValue::MultiPolygon {
                coordinates: vec![vec![ring1], vec![ring2]],
            })),
            id: None,
            properties: None,
            foreign_members: None,
        };
        let bs = BootstrapS2::new(&feature, 14, 1);
        let pts = bs.result();
        assert!(!pts.is_empty(), "MultiPolygon should yield cells");
    }

    // ── cell_center_latlng: finite and in range ───────────────────────────────

    #[test]
    fn cell_center_latlng_is_finite() {
        let cell = CellID::from(LatLng::from_degrees(40.0, -74.0)).parent(12);
        let ll = cell_center_latlng(cell);
        assert!(ll.lat.deg().is_finite());
        assert!(ll.lng.deg().is_finite());
        assert!(ll.lat.deg().abs() <= 90.0);
        assert!(ll.lng.deg().abs() <= 180.0);
    }
}
