//! `/api/v1` calc shim: Dragonite's request shapes in, v1's envelope out.
//!
//! DB-free: every request supplies its own `area` / `data_points`, so the
//! (disconnected) `KojiDb` is never queried. Tests that touch `KOJI_SECRET`
//! hold `SERIAL` for their whole body since the env is process-wide.
#![allow(clippy::await_holding_lock)]

use std::sync::{Arc, Mutex, MutexGuard};

use actix_web::test;
use serde_json::{Value, json};

static SERIAL: Mutex<()> = Mutex::new(());

/// Holds `SERIAL` and sets `KOJI_SECRET` for one test; restores it on drop.
struct Secret {
    _lock: MutexGuard<'static, ()>,
    original: Option<String>,
}

impl Secret {
    fn set(value: &str) -> Self {
        let _lock = SERIAL.lock().unwrap_or_else(|p| p.into_inner());
        let original = std::env::var("KOJI_SECRET").ok();
        // SAFETY: serialized by SERIAL; no other thread reads the env meanwhile.
        unsafe { std::env::set_var("KOJI_SECRET", value) };
        Secret { _lock, original }
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        // SAFETY: same as `set`.
        unsafe {
            match &self.original {
                Some(v) => std::env::set_var("KOJI_SECRET", v),
                None => std::env::remove_var("KOJI_SECRET"),
            }
        }
    }
}

const TOKEN: &str = "dragonite-token";

fn app() -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let db = koji_db::KojiDb {
        koji: Default::default(),
        golbat: Default::default(),
    };
    let jobs = Arc::new(koji_jobs::JobQueue::new(Default::default(), "test"));
    koji_service::test_v1_shim_app(db, jobs)
}

/// A ~1km square FeatureCollection, as Dragonite sends a geofence.
fn area() -> Value {
    json!({
        "type": "FeatureCollection",
        "features": [{
            "type": "Feature",
            "properties": {},
            "geometry": {
                "type": "Polygon",
                "coordinates": [[
                    [-0.005, 51.500], [0.005, 51.500], [0.005, 51.509],
                    [-0.005, 51.509], [-0.005, 51.500]
                ]]
            }
        }]
    })
}

async fn post(uri: &str, token: &str, body: &Value) -> (u16, Value) {
    let app = test::init_service(app()).await;
    let req = test::TestRequest::post()
        .uri(uri)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let bytes = test::read_body(resp).await;
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn assert_lat_lon_pairs(data: &Value) {
    let arr = data.as_array().expect("data is an array");
    assert!(!arr.is_empty(), "data has points");
    for p in arr {
        let p = p.as_array().expect("each point is an array");
        assert_eq!(p.len(), 2, "each point is [lat, lon]");
        let (lat, lon) = (p[0].as_f64().unwrap(), p[1].as_f64().unwrap());
        assert!((51.4..51.6).contains(&lat), "lat {lat} first");
        assert!((-0.1..0.1).contains(&lon), "lon {lon} second");
    }
}

const STATS_KEYS: [&str; 11] = [
    "best_clusters",
    "best_cluster_point_count",
    "cluster_time",
    "route_time",
    "stats_time",
    "total_points",
    "points_covered",
    "total_clusters",
    "total_distance",
    "longest_distance",
    "mygod_score",
];

fn assert_envelope(status: u16, v: &Value) {
    assert_eq!(status, 200, "body: {v}");
    assert_eq!(v["message"], "Success");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["status_code"], 200);
    for k in STATS_KEYS {
        assert!(v["stats"].get(k).is_some(), "stats.{k} missing: {v}");
    }
}

#[actix_web::test]
async fn bootstrap_returns_v1_envelope_with_lat_lon_pairs() {
    let _s = Secret::set(TOKEN);
    let body = json!({
        "instance": "london",
        "radius": 200.0,
        "min_points": 1,
        "area": area(),
        "return_type": "single_array",
        "calculation_mode": "Radius",
        "s2_level": 15,
        "s2_size": 9,
        "sort_by": "tsp"
    });
    let (status, v) = post("/api/v1/calc/bootstrap", TOKEN, &body).await;
    assert_envelope(status, &v);
    assert_lat_lon_pairs(&v["data"]);
}

/// A grid of points inside `area()`, as `[[lat, lon], ...]`.
fn data_points() -> Value {
    let mut pts = vec![];
    for i in 0..5 {
        for j in 0..5 {
            pts.push(json!([
                51.501 + i as f64 * 0.002,
                -0.004 + j as f64 * 0.002
            ]));
        }
    }
    Value::Array(pts)
}

fn route_body() -> Value {
    json!({
        "instance": "london",
        "radius": 70,
        "min_points": 1,
        "data_points": data_points(),
        "area": area(),
        "return_type": "single_array",
        "calculation_mode": "radius",
        "s2_level": 15,
        "s2_size": 9,
        "cluster_mode": "fast",
        "center_clusters": false,
        "sort_by": "tsp"
    })
}

#[actix_web::test]
async fn route_pokestop_returns_v1_envelope_with_lat_lon_pairs() {
    let _s = Secret::set(TOKEN);
    let (status, v) = post("/api/v1/calc/route/pokestop", TOKEN, &route_body()).await;
    assert_envelope(status, &v);
    assert_lat_lon_pairs(&v["data"]);
    assert_eq!(v["stats"]["total_points"], 25);
}

#[actix_web::test]
async fn route_spawnpoint_returns_v1_envelope_with_lat_lon_pairs() {
    let _s = Secret::set(TOKEN);
    let (status, v) = post("/api/v1/calc/route/spawnpoint", TOKEN, &route_body()).await;
    assert_envelope(status, &v);
    assert_lat_lon_pairs(&v["data"]);
}

#[actix_web::test]
async fn route_fort_returns_v1_envelope_with_lat_lon_pairs() {
    let _s = Secret::set(TOKEN);
    let (status, v) = post("/api/v1/calc/route/fort", TOKEN, &route_body()).await;
    assert_envelope(status, &v);
    assert_lat_lon_pairs(&v["data"]);
}

#[actix_web::test]
async fn health_is_200_with_a_valid_token() {
    let _s = Secret::set(TOKEN);
    let app = test::init_service(app()).await;
    let req = test::TestRequest::get()
        .uri("/api/v1/health")
        .insert_header(("Authorization", format!("Bearer {TOKEN}")))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);
}

#[actix_web::test]
async fn bad_token_is_401_on_every_route() {
    let _s = Secret::set(TOKEN);
    let app = test::init_service(app()).await;
    for (method, uri) in [
        ("GET", "/api/v1/health"),
        ("POST", "/api/v1/calc/bootstrap"),
        ("POST", "/api/v1/calc/route/pokestop"),
    ] {
        let req = match method {
            "GET" => test::TestRequest::get(),
            _ => test::TestRequest::post().set_json(route_body()),
        }
        .uri(uri)
        .insert_header(("Authorization", "Bearer wrong"))
        .to_request();
        assert_eq!(test::call_service(&app, req).await.status(), 401, "{uri}");
    }
}

#[actix_web::test]
async fn missing_token_is_401() {
    let _s = Secret::set(TOKEN);
    let app = test::init_service(app()).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/calc/route/pokestop")
        .set_json(route_body())
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 401);
}

/// Dragonite overwrites the route on a 200 with null `data`, so a failure must be
/// a non-200 carrying v1's error envelope.
fn assert_failure(status: u16, v: &Value) {
    assert!(status >= 400, "failure must be non-200, got {status}: {v}");
    assert_eq!(v["status"], "error");
    assert_eq!(v["status_code"], status);
    assert!(v["data"].is_null());
    assert!(v["message"].as_str().is_some_and(|m| !m.is_empty()));
}

#[actix_web::test]
async fn no_area_instance_or_points_is_a_400_not_a_200() {
    let _s = Secret::set(TOKEN);
    let (status, v) = post("/api/v1/calc/route/pokestop", TOKEN, &json!({"radius": 70})).await;
    assert_eq!(status, 400);
    assert_failure(status, &v);
}

#[actix_web::test]
async fn failed_bootstrap_calc_is_non_200() {
    let _s = Secret::set(TOKEN);
    // A point is not a bootstrappable area.
    let body = json!({
        "radius": 70,
        "area": {"type": "FeatureCollection", "features": [{
            "type": "Feature", "properties": {},
            "geometry": {"type": "Point", "coordinates": [0.0, 51.5]}
        }]}
    });
    let (status, v) = post("/api/v1/calc/bootstrap", TOKEN, &body).await;
    assert_failure(status, &v);
}

#[actix_web::test]
async fn malformed_body_is_a_400_in_the_v1_envelope() {
    let _s = Secret::set(TOKEN);
    let app = test::init_service(app()).await;
    let req = test::TestRequest::post()
        .uri("/api/v1/calc/route/pokestop")
        .insert_header(("Authorization", format!("Bearer {TOKEN}")))
        .insert_header(("content-type", "application/json"))
        .set_payload("{not json")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let v: Value = serde_json::from_slice(&test::read_body(resp).await).unwrap();
    assert_eq!(status, 400);
    assert_failure(status, &v);
}

#[actix_web::test]
async fn unknown_category_is_a_400() {
    let _s = Secret::set(TOKEN);
    let (status, v) = post("/api/v1/calc/route/bogus", TOKEN, &route_body()).await;
    assert_eq!(status, 400);
    assert_failure(status, &v);
}
