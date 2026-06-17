use geo::{Contains, MultiPolygon, Point, Polygon};
use koji_core::Precision;
use geojson::{FeatureCollection, Value};

use koji_core::HasLatLon;

use crate::rows::{GenericData, LatLonRow, Spawnpoint};

pub(crate) struct AreaPolygons {
    polys: Vec<Polygon<Precision>>,
    multi_polys: Vec<MultiPolygon<Precision>>,
}

impl AreaPolygons {
    pub fn from_collection(area: &FeatureCollection) -> Self {
        let mut polys = Vec::new();
        let mut multi_polys = Vec::new();

        for feature in &area.features {
            if let Some(geometry) = &feature.geometry {
                match &geometry.value {
                    Value::Polygon(_) => match Polygon::try_from(geometry) {
                        Ok(poly) => polys.push(poly),
                        Err(e) => log::warn!("Failed to convert Polygon: {}", e),
                    },
                    Value::MultiPolygon(_) => match MultiPolygon::try_from(geometry) {
                        Ok(mp) => multi_polys.push(mp),
                        Err(e) => log::warn!("Failed to convert MultiPolygon: {}", e),
                    },
                    _ => {}
                }
            }
        }

        Self { polys, multi_polys }
    }

    pub fn contains(&self, lat: Precision, lon: Precision) -> bool {
        let point = Point::new(lon, lat);
        self.polys.iter().any(|poly| poly.contains(&point))
            || self.multi_polys.iter().any(|mp| mp.contains(&point))
    }
}

pub fn count_in_area<T: HasLatLon>(items: &[T], area: &FeatureCollection) -> i32 {
    let polygons = AreaPolygons::from_collection(area);
    items
        .iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .count() as i32
}

impl HasLatLon for Spawnpoint {
    fn lat(&self) -> Precision {
        self.lat
    }
    fn lon(&self) -> Precision {
        self.lon
    }
}

impl HasLatLon for LatLonRow {
    fn lat(&self) -> Precision {
        self.lat
    }
    fn lon(&self) -> Precision {
        self.lon
    }
}

/// Shared filter→enumerate→format→collect pipeline behind the four public
/// normalize fns. `prefix` yields each item's id prefix (a constant for forts,
/// despawn-conditional for spawnpoints); `area`, when present, gates items by
/// polygon containment. Enumeration is post-filter, matching the original fns.
fn to_generic_data<T: HasLatLon>(
    items: Vec<T>,
    area: Option<&FeatureCollection>,
    prefix: impl Fn(&T) -> &'static str,
) -> Vec<GenericData> {
    let polys = area.map(AreaPolygons::from_collection);
    items
        .into_iter()
        .filter(|it| {
            polys
                .as_ref()
                .is_none_or(|p| p.contains(it.lat(), it.lon()))
        })
        .enumerate()
        .map(|(i, it)| GenericData::new(format!("{}{}", prefix(&it), i), it.lat(), it.lon()))
        .collect()
}

pub fn fort(items: Vec<LatLonRow>, prefix: &'static str) -> Vec<GenericData> {
    to_generic_data(items, None, |_| prefix)
}

pub fn fort_filtered(
    items: Vec<LatLonRow>,
    area: &FeatureCollection,
    prefix: &'static str,
) -> Vec<GenericData> {
    to_generic_data(items, Some(area), |_| prefix)
}

pub fn spawnpoint(items: Vec<Spawnpoint>) -> Vec<GenericData> {
    to_generic_data(items, None, |it| {
        if it.despawn_sec.is_some() { "v" } else { "u" }
    })
}

pub fn spawnpoint_filtered(items: Vec<Spawnpoint>, area: &FeatureCollection) -> Vec<GenericData> {
    to_generic_data(items, Some(area), |it| {
        if it.despawn_sec.is_some() { "v" } else { "u" }
    })
}

#[cfg(test)]
mod tests {
    use geojson::FeatureCollection;

    use super::*;
    use crate::rows::{LatLonRow, Spawnpoint};

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a GeoJSON FeatureCollection from a JSON literal (panics on bad JSON).
    fn fc(json: serde_json::Value) -> FeatureCollection {
        serde_json::from_value(json).unwrap()
    }

    /// A simple square polygon (lon/lat order per GeoJSON spec):
    /// SW=(0,0) NE=(10,10) in lon/lat, so lat ∈ [0,10], lon ∈ [0,10].
    fn square_fc() -> FeatureCollection {
        fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [0.0, 0.0],
                        [10.0, 0.0],
                        [10.0, 10.0],
                        [0.0, 10.0],
                        [0.0, 0.0]
                    ]]
                },
                "properties": null
            }]
        }))
    }

    /// Two disjoint squares via MultiPolygon:
    ///   Box A: lon/lat [0,0]..[5,5]
    ///   Box B: lon/lat [20,20]..[25,25]
    fn multi_polygon_fc() -> FeatureCollection {
        fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "MultiPolygon",
                    "coordinates": [
                        [[[0.0, 0.0],[5.0, 0.0],[5.0, 5.0],[0.0, 5.0],[0.0, 0.0]]],
                        [[[20.0, 20.0],[25.0, 20.0],[25.0, 25.0],[20.0, 25.0],[20.0, 20.0]]]
                    ]
                },
                "properties": null
            }]
        }))
    }

    // ── fort ─────────────────────────────────────────────────────────────────

    #[test]
    fn fort_prefixes_and_enumerates() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 2.0 },
            LatLonRow { lat: 3.0, lon: 4.0 },
        ];
        let out = fort(items, "g");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].i, "g0");
        assert_eq!(out[1].i, "g1");
        // p is [lat, lon] — verify coordinates carry through unchanged.
        assert_eq!(out[0].p, [1.0, 2.0]);
        assert_eq!(out[1].p, [3.0, 4.0]);
    }

    #[test]
    fn fort_empty_input_returns_empty() {
        assert_eq!(fort(vec![], "g").len(), 0);
    }

    #[test]
    fn fort_custom_prefix_is_used() {
        let items = vec![LatLonRow { lat: 0.0, lon: 0.0 }];
        let out = fort(items, "pokestop_");
        assert_eq!(out[0].i, "pokestop_0");
    }

    // ── fort_filtered ────────────────────────────────────────────────────────

    #[test]
    fn fort_filtered_keeps_items_inside_polygon() {
        // inside: lat=5, lon=5 — well within [0,10]x[0,10]
        // outside: lat=50, lon=50
        let items = vec![
            LatLonRow { lat: 5.0, lon: 5.0 },
            LatLonRow {
                lat: 50.0,
                lon: 50.0,
            },
        ];
        let out = fort_filtered(items, &square_fc(), "g");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].p, [5.0, 5.0]);
        // index is post-filter: the one surviving item gets index 0
        assert_eq!(out[0].i, "g0");
    }

    #[test]
    fn fort_filtered_all_outside_returns_empty() {
        let items = vec![
            LatLonRow {
                lat: 100.0,
                lon: 100.0,
            },
            LatLonRow {
                lat: -50.0,
                lon: -50.0,
            },
        ];
        assert_eq!(fort_filtered(items, &square_fc(), "g").len(), 0);
    }

    #[test]
    fn fort_filtered_all_inside_returns_all() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 1.0 },
            LatLonRow { lat: 2.0, lon: 2.0 },
            LatLonRow { lat: 9.0, lon: 9.0 },
        ];
        let out = fort_filtered(items, &square_fc(), "p");
        assert_eq!(out.len(), 3);
        assert_eq!(out[2].i, "p2");
    }

    #[test]
    fn fort_filtered_empty_input_returns_empty() {
        assert_eq!(fort_filtered(vec![], &square_fc(), "g").len(), 0);
    }

    #[test]
    fn fort_filtered_index_is_post_filter() {
        // 3 items; item at index 1 (lat=50) is outside.
        // After filtering: [lat=1, lat=9] — indices must be 0 and 1, not 0 and 2.
        let items = vec![
            LatLonRow { lat: 1.0, lon: 1.0 },
            LatLonRow {
                lat: 50.0,
                lon: 50.0,
            },
            LatLonRow { lat: 9.0, lon: 9.0 },
        ];
        let out = fort_filtered(items, &square_fc(), "x");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].i, "x0");
        assert_eq!(out[1].i, "x1");
    }

    // ── spawnpoint ───────────────────────────────────────────────────────────

    #[test]
    fn spawnpoint_prefix_depends_on_despawn_sec() {
        let items = vec![
            Spawnpoint {
                lat: 1.0,
                lon: 2.0,
                despawn_sec: Some(30),
            },
            Spawnpoint {
                lat: 3.0,
                lon: 4.0,
                despawn_sec: None,
            },
        ];
        let out = spawnpoint(items);
        assert_eq!(out.len(), 2);
        // verified (Some) → "v", unverified (None) → "u"; index is post-filter.
        assert_eq!(out[0].i, "v0");
        assert_eq!(out[1].i, "u1");
    }

    #[test]
    fn spawnpoint_all_verified() {
        let items = vec![
            Spawnpoint {
                lat: 0.0,
                lon: 0.0,
                despawn_sec: Some(1800),
            },
            Spawnpoint {
                lat: 1.0,
                lon: 1.0,
                despawn_sec: Some(900),
            },
        ];
        let out = spawnpoint(items);
        assert!(out.iter().all(|g| g.i.starts_with('v')));
    }

    #[test]
    fn spawnpoint_all_unverified() {
        let items = vec![
            Spawnpoint {
                lat: 0.0,
                lon: 0.0,
                despawn_sec: None,
            },
            Spawnpoint {
                lat: 1.0,
                lon: 1.0,
                despawn_sec: None,
            },
        ];
        let out = spawnpoint(items);
        assert!(out.iter().all(|g| g.i.starts_with('u')));
    }

    #[test]
    fn spawnpoint_empty_returns_empty() {
        assert_eq!(spawnpoint(vec![]).len(), 0);
    }

    // ── spawnpoint_filtered ──────────────────────────────────────────────────

    #[test]
    fn spawnpoint_filtered_filters_by_area() {
        let items = vec![
            Spawnpoint {
                lat: 5.0,
                lon: 5.0,
                despawn_sec: Some(1800),
            }, // inside
            Spawnpoint {
                lat: 50.0,
                lon: 50.0,
                despawn_sec: None,
            }, // outside
        ];
        let out = spawnpoint_filtered(items, &square_fc());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].i, "v0");
        assert_eq!(out[0].p, [5.0, 5.0]);
    }

    #[test]
    fn spawnpoint_filtered_preserves_prefix_after_filter() {
        // unverified inside, verified outside — only the unverified survives
        let items = vec![
            Spawnpoint {
                lat: 5.0,
                lon: 5.0,
                despawn_sec: None,
            },
            Spawnpoint {
                lat: 50.0,
                lon: 50.0,
                despawn_sec: Some(1800),
            },
        ];
        let out = spawnpoint_filtered(items, &square_fc());
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].i, "u0");
    }

    // ── AreaPolygons ─────────────────────────────────────────────────────────

    #[test]
    fn area_none_keeps_all_items() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 2.0 },
            LatLonRow { lat: 3.0, lon: 4.0 },
        ];
        // area=None path (via fort) must retain every item — no polygon gate.
        assert_eq!(to_generic_data(items, None, |_| "g").len(), 2);
    }

    #[test]
    fn area_polygons_contains_inside_point() {
        let ap = AreaPolygons::from_collection(&square_fc());
        assert!(ap.contains(5.0, 5.0));
    }

    #[test]
    fn area_polygons_excludes_outside_point() {
        let ap = AreaPolygons::from_collection(&square_fc());
        assert!(!ap.contains(50.0, 50.0));
    }

    #[test]
    fn area_polygons_multipolygon_contains_both_sub_boxes() {
        let ap = AreaPolygons::from_collection(&multi_polygon_fc());
        // inside box A
        assert!(ap.contains(2.0, 2.0));
        // inside box B
        assert!(ap.contains(22.0, 22.0));
        // between the two boxes — outside
        assert!(!ap.contains(12.0, 12.0));
    }

    #[test]
    fn area_polygons_empty_collection_contains_nothing() {
        let empty = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": []
        }));
        let ap = AreaPolygons::from_collection(&empty);
        assert!(!ap.contains(0.0, 0.0));
    }

    #[test]
    fn area_polygons_skips_non_polygon_geometries() {
        // A LineString feature — should not panic, just produce zero polygons.
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "LineString",
                    "coordinates": [[0.0,0.0],[1.0,1.0]]
                },
                "properties": null
            }]
        }));
        let ap = AreaPolygons::from_collection(&area);
        assert!(!ap.contains(0.5, 0.5));
    }

    #[test]
    fn area_polygons_feature_with_no_geometry_is_ignored() {
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": null,
                "properties": null
            }]
        }));
        // Must not panic; nothing to contain anything.
        let ap = AreaPolygons::from_collection(&area);
        assert!(!ap.contains(0.0, 0.0));
    }

    // ── count_in_area ─────────────────────────────────────────────────────────

    #[test]
    fn count_in_area_counts_only_inside_items() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 1.0 }, // inside
            LatLonRow { lat: 5.0, lon: 5.0 }, // inside
            LatLonRow {
                lat: 50.0,
                lon: 50.0,
            }, // outside
        ];
        assert_eq!(count_in_area(&items, &square_fc()), 2);
    }

    #[test]
    fn count_in_area_returns_zero_for_all_outside() {
        let items = vec![
            LatLonRow {
                lat: 100.0,
                lon: 100.0,
            },
            LatLonRow {
                lat: -50.0,
                lon: -50.0,
            },
        ];
        assert_eq!(count_in_area(&items, &square_fc()), 0);
    }

    #[test]
    fn count_in_area_empty_items_returns_zero() {
        let items: Vec<LatLonRow> = vec![];
        assert_eq!(count_in_area(&items, &square_fc()), 0);
    }

    #[test]
    fn count_in_area_returns_all_when_all_inside() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 1.0 },
            LatLonRow { lat: 2.0, lon: 2.0 },
            LatLonRow { lat: 9.0, lon: 9.0 },
        ];
        assert_eq!(count_in_area(&items, &square_fc()), 3);
    }

    #[test]
    fn count_in_area_uses_spawnpoint_haslatlon() {
        let items = vec![
            Spawnpoint {
                lat: 3.0,
                lon: 3.0,
                despawn_sec: None,
            }, // inside
            Spawnpoint {
                lat: 99.0,
                lon: 99.0,
                despawn_sec: Some(1800),
            }, // outside
        ];
        assert_eq!(count_in_area(&items, &square_fc()), 1);
    }
}
