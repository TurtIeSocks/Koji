use actix_web::HttpResponse;
use algorithms::stats::Stats;
use geojson::JsonValue;
use koji_core::{KojiGeometryCollection, Precision};
use serde::Serialize;
use serde_json::json;

use crate::requests::ReturnTypeArg;

#[derive(Debug, Serialize)]
pub(crate) struct ConfigResponse {
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
pub(crate) struct Response {
    pub message: String,
    pub status: String,
    pub status_code: u16,
    pub data: Option<JsonValue>,
    pub stats: Option<Stats>,
}

impl Response {
    pub(crate) fn send_error(message: &str) -> Response {
        Response {
            message: message.to_string(),
            status: "error".to_string(),
            status_code: 500,
            data: None,
            stats: None,
        }
    }
}

/// The success-envelope shared by ~all CRUD/query handlers: a 200 `Response`
/// with `status: "ok"`, `message: "Success"`, no `stats`, and `data` set to the
/// caller's payload. Single-sources the block that was hand-rolled at every
/// handler return; callers wrap the result in `Ok(...)`.
pub(crate) fn ok_response(data: JsonValue) -> HttpResponse {
    HttpResponse::Ok().json(Response {
        data: Some(data),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    })
}

/// Serialize a `KojiGeometryCollection` into the wire `serde_json::Value` for a
/// requested `ReturnTypeArg`, dispatching to the Phase 1B inherent adapters (on
/// `KojiGeometryCollection`) and the Phase 1 geojson `From` edge conversions.
///
/// Each arm produces its native adapter result and serializes it with
/// `serde_json::to_value`, so the function carries no dependency on the legacy
/// serialization-envelope enum or the `To*` conversion matrix. The output bytes
/// are pinned per-variant by the `golden_*` tests below.
fn response_body(coll: &KojiGeometryCollection, return_type: ReturnTypeArg) -> JsonValue {
    match return_type {
        ReturnTypeArg::SingleStruct => json!(coll.to_single_struct()),
        ReturnTypeArg::MultiStruct => json!(coll.to_multi_struct()),
        ReturnTypeArg::Text => json!(coll.to_text(",", "\n", true)),
        ReturnTypeArg::AltText => json!(coll.to_text(" ", ",", false)),
        ReturnTypeArg::SingleArray => json!(coll.to_single_vec()),
        ReturnTypeArg::MultiArray => json!(coll.to_multi_vec()),
        // Phase 1 outbound: a property-less `GeometryCollection` wrapping every
        // item's geometry (replaces the old bare `[Geometry]` / single-geometry
        // special-case). Always a `GeometryCollection`, regardless of item count.
        ReturnTypeArg::Geometry => json!(geojson::Geometry::from(coll)),
        ReturnTypeArg::Feature => {
            let mut features = geojson::FeatureCollection::from(coll).features;
            if features.len() == 1 {
                json!(features.remove(0))
            } else {
                log::info!(
                    "\"Feature\" was requested as the return type but multiple features were found so a Vec of features is being returned"
                );
                json!(features)
            }
        }
        ReturnTypeArg::FeatureCollection => json!(geojson::FeatureCollection::from(coll)),
        ReturnTypeArg::Poracle => json!(coll.to_poracle_vec()),
        ReturnTypeArg::PoracleSingle => json!(coll.to_poracle_vec().first().unwrap().clone()),
        ReturnTypeArg::Sql => json!(coll.to_sql()),
    }
}

pub(crate) fn send(
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
            Some(response_body(&coll, return_type))
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
    use koji_core::{KojiGeometry, KojiMeta, Mode};

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

    /// A single-item collection: exercises the `Feature` len==1 path (a lone
    /// `Feature`, not an array) and the promoted `Geometry` path (still a
    /// `GeometryCollection`, now with one member).
    fn single_item() -> KojiGeometryCollection {
        let poly = Polygon::new(
            LineString::from(vec![
                coord! {x:0.0,y:0.0},
                coord! {x:2.0,y:0.0},
                coord! {x:2.0,y:1.0},
                coord! {x:0.0,y:0.0},
            ]),
            vec![],
        );
        KojiGeometryCollection::new(vec![KojiGeometry::new(poly).with_meta(KojiMeta {
            id: Some(1),
            name: Some("solo".into()),
            mode: Mode::Fort,
            ..Default::default()
        })])
    }

    /// Golden snapshots for every retained `ReturnTypeArg`, over the adversarial
    /// `sample_rich` collection. These were captured byte-for-byte from the matrix
    /// oracle while it was still alive (Section 5 parity step) — except `Geometry`,
    /// which Task 3 intentionally promotes from the old bare `[Geometry]` array to
    /// a single Phase 1 `GeometryCollection`. They pin the wire output as a
    /// permanent regression guard after the oracle and the legacy serialization
    /// envelope are removed.
    ///
    /// `response_body` returns `serde_json::Value` directly (no envelope enum), so
    /// the golden IS the exact `data` payload `send` emits.
    #[test]
    fn golden_all_retained_return_types() {
        let c = sample_rich();
        let expected: &[(ReturnTypeArg, JsonValue)] = &[
            (
                ReturnTypeArg::SingleStruct,
                json!([{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":10.0},{"lat":4.0,"lon":10.0},{"lat":4.0,"lon":0.0},{"lat":0.0,"lon":0.0},{"lat":1.0,"lon":2.0},{"lat":1.0,"lon":4.0},{"lat":3.0,"lon":4.0},{"lat":3.0,"lon":2.0},{"lat":1.0,"lon":2.0},{"lat":20.0,"lon":20.0},{"lat":20.0,"lon":23.0},{"lat":22.0,"lon":23.0},{"lat":20.0,"lon":20.0},{"lat":30.0,"lon":30.0},{"lat":30.0,"lon":34.0},{"lat":33.0,"lon":34.0},{"lat":33.0,"lon":30.0},{"lat":30.0,"lon":30.0},{"lat":40.0,"lon":40.0},{"lat":40.0,"lon":42.0},{"lat":41.0,"lon":42.0},{"lat":40.0,"lon":40.0},{"lat":51.0,"lon":50.0},{"lat":53.0,"lon":52.0},{"lat":0.0,"lon":0.0}]),
            ),
            (
                ReturnTypeArg::MultiStruct,
                json!([[{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":10.0},{"lat":4.0,"lon":10.0},{"lat":4.0,"lon":0.0},{"lat":0.0,"lon":0.0},{"lat":1.0,"lon":2.0},{"lat":1.0,"lon":4.0},{"lat":3.0,"lon":4.0},{"lat":3.0,"lon":2.0},{"lat":1.0,"lon":2.0},{"lat":0.0,"lon":0.0}],[{"lat":20.0,"lon":20.0},{"lat":20.0,"lon":23.0},{"lat":22.0,"lon":23.0},{"lat":20.0,"lon":20.0},{"lat":30.0,"lon":30.0},{"lat":30.0,"lon":34.0},{"lat":33.0,"lon":34.0},{"lat":33.0,"lon":30.0},{"lat":30.0,"lon":30.0},{"lat":20.0,"lon":20.0}],[{"lat":40.0,"lon":40.0},{"lat":40.0,"lon":42.0},{"lat":41.0,"lon":42.0},{"lat":40.0,"lon":40.0},{"lat":51.0,"lon":50.0},{"lat":53.0,"lon":52.0},{"lat":40.0,"lon":40.0}]]),
            ),
            (
                ReturnTypeArg::Text,
                json!(
                    "[Geofence 1]\n0,0\n0,10\n4,10\n4,0\n0,0\n1,2\n1,4\n3,4\n3,2\n1,2\n\n[Geofence 2]\n20,20\n20,23\n22,23\n20,20\n30,30\n30,34\n33,34\n33,30\n30,30\n\n[Geofence 3]\n40,40\n40,42\n41,42\n40,40\n51,50\n53,52"
                ),
            ),
            (
                ReturnTypeArg::AltText,
                json!(
                    "0 0,0 10,4 10,4 0,0 0,1 2,1 4,3 4,3 2,1 2,20 20,20 23,22 23,20 20,30 30,30 34,33 34,33 30,30 30,40 40,40 42,41 42,40 40,51 50,53 52"
                ),
            ),
            (
                ReturnTypeArg::SingleArray,
                json!([
                    [0.0, 0.0],
                    [0.0, 10.0],
                    [4.0, 10.0],
                    [4.0, 0.0],
                    [0.0, 0.0],
                    [1.0, 2.0],
                    [1.0, 4.0],
                    [3.0, 4.0],
                    [3.0, 2.0],
                    [1.0, 2.0],
                    [20.0, 20.0],
                    [20.0, 23.0],
                    [22.0, 23.0],
                    [20.0, 20.0],
                    [30.0, 30.0],
                    [30.0, 34.0],
                    [33.0, 34.0],
                    [33.0, 30.0],
                    [30.0, 30.0],
                    [40.0, 40.0],
                    [40.0, 42.0],
                    [41.0, 42.0],
                    [40.0, 40.0],
                    [51.0, 50.0],
                    [53.0, 52.0]
                ]),
            ),
            (
                ReturnTypeArg::MultiArray,
                json!([
                    [
                        [0.0, 0.0],
                        [0.0, 10.0],
                        [4.0, 10.0],
                        [4.0, 0.0],
                        [0.0, 0.0],
                        [1.0, 2.0],
                        [1.0, 4.0],
                        [3.0, 4.0],
                        [3.0, 2.0],
                        [1.0, 2.0]
                    ],
                    [
                        [20.0, 20.0],
                        [20.0, 23.0],
                        [22.0, 23.0],
                        [20.0, 20.0],
                        [30.0, 30.0],
                        [30.0, 34.0],
                        [33.0, 34.0],
                        [33.0, 30.0],
                        [30.0, 30.0]
                    ],
                    [
                        [40.0, 40.0],
                        [40.0, 42.0],
                        [41.0, 42.0],
                        [40.0, 40.0],
                        [51.0, 50.0],
                        [53.0, 52.0]
                    ]
                ]),
            ),
            // PROMOTED (Task 3): a single Phase 1 GeometryCollection wrapping every
            // item's geometry, replacing the old bare `[Geometry]` array.
            (
                ReturnTypeArg::Geometry,
                json!({"type":"GeometryCollection","geometries":[{"type":"Polygon","coordinates":[[[0.0,0.0],[10.0,0.0],[10.0,4.0],[0.0,4.0],[0.0,0.0]],[[2.0,1.0],[4.0,1.0],[4.0,3.0],[2.0,3.0],[2.0,1.0]]]},{"type":"MultiPolygon","coordinates":[[[[20.0,20.0],[23.0,20.0],[23.0,22.0],[20.0,20.0]]],[[[30.0,30.0],[34.0,30.0],[34.0,33.0],[30.0,33.0],[30.0,30.0]]]]},{"type":"GeometryCollection","geometries":[{"type":"Polygon","coordinates":[[[40.0,40.0],[42.0,40.0],[42.0,41.0],[40.0,40.0]]]},{"type":"MultiPoint","coordinates":[[50.0,51.0],[52.0,53.0]]}]}]}),
            ),
            (
                ReturnTypeArg::Feature,
                json!([{"geometry":{"coordinates":[[[0.0,0.0],[10.0,0.0],[10.0,4.0],[0.0,4.0],[0.0,0.0]],[[2.0,1.0],[4.0,1.0],[4.0,3.0],[2.0,3.0],[2.0,1.0]]],"type":"Polygon"},"id":7,"properties":{"color":"#00ff00","description":"a rich fence","displayInMatches":false,"group":"alpha","id":7,"mode":"fort","name":"poly","userSelectable":false},"type":"Feature"},{"geometry":{"coordinates":[[[[20.0,20.0],[23.0,20.0],[23.0,22.0],[20.0,20.0]]],[[[30.0,30.0],[34.0,30.0],[34.0,33.0],[30.0,33.0],[30.0,30.0]]]],"type":"MultiPolygon"},"properties":{"mode":"fort","name":"multi"},"type":"Feature"},{"geometry":{"geometries":[{"coordinates":[[[40.0,40.0],[42.0,40.0],[42.0,41.0],[40.0,40.0]]],"type":"Polygon"},{"coordinates":[[50.0,51.0],[52.0,53.0]],"type":"MultiPoint"}],"type":"GeometryCollection"},"properties":{"mode":"unset","name":"gc"},"type":"Feature"}]),
            ),
            (
                ReturnTypeArg::FeatureCollection,
                json!({"features":[{"geometry":{"coordinates":[[[0.0,0.0],[10.0,0.0],[10.0,4.0],[0.0,4.0],[0.0,0.0]],[[2.0,1.0],[4.0,1.0],[4.0,3.0],[2.0,3.0],[2.0,1.0]]],"type":"Polygon"},"id":7,"properties":{"color":"#00ff00","description":"a rich fence","displayInMatches":false,"group":"alpha","id":7,"mode":"fort","name":"poly","userSelectable":false},"type":"Feature"},{"geometry":{"coordinates":[[[[20.0,20.0],[23.0,20.0],[23.0,22.0],[20.0,20.0]]],[[[30.0,30.0],[34.0,30.0],[34.0,33.0],[30.0,33.0],[30.0,30.0]]]],"type":"MultiPolygon"},"properties":{"mode":"fort","name":"multi"},"type":"Feature"},{"geometry":{"geometries":[{"coordinates":[[[40.0,40.0],[42.0,40.0],[42.0,41.0],[40.0,40.0]]],"type":"Polygon"},{"coordinates":[[50.0,51.0],[52.0,53.0]],"type":"MultiPoint"}],"type":"GeometryCollection"},"properties":{"mode":"unset","name":"gc"},"type":"Feature"}],"type":"FeatureCollection"}),
            ),
            (
                ReturnTypeArg::Poracle,
                json!([{"color":"#00ff00","description":"a rich fence","displayInMatches":false,"group":"alpha","id":7,"name":"poly","path":[[0.0,0.0],[0.0,10.0],[4.0,10.0],[4.0,0.0],[0.0,0.0],[1.0,2.0],[1.0,4.0],[3.0,4.0],[3.0,2.0],[1.0,2.0]],"userSelectable":false},{"displayInMatches":true,"id":2,"multipath":[[[20.0,20.0],[20.0,23.0],[22.0,23.0],[20.0,20.0]],[[30.0,30.0],[30.0,34.0],[33.0,34.0],[33.0,30.0],[30.0,30.0]]],"name":"multi","path":[],"userSelectable":true},{"displayInMatches":true,"id":3,"multipath":[[[40.0,40.0],[40.0,42.0],[41.0,42.0],[40.0,40.0]]],"name":"gc","path":[],"userSelectable":true}]),
            ),
            (
                ReturnTypeArg::PoracleSingle,
                json!({"color":"#00ff00","description":"a rich fence","displayInMatches":false,"group":"alpha","id":7,"name":"poly","path":[[0.0,0.0],[0.0,10.0],[4.0,10.0],[4.0,0.0],[0.0,0.0],[1.0,2.0],[1.0,4.0],[3.0,4.0],[3.0,2.0],[1.0,2.0]],"userSelectable":false}),
            ),
            (
                ReturnTypeArg::Sql,
                json!(
                    "SELECT * FROM {database.table} WHERE (\n\tlon BETWEEN 0 AND 10\n\tAND lat BETWEEN 0 AND 4\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{\"type\":\"Polygon\",\"coordinates\":[[[0.0,0.0],[10.0,0.0],[10.0,4.0],[0.0,4.0],[0.0,0.0]],[[2.0,1.0],[4.0,1.0],[4.0,3.0],[2.0,3.0],[2.0,1.0]]]}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)\nOR (\n\tlon BETWEEN 20 AND 34\n\tAND lat BETWEEN 20 AND 33\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{\"type\":\"MultiPolygon\",\"coordinates\":[[[[20.0,20.0],[23.0,20.0],[23.0,22.0],[20.0,20.0]]],[[[30.0,30.0],[34.0,30.0],[34.0,33.0],[30.0,33.0],[30.0,30.0]]]]}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)"
                ),
            ),
        ];
        for (rt, want) in expected {
            let got = response_body(&c, rt.clone());
            assert_eq!(&got, want, "return type {rt:?} diverged from its golden");
        }
    }

    /// Single-item collection goldens: `Feature` collapses to a lone `Feature`
    /// (len==1 path); `Geometry` is a one-member `GeometryCollection`.
    #[test]
    fn golden_single_item() {
        let c = single_item();
        assert_eq!(
            response_body(&c, ReturnTypeArg::Geometry),
            json!({"type":"GeometryCollection","geometries":[{"type":"Polygon","coordinates":[[[0.0,0.0],[2.0,0.0],[2.0,1.0],[0.0,0.0]]]}]}),
        );
        assert_eq!(
            response_body(&c, ReturnTypeArg::Feature),
            json!({"geometry":{"coordinates":[[[0.0,0.0],[2.0,0.0],[2.0,1.0],[0.0,0.0]]],"type":"Polygon"},"id":1,"properties":{"id":1,"mode":"fort","name":"solo"},"type":"Feature"}),
        );
    }

    /// A bare-LineString collection must not panic and must carry the line's
    /// coordinates through the adapter (the matrix used to silently drop them).
    #[test]
    fn line_geometry_is_carried_not_dropped() {
        let line = LineString::from(vec![coord! {x:1.0,y:2.0}, coord! {x:3.0,y:4.0}]);
        let c = KojiGeometryCollection::new(vec![KojiGeometry::new(line)]);

        // Adapter keeps the two coordinates ([lat, lon] order).
        assert_eq!(
            response_body(&c, ReturnTypeArg::SingleArray),
            json!([[2.0, 1.0], [4.0, 3.0]]),
        );
    }
}
