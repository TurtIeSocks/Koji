use std::fmt::Write;
#[cfg(test)]
use koji_core::Precision;

use geojson::{Feature, FeatureCollection, Value};

use koji_core::{KojiBbox, KojiGeometry, KojiGeometryCollection, SingleVec};

/// The `[lat, lon]` coordinate list of one geojson `Feature`, via the Koji-native
/// path (`KojiGeometry::try_from` → the inherent `to_single_vec`). A feature with
/// no/unconvertible geometry yields an empty vec. Bbox is only consumed for
/// `Polygon`/`MultiPolygon` in `sql_raw_bbox`, where `single_group` reproduces the
/// flattening exactly.
fn feature_single_vec(feature: &Feature) -> SingleVec {
    match KojiGeometry::try_from(feature.clone()) {
        Ok(kg) => KojiGeometryCollection::new(vec![kg]).to_single_vec(),
        Err(_) => vec![],
    }
}

pub fn sql_raw_bbox(area: &FeatureCollection) -> String {
    let mut string = String::new();
    for (i, feature) in area.into_iter().enumerate() {
        let bbox = if let Some(bbox) = feature.bbox.as_ref() {
            bbox.clone()
        } else if let Some(bbox) = KojiBbox::from_points(&feature_single_vec(feature))
            .map(|b| b.trim(6).to_geojson_bbox_vec())
        {
            bbox
        } else {
            continue;
        };
        if let Some(geometry) = &feature.geometry {
            match geometry.value {
                Value::Polygon(_) | Value::MultiPolygon(_) => {
                    let _ = write!(
                        string,
                        "{} (lon BETWEEN {} AND {} AND lat BETWEEN {} AND {})",
                        if i == 0 { "" } else { "\nOR" },
                        bbox[0],
                        bbox[2],
                        bbox[1],
                        bbox[3]
                    );
                }
                _ => {}
            }
        }
    }
    string
}

#[cfg(test)]
mod tests {
    use geojson::FeatureCollection;

    use super::*;

    fn fc(json: serde_json::Value) -> FeatureCollection {
        serde_json::from_value(json).unwrap()
    }

    /// A GeoJSON Polygon whose bbox covers lon∈[0,10] lat∈[0,10].
    /// GeoJSON coord order: [lon, lat].
    fn polygon_fc() -> FeatureCollection {
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

    // ── basic output shape ─────────────────────────────────────────────────────

    #[test]
    fn empty_collection_returns_empty_string() {
        let area = fc(serde_json::json!({ "type": "FeatureCollection", "features": [] }));
        assert_eq!(sql_raw_bbox(&area), "");
    }

    #[test]
    fn polygon_produces_between_clause() {
        let sql = sql_raw_bbox(&polygon_fc());
        // Must contain the four BETWEEN keywords for the two axes.
        assert!(sql.contains("lon BETWEEN"), "missing lon BETWEEN in: {sql}");
        assert!(sql.contains("lat BETWEEN"), "missing lat BETWEEN in: {sql}");
        // No leading OR for the first (and only) feature.
        assert!(!sql.starts_with("\nOR"), "unexpected leading OR: {sql}");
    }

    // ── computed bbox from polygon coords ─────────────────────────────────────

    #[test]
    fn polygon_bbox_values_are_correct() {
        // Polygon: lon∈[0,10], lat∈[0,10].
        // GeoJSON bbox order: [min_lon, min_lat, max_lon, max_lat].
        // SQL template: lon BETWEEN bbox[0] AND bbox[2]  (= min_lon AND max_lon)
        //               lat BETWEEN bbox[1] AND bbox[3]  (= min_lat AND max_lat)
        let sql = sql_raw_bbox(&polygon_fc());
        assert!(
            sql.contains("lon BETWEEN 0 AND 10"),
            "lon bounds wrong: {sql}"
        );
        assert!(
            sql.contains("lat BETWEEN 0 AND 10"),
            "lat bounds wrong: {sql}"
        );
    }

    // ── explicit bbox on feature overrides computed ────────────────────────────

    #[test]
    fn explicit_feature_bbox_is_used_verbatim() {
        // Provide an explicit bbox [min_lon=1, min_lat=2, max_lon=3, max_lat=4]
        // so the function must skip computing it from coordinates.
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "bbox": [1.0, 2.0, 3.0, 4.0],
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [0.0, 0.0],[10.0, 0.0],[10.0, 10.0],[0.0, 10.0],[0.0, 0.0]
                    ]]
                },
                "properties": null
            }]
        }));
        let sql = sql_raw_bbox(&area);
        // bbox[0]=1, bbox[2]=3 → lon BETWEEN 1 AND 3
        // bbox[1]=2, bbox[3]=4 → lat BETWEEN 2 AND 4
        assert!(
            sql.contains("lon BETWEEN 1 AND 3"),
            "explicit lon wrong: {sql}"
        );
        assert!(
            sql.contains("lat BETWEEN 2 AND 4"),
            "explicit lat wrong: {sql}"
        );
    }

    // ── non-polygon geometry skipped ──────────────────────────────────────────

    #[test]
    fn linestring_geometry_produces_no_clause() {
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
        // LineString has no polygon bbox — sql_raw_bbox must skip it silently.
        assert_eq!(sql_raw_bbox(&area), "");
    }

    #[test]
    fn point_geometry_produces_no_clause() {
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
                "properties": null
            }]
        }));
        assert_eq!(sql_raw_bbox(&area), "");
    }

    // ── feature with null geometry ────────────────────────────────────────────

    #[test]
    fn null_geometry_produces_no_clause() {
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": null,
                "properties": null
            }]
        }));
        assert_eq!(sql_raw_bbox(&area), "");
    }

    // ── multiple polygon features → OR separator ──────────────────────────────

    #[test]
    fn two_polygons_joined_with_or() {
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[0.0,0.0],[5.0,0.0],[5.0,5.0],[0.0,5.0],[0.0,0.0]]]
                    },
                    "properties": null
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [[[20.0,20.0],[25.0,20.0],[25.0,25.0],[20.0,25.0],[20.0,20.0]]]
                    },
                    "properties": null
                }
            ]
        }));
        let sql = sql_raw_bbox(&area);
        // Second feature must be joined with "\nOR"
        assert!(sql.contains("\nOR"), "missing OR separator: {sql}");
        // Both BETWEEN clauses present
        assert_eq!(
            sql.matches("lon BETWEEN").count(),
            2,
            "expected 2 lon clauses: {sql}"
        );
        assert_eq!(
            sql.matches("lat BETWEEN").count(),
            2,
            "expected 2 lat clauses: {sql}"
        );
    }

    #[test]
    fn first_clause_has_no_leading_or() {
        let sql = sql_raw_bbox(&polygon_fc());
        // The very first character must not be 'O' (from OR), nor '\n'.
        let first = sql.chars().next().unwrap_or(' ');
        assert_ne!(first, '\n', "first clause has leading newline: {sql}");
        assert!(sql.starts_with(" (lon"), "expected leading space+(: {sql}");
    }

    // ── multipolygon geometry ─────────────────────────────────────────────────

    #[test]
    fn multipolygon_produces_between_clause() {
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "MultiPolygon",
                    "coordinates": [
                        [[[0.0,0.0],[5.0,0.0],[5.0,5.0],[0.0,5.0],[0.0,0.0]]],
                        [[[10.0,10.0],[15.0,10.0],[15.0,15.0],[10.0,15.0],[10.0,10.0]]]
                    ]
                },
                "properties": null
            }]
        }));
        let sql = sql_raw_bbox(&area);
        assert!(
            sql.contains("lon BETWEEN"),
            "multipolygon missing lon: {sql}"
        );
        assert!(
            sql.contains("lat BETWEEN"),
            "multipolygon missing lat: {sql}"
        );
    }

    // ── precision trimming ─────────────────────────────────────────────────────

    #[test]
    fn computed_bbox_trimmed_to_six_decimals() {
        // Polygon with irrational-ish coordinates to force trimming.
        let area = fc(serde_json::json!({
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[
                        [1.123456789, 2.987654321],
                        [3.111111111, 2.987654321],
                        [3.111111111, 4.999999999],
                        [1.123456789, 4.999999999],
                        [1.123456789, 2.987654321]
                    ]]
                },
                "properties": null
            }]
        }));
        let sql = sql_raw_bbox(&area);
        // All numbers in the SQL string should have at most 6 decimal places.
        // Extract all decimal-containing numbers and check.
        for token in sql.split_whitespace() {
            if let Ok(n) = token.trim_end_matches(')').parse::<Precision>() {
                let s = format!("{n}");
                if let Some(dot) = s.find('.') {
                    let decimals = s.len() - dot - 1;
                    assert!(
                        decimals <= 6,
                        "value {n} has more than 6 decimal places in: {sql}"
                    );
                }
            }
        }
    }
}
