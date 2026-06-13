use actix_web::HttpResponse;
use algorithms::stats::Stats;
use geojson::JsonValue;
use koji_core::{KojiGeometryCollection, Precision};
use serde::Serialize;
use serde_json::json;

use koji_core::GeoFormats;
use koji_core::ReturnTypeArg;

#[derive(Debug, Serialize)]
pub struct ConfigResponse {
    pub start_lat: Precision,
    pub start_lon: Precision,
    pub tile_server: String,
    pub logged_in: bool,
    pub dangerous: bool,
    pub route_plugins: Vec<String>,
    pub clustering_plugins: Vec<String>,
    pub bootstrap_plugins: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: Option<JsonValue>,
    pub stats: Option<Stats>,
}

impl Response {
    pub fn send_error(message: &str) -> Response {
        Response {
            message: message.to_string(),
            status: "error".to_string(),
            status_code: 500,
            data: None,
            stats: None,
        }
    }
}

/// Build the `GeoFormats` response envelope for a `KojiGeometryCollection` and a
/// requested `ReturnTypeArg`, dispatching to the Phase 1B inherent adapters (on
/// `KojiGeometryCollection`) and the Phase 1 geojson `From` edge conversions.
///
/// `GeoFormats` is `#[serde(untagged)]`, so it is purely a serialization-shape
/// envelope here — `json!(GeoFormats::X(v))` serializes identically to `json!(v)`.
/// We never route through the `To*` matrix (its `GeoFormats::to_collection` path
/// is unused; only serde serialization of the inner value runs).
///
/// Factored out of [`send`] so the dispatch mapping is unit-testable against the
/// (still-alive in this section) matrix oracle.
fn response_body(coll: &KojiGeometryCollection, return_type: ReturnTypeArg) -> GeoFormats {
    match return_type {
        ReturnTypeArg::SingleStruct => GeoFormats::SingleStruct(coll.to_single_struct()),
        ReturnTypeArg::MultiStruct => GeoFormats::MultiStruct(coll.to_multi_struct()),
        ReturnTypeArg::Text => GeoFormats::Text(coll.to_text(",", "\n", true)),
        ReturnTypeArg::AltText => GeoFormats::Text(coll.to_text(" ", ",", false)),
        ReturnTypeArg::SingleArray => GeoFormats::SingleArray(coll.to_single_vec()),
        ReturnTypeArg::MultiArray => GeoFormats::MultiArray(coll.to_multi_vec()),
        ReturnTypeArg::Geometry => {
            if coll.items.len() == 1 {
                GeoFormats::Geometry(item_geometry(&coll.items[0]))
            } else {
                log::info!(
                    "\"Geometry\" was requested as the return type but multiple features were found so a Vec of geometries is being returned"
                );
                GeoFormats::GeometryVec(coll.items.iter().map(item_geometry).collect())
            }
        }
        ReturnTypeArg::GeometryVec => {
            GeoFormats::GeometryVec(coll.items.iter().map(item_geometry).collect())
        }
        ReturnTypeArg::Feature => {
            let mut features = geojson::FeatureCollection::from(coll).features;
            if features.len() == 1 {
                GeoFormats::Feature(features.remove(0))
            } else {
                log::info!(
                    "\"Feature\" was requested as the return type but multiple features were found so a Vec of features is being returned"
                );
                GeoFormats::FeatureVec(features)
            }
        }
        ReturnTypeArg::FeatureVec => {
            GeoFormats::FeatureVec(geojson::FeatureCollection::from(coll).features)
        }
        ReturnTypeArg::FeatureCollection => {
            GeoFormats::FeatureCollection(geojson::FeatureCollection::from(coll))
        }
        ReturnTypeArg::Poracle => GeoFormats::Poracle(coll.to_poracle_vec()),
        ReturnTypeArg::PoracleSingle => {
            GeoFormats::PoracleSingle(coll.to_poracle_vec().first().unwrap().clone())
        }
        ReturnTypeArg::Sql => GeoFormats::Text(coll.to_sql()),
    }
}

/// One item's geojson `Geometry` via the Phase 1 edge conversion
/// (`geojson::Value::from(&geo::Geometry)`). Byte-parity with the old matrix
/// `Feature::to_geometry` over `FeatureCollection::from(coll)` — that path also
/// just unwrapped the feature's Phase 1 geometry. Matrix-free.
fn item_geometry(item: &koji_core::KojiGeometry) -> geojson::Geometry {
    geojson::Geometry::new(geojson::Value::from(&item.geometry))
}

pub fn send(
    coll: KojiGeometryCollection,
    return_type: ReturnTypeArg,
    stats: Option<Stats>,
    benchmark_mode: bool,
    area: Option<String>,
) -> HttpResponse {
    if let Some(stats) = stats.as_ref() {
        stats.log(area);
    }
    HttpResponse::Ok().json(Response {
        message: "Success".to_string(),
        status: "ok".to_string(),
        status_code: 200,
        data: if benchmark_mode {
            None
        } else {
            Some(json!(response_body(&coll, return_type)))
        },
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{
        Geometry, GeometryCollection, LineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
    };
    use koji_core::geometry::{
        ToMultiStruct, ToMultiVec, ToPoracleVec, ToSingleStruct, ToSingleVec, ToSql, ToText,
    };
    use koji_core::{KojiGeometry, KojiMeta, Mode};

    /// The pre-swap oracle: the still-alive geojson `FeatureCollection` matrix.
    /// Phase 2 Section 5 deletes it; until then it is an exact reference for the
    /// non-line return types.
    fn oracle(c: &KojiGeometryCollection) -> geojson::FeatureCollection {
        geojson::FeatureCollection::from(c)
    }

    /// The same `GeoFormats` body the *old* `send` would have produced, built by
    /// feeding the matrix oracle through the original match arms. Mirrors the
    /// pre-Task-3.1 `send` body exactly (minus the line-dropping the adapters fix).
    fn oracle_body(c: &KojiGeometryCollection, rt: ReturnTypeArg) -> GeoFormats {
        use koji_core::geometry::ToGeometry;
        let value = oracle(c);
        match rt {
            ReturnTypeArg::SingleStruct => GeoFormats::SingleStruct(value.to_single_struct()),
            ReturnTypeArg::MultiStruct => GeoFormats::MultiStruct(value.to_multi_struct()),
            ReturnTypeArg::Text => GeoFormats::Text(value.to_text(",", "\n", true)),
            ReturnTypeArg::AltText => GeoFormats::Text(value.to_text(" ", ",", false)),
            ReturnTypeArg::SingleArray => GeoFormats::SingleArray(value.to_single_vec()),
            ReturnTypeArg::MultiArray => GeoFormats::MultiArray(value.to_multi_vec()),
            ReturnTypeArg::Geometry => {
                if value.features.len() == 1 {
                    GeoFormats::Geometry(value.features.first().unwrap().to_owned().to_geometry())
                } else {
                    GeoFormats::GeometryVec(
                        value.into_iter().map(|feat| feat.to_geometry()).collect(),
                    )
                }
            }
            ReturnTypeArg::GeometryVec => {
                GeoFormats::GeometryVec(value.into_iter().map(|feat| feat.to_geometry()).collect())
            }
            ReturnTypeArg::Feature => {
                if value.features.len() == 1 {
                    GeoFormats::Feature(value.features.first().unwrap().clone())
                } else {
                    GeoFormats::FeatureVec(value.features)
                }
            }
            ReturnTypeArg::FeatureVec => GeoFormats::FeatureVec(value.features),
            ReturnTypeArg::FeatureCollection => GeoFormats::FeatureCollection(value),
            ReturnTypeArg::Poracle => GeoFormats::Poracle(value.to_poracle_vec()),
            ReturnTypeArg::PoracleSingle => {
                GeoFormats::PoracleSingle(value.to_poracle_vec().first().unwrap().clone())
            }
            ReturnTypeArg::Sql => GeoFormats::Text(value.to_sql()),
        }
    }

    /// Adversarial collection mirroring koji-core's `sample_rich`: asymmetric
    /// polygon with a hole + rich meta, a MultiPolygon, and a GeometryCollection.
    /// Exercises every adapter branch the matrix oracle can also represent.
    /// (Bare LineString is intentionally excluded — see the whitelist note below.)
    fn sample_rich() -> KojiGeometryCollection {
        let poly_with_hole = Polygon::new(
            LineString::from(vec![
                coord! {x:0.0,y:0.0},
                coord! {x:10.0,y:0.0},
                coord! {x:10.0,y:4.0},
                coord! {x:0.0,y:4.0},
                coord! {x:0.0,y:0.0},
            ]),
            vec![LineString::from(vec![
                coord! {x:2.0,y:1.0},
                coord! {x:4.0,y:1.0},
                coord! {x:4.0,y:3.0},
                coord! {x:2.0,y:3.0},
                coord! {x:2.0,y:1.0},
            ])],
        );
        let extra = serde_json::json!({
            "color": "#00ff00",
            "description": "a rich fence",
            "group": "alpha",
            "displayInMatches": false,
            "userSelectable": false,
        })
        .as_object()
        .unwrap()
        .clone();
        let poly_meta = KojiMeta {
            id: Some(7),
            name: Some("poly".into()),
            mode: Mode::Fort,
            extra,
            ..Default::default()
        };

        let multi_poly = MultiPolygon::new(vec![
            Polygon::new(
                LineString::from(vec![
                    coord! {x:20.0,y:20.0},
                    coord! {x:23.0,y:20.0},
                    coord! {x:23.0,y:22.0},
                    coord! {x:20.0,y:20.0},
                ]),
                vec![],
            ),
            Polygon::new(
                LineString::from(vec![
                    coord! {x:30.0,y:30.0},
                    coord! {x:34.0,y:30.0},
                    coord! {x:34.0,y:33.0},
                    coord! {x:30.0,y:33.0},
                    coord! {x:30.0,y:30.0},
                ]),
                vec![],
            ),
        ]);

        let gc = GeometryCollection::new_from(vec![
            Polygon::new(
                LineString::from(vec![
                    coord! {x:40.0,y:40.0},
                    coord! {x:42.0,y:40.0},
                    coord! {x:42.0,y:41.0},
                    coord! {x:40.0,y:40.0},
                ]),
                vec![],
            )
            .into(),
            MultiPoint::from(vec![Point::new(50.0, 51.0), Point::new(52.0, 53.0)]).into(),
        ]);

        KojiGeometryCollection::new(vec![
            KojiGeometry::new(poly_with_hole).with_meta(poly_meta),
            KojiGeometry::new(multi_poly).with_meta(KojiMeta {
                name: Some("multi".into()),
                mode: Mode::Fort,
                ..Default::default()
            }),
            KojiGeometry::new(Geometry::GeometryCollection(gc)).with_meta(KojiMeta {
                name: Some("gc".into()),
                ..Default::default()
            }),
        ])
    }

    /// Every non-line `ReturnTypeArg` must parity-match the old matrix `send`
    /// body. The body is serialized to JSON (the actual wire shape — `GeoFormats`
    /// is untagged) for a full-fidelity comparison even where the inner types lack
    /// `PartialEq`.
    ///
    /// LineString geometries are deliberately NOT covered: the adapters correctly
    /// emit line coords the matrix silently dropped (locked-decision intentional
    /// divergence), so a bare-line collection would fail parity on a pre-existing
    /// matrix bug, not an adapter defect.
    #[test]
    fn dispatch_parity_with_matrix_oracle_all_return_types() {
        let c = sample_rich();
        let cases = [
            ReturnTypeArg::SingleStruct,
            ReturnTypeArg::MultiStruct,
            ReturnTypeArg::Text,
            ReturnTypeArg::AltText,
            ReturnTypeArg::SingleArray,
            ReturnTypeArg::MultiArray,
            ReturnTypeArg::Geometry,
            ReturnTypeArg::GeometryVec,
            ReturnTypeArg::Feature,
            ReturnTypeArg::FeatureVec,
            ReturnTypeArg::FeatureCollection,
            ReturnTypeArg::Poracle,
            ReturnTypeArg::Sql,
        ];
        for rt in cases {
            let mine = serde_json::to_value(response_body(&c, rt.clone())).unwrap();
            let oracle = serde_json::to_value(oracle_body(&c, rt.clone())).unwrap();
            assert_eq!(
                mine, oracle,
                "return type {rt:?} diverged from the matrix oracle"
            );
        }
    }

    /// `PoracleSingle` returns the first item only; parity-check it separately
    /// (the multi-feature `sample_rich` makes `.first()` meaningful).
    #[test]
    fn dispatch_parity_poracle_single() {
        let c = sample_rich();
        let mine = serde_json::to_value(response_body(&c, ReturnTypeArg::PoracleSingle)).unwrap();
        let oracle = serde_json::to_value(oracle_body(&c, ReturnTypeArg::PoracleSingle)).unwrap();
        assert_eq!(mine, oracle);
    }

    /// A bare-LineString collection: NOT a parity case (the oracle drops lines),
    /// but it must not panic and must carry the line's coordinates through the
    /// adapter — proving the swap fixes the coordinate-dropping bug.
    #[test]
    fn line_geometry_is_carried_not_dropped() {
        let line = LineString::from(vec![coord! {x:1.0,y:2.0}, coord! {x:3.0,y:4.0}]);
        let c = KojiGeometryCollection::new(vec![KojiGeometry::new(line)]);

        // Adapter keeps the two coordinates ([lat, lon] order).
        match response_body(&c, ReturnTypeArg::SingleArray) {
            GeoFormats::SingleArray(v) => assert_eq!(v, vec![[2.0, 1.0], [4.0, 3.0]]),
            other => panic!("expected SingleArray, got {other:?}"),
        }
        // The matrix oracle would have dropped them.
        match oracle_body(&c, ReturnTypeArg::SingleArray) {
            GeoFormats::SingleArray(v) => assert!(v.is_empty(), "oracle is expected to drop lines"),
            other => panic!("expected SingleArray, got {other:?}"),
        }
    }
}
