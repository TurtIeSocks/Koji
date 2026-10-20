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
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
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
