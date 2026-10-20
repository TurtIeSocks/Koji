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

// ── GET /geofence/{return_type}/{project} ────────────────────────────────────

async fn get(uri: &str, token: &str) -> (u16, Value) {
    let app = test::init_service(app()).await;
    let req = test::TestRequest::get()
        .uri(uri)
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let bytes = test::read_body(resp).await;
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[actix_web::test]
async fn geofence_route_needs_the_bearer_token() {
    let _s = Secret::set(TOKEN);
    let (status, _) = get("/api/v1/geofence/feature-collection/consumer", "wrong").await;
    assert_eq!(status, 401);
}

#[actix_web::test]
async fn old_return_type_spellings_are_a_404_not_a_fallback() {
    let _s = Secret::set(TOKEN);
    for rt in [
        "featureCollection",
        "featurecollection",
        "feature_collection",
        "FEATURE-COLLECTION",
        "single_array",
        "nonsense",
    ] {
        let (status, v) = get(&format!("/api/v1/geofence/{rt}/consumer"), TOKEN).await;
        assert_eq!(status, 404, "{rt}");
        assert_failure(status, &v);
    }
}

#[actix_web::test]
async fn a_bad_query_value_is_a_400_in_the_v1_envelope() {
    let _s = Secret::set(TOKEN);
    let (status, v) = get(
        "/api/v1/geofence/feature-collection/consumer?name=maybe",
        TOKEN,
    )
    .await;
    assert_eq!(status, 400);
    assert_failure(status, &v);
}

// ── against a seeded project ─────────────────────────────────────────────────
//
// Gated on `KOJI_DB_URL` like `tests/v2_db.rs` (skips when unset):
// ```sh
// set -a; source ./.env.test; set +a
// cargo test -p koji-service --test v1_shim -- --nocapture
// ```
//
// The project holds three 0.1 degree squares, named so the `name` args have
// something to bite on:
//
//   "k8 Paris"         lon 2.0 lat 48.0   no parent
//   "k8 Paris Centre"  lon 2.5 lat 48.5   parent geofence "k8 Paris"
//   "k8 Paris North"   lon 2.8 lat 48.8   parent geofence "k8 Paris", a manual
//                                         `parent` property "Manual" and a
//                                         `k8secret` property
//
// plus "k8 Elsewhere", which is in no project.

use koji_db::db::{geofence, project};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

async fn test_db() -> Option<DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return None;
    };
    match Database::connect(&url).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("skip: could not connect to KOJI_DB_URL: {e}");
            None
        }
    }
}

fn square(lon: f64, lat: f64) -> Value {
    // Literal corners, so the expected values below are exact decimals.
    let (lon2, lat2) = (
        (lon * 10.0 + 1.0).round() / 10.0,
        (lat * 10.0 + 1.0).round() / 10.0,
    );
    json!({
        "type": "Polygon",
        "coordinates": [[[lon, lat], [lon2, lat], [lon2, lat2], [lon, lat2], [lon, lat]]]
    })
}

async fn make_fence(
    db: &DatabaseConnection,
    name: &str,
    geometry: Value,
    project_id: Option<u32>,
    properties: Value,
) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({
            "name": name,
            "mode": "unset",
            "geometry": geometry,
            "projects": project_id.into_iter().collect::<Vec<_>>(),
            "properties": properties,
        }),
    )
    .await
    .expect("make_fence");
    created["id"].as_u64().expect("fence id") as u32
}

/// Seed the project, request every `uris` entry (`{project}` and `{project_id}`
/// are filled in), clean up, and hand back `(status, body)` for each. `None`
/// without a database.
async fn seeded(uris: &[&str]) -> Option<Vec<(u16, Value)>> {
    let db = test_db().await?;
    let project_name = format!("k8-{}", koji_jobs::JobId::new().as_string());
    let project_id = project::Query::upsert_json_return(&db, 0, json!({ "name": project_name }))
        .await
        .expect("make project")["id"]
        .as_u64()
        .expect("project id") as u32;

    let paris = make_fence(
        &db,
        "k8 Paris",
        square(2.0, 48.0),
        Some(project_id),
        json!([]),
    )
    .await;
    let centre = make_fence(
        &db,
        "k8 Paris Centre",
        square(2.5, 48.5),
        Some(project_id),
        json!([]),
    )
    .await;
    let north = make_fence(
        &db,
        "k8 Paris North",
        square(2.8, 48.8),
        Some(project_id),
        json!([
            { "name": "k8secret", "category": "string", "value": "x" },
            { "name": "parent", "category": "string", "value": "Manual" },
        ]),
    )
    .await;
    let elsewhere = make_fence(&db, "k8 Elsewhere", square(3.0, 49.0), None, json!([])).await;
    for child in [centre, north] {
        geofence::Query::set_parent(&db, child, Some(paris))
            .await
            .expect("set parent");
    }

    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat connect");
    let koji = koji_db::KojiDb {
        koji: db.clone(),
        golbat,
    };
    let jobs = Arc::new(koji_jobs::JobQueue::new(db.clone(), "test"));
    let app = test::init_service(koji_service::test_v1_shim_app(koji, jobs)).await;

    let mut out = vec![];
    for uri in uris {
        let uri = uri
            .replace("{project_id}", &project_id.to_string())
            .replace("{project}", &project_name);
        let req = test::TestRequest::get()
            .uri(&uri)
            .insert_header(("Authorization", format!("Bearer {TOKEN}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        let status = resp.status().as_u16();
        let bytes = test::read_body(resp).await;
        out.push((
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        ));
    }

    for id in [paris, centre, north, elsewhere] {
        let _ = geofence::Query::delete(&db, id).await;
    }
    let _ = project::Query::delete(&db, project_id).await;
    let _ = db
        .execute(Statement::from_string(
            DbBackend::MySql,
            "DELETE FROM `property` WHERE `name` = 'k8secret'".to_string(),
        ))
        .await;
    Some(out)
}

const GET: &str = "/api/v1/geofence";

/// The ok envelope of a geofence read: no stats, as on main.
fn assert_geofence_envelope(status: u16, v: &Value) {
    assert_eq!(status, 200, "body: {v}");
    assert_eq!(v["message"], "Success");
    assert_eq!(v["status"], "ok");
    assert_eq!(v["status_code"], 200);
    assert!(v["stats"].is_null(), "stats: {v}");
    assert!(!v["data"].is_null(), "data: {v}");
}

/// The `name` property of each feature, in order.
fn names(data: &Value) -> Vec<String> {
    data["features"]
        .as_array()
        .expect("data.features")
        .iter()
        .map(|f| f["properties"]["name"].as_str().unwrap().to_string())
        .collect()
}

/// `[lat, lon]` of the three squares' rings, in name order.
fn rings() -> Value {
    json!([
        [
            [48.0, 2.0],
            [48.0, 2.1],
            [48.1, 2.1],
            [48.1, 2.0],
            [48.0, 2.0]
        ],
        [
            [48.5, 2.5],
            [48.5, 2.6],
            [48.6, 2.6],
            [48.6, 2.5],
            [48.5, 2.5]
        ],
        [
            [48.8, 2.8],
            [48.8, 2.9],
            [48.9, 2.9],
            [48.9, 2.8],
            [48.8, 2.8]
        ],
    ])
}

fn flat_rings() -> Vec<Value> {
    rings()
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|ring| ring.as_array().unwrap().clone())
        .collect()
}

#[actix_web::test]
async fn single_array_is_every_ring_flat_as_lat_lon() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/single-array/{{project}}"),
        &format!("{GET}/array/{{project}}"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    assert_eq!(r[0].1["data"], Value::Array(flat_rings()));
    assert_eq!(r[1].1["data"], r[0].1["data"]);
}

#[actix_web::test]
async fn multi_array_is_one_ring_per_geofence() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/multi-array/{{project}}")]).await else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    assert_eq!(r[0].1["data"], rings());
}

#[actix_web::test]
async fn single_struct_is_lat_lon_objects_closed_on_the_first_point() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/single-struct/{{project}}"),
        &format!("{GET}/struct/{{project}}"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let data = r[0].1["data"].as_array().unwrap();
    assert_eq!(data.len(), 16);
    assert_eq!(data[0], json!({ "lat": 48.0, "lon": 2.0 }));
    assert_eq!(data[15], data[0]);
    assert_eq!(r[1].1["data"], r[0].1["data"]);
}

#[actix_web::test]
async fn multi_struct_is_one_lat_lon_list_per_geofence() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/multi-struct/{{project}}")]).await else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let data = r[0].1["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);
    assert_eq!(data[2][2], json!({ "lat": 48.9, "lon": 2.9 }));
    assert!(data.iter().all(|ring| ring.as_array().unwrap().len() == 5));
}

#[actix_web::test]
async fn alt_text_is_space_separated_points_joined_by_commas() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/alt-text/{{project}}")]).await else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    assert_eq!(
        r[0].1["data"],
        "48 2,48 2.1,48.1 2.1,48.1 2,48 2,\
         48.5 2.5,48.5 2.6,48.6 2.6,48.6 2.5,48.5 2.5,\
         48.8 2.8,48.8 2.9,48.9 2.9,48.9 2.8,48.8 2.8"
    );
}

#[actix_web::test]
async fn text_labels_each_geofence() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/text/{{project}}")]).await else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let text = r[0].1["data"].as_str().unwrap();
    assert!(text.starts_with("[Geofence 1]\n48,2\n48,2.1\n"), "{text}");
    assert!(text.contains("[Geofence 2]\n48.5,2.5\n"), "{text}");
    assert!(text.contains("[Geofence 3]\n48.8,2.8\n"), "{text}");
}

#[actix_web::test]
async fn feature_collection_is_the_geofences_in_name_order() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/{{project}}?name=true&parent=true"),
        &format!("{GET}/feature-collection/{{project}}"),
    ])
    .await
    else {
        return;
    };
    let (status, v) = &r[0];
    assert_geofence_envelope(*status, v);
    let data = &v["data"];
    assert_eq!(data["type"], "FeatureCollection");
    assert_eq!(
        names(data),
        ["k8 Paris", "k8 Paris Centre", "k8 Paris North"]
    );
    let features = data["features"].as_array().unwrap();
    assert!(features[0]["properties"]["parent"].is_null());
    assert_eq!(features[1]["properties"]["parent"], "k8 Paris");
    // The manual `parent` property wins over the parent geofence.
    assert_eq!(features[2]["properties"]["parent"], "Manual");
    assert_eq!(features[2]["properties"]["k8secret"], "x");
    assert_eq!(features[0]["geometry"]["type"], "Polygon");
    assert_eq!(
        features[1]["geometry"]["coordinates"][0][0],
        json!([2.5, 48.5])
    );
    assert_eq!(data["bbox"], json!([2.0, 48.0, 2.9, 48.9]));
    assert_eq!(features[0]["bbox"], json!([2.0, 48.0, 2.1, 48.1]));
    // Nothing else in the db is in this project.
    assert_eq!(features.len(), 3);
    // `name` and `parent` are opt-in args; without them neither `mode` nor `id`
    // is added.
    let plain = &r[1].1["data"]["features"][1]["properties"];
    assert!(
        plain.get("mode").is_none() && plain.get("id").is_none(),
        "{plain}"
    );
}

#[actix_web::test]
async fn feature_is_a_list_for_many_and_bare_for_one() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature/{{project}}"),
        &format!("{GET}/feature/{{project}}?exclude=k8%20Paris,k8%20Paris%20Centre"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let many = r[0].1["data"].as_array().unwrap();
    assert_eq!(many.len(), 3);
    assert_eq!(many[0]["type"], "Feature");
    assert_eq!(r[1].1["data"]["type"], "Feature");
    assert_eq!(r[1].1["data"]["geometry"]["type"], "Polygon");
}

#[actix_web::test]
async fn feature_vec_is_always_a_list() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-vec/{{project}}"),
        &format!("{GET}/feature-vec/{{project}}?exclude=k8%20Paris,k8%20Paris%20Centre"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    assert_eq!(r[0].1["data"].as_array().unwrap().len(), 3);
    assert_eq!(r[0].1["data"][0]["type"], "Feature");
    assert_eq!(r[1].1["data"].as_array().unwrap().len(), 1);
}

#[actix_web::test]
async fn geometry_is_a_list_for_many_and_bare_for_one() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/geometry/{{project}}"),
        &format!("{GET}/geometry-vec/{{project}}"),
        &format!("{GET}/geometries/{{project}}"),
        &format!("{GET}/geometry/{{project}}?exclude=k8%20Paris,k8%20Paris%20Centre"),
        &format!("{GET}/geometry-vec/{{project}}?exclude=k8%20Paris,k8%20Paris%20Centre"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let many = r[0].1["data"].as_array().unwrap();
    assert_eq!(many.len(), 3);
    assert_eq!(many[0]["type"], "Polygon");
    assert_eq!(r[1].1["data"], r[0].1["data"]);
    assert_eq!(r[2].1["data"], r[0].1["data"]);
    assert_eq!(r[3].1["data"]["type"], "Polygon");
    assert_eq!(r[4].1["data"].as_array().unwrap().len(), 1);
}

#[actix_web::test]
async fn poracle_is_named_grouped_and_numbered() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/poracle/{{project}}?name=true&parent=true")]).await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let data = r[0].1["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);
    assert_eq!(data[0]["name"], "k8 Paris");
    assert_eq!(data[1]["name"], "k8 Paris Centre");
    assert_eq!(data[1]["group"], "k8 Paris");
    assert_eq!(data[1]["id"], 2);
    assert_eq!(data[0]["path"][0], json!([48.0, 2.0]));
}

#[actix_web::test]
async fn sql_has_one_clause_per_geofence() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!("{GET}/sql/{{project}}")]).await else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    let sql = r[0].1["data"].as_str().unwrap();
    assert_eq!(sql.matches("ST_CONTAINS").count(), 3, "{sql}");
    assert!(
        sql.contains("lon BETWEEN 2 AND 2.1\n\tAND lat BETWEEN 48 AND 48.1"),
        "{sql}"
    );
}

#[actix_web::test]
async fn a_project_is_found_by_name_or_id() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/{{project}}?name=true"),
        &format!("{GET}/feature-collection/{{project_id}}?name=true"),
    ])
    .await
    else {
        return;
    };
    assert_eq!(names(&r[0].1["data"]).len(), 3);
    assert_eq!(r[1].1["data"], r[0].1["data"]);
}

#[actix_web::test]
async fn an_unknown_project_is_an_empty_200() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/k8-no-such-project"),
        &format!("{GET}/single-array/k8-no-such-project"),
        &format!("{GET}/feature/k8-no-such-project"),
    ])
    .await
    else {
        return;
    };
    assert_geofence_envelope(r[0].0, &r[0].1);
    assert_eq!(r[0].1["data"]["features"], json!([]));
    assert_geofence_envelope(r[1].0, &r[1].1);
    assert_eq!(r[1].1["data"], json!([]));
    assert_eq!(r[2].1["data"], json!([]));
}

#[actix_web::test]
async fn parentstart_and_parentend_wrap_the_name_in_the_parent() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/{{project}}?name=true&parent=true&parentstart=-"),
        &format!("{GET}/feature-collection/{{project}}?name=true&parent=true&parentend=_"),
    ])
    .await
    else {
        return;
    };
    assert_eq!(
        names(&r[0].1["data"]),
        [
            "k8 Paris",
            "k8 Paris-k8 Paris Centre",
            // The manual parent is the parent here.
            "Manual-k8 Paris North",
        ]
    );
    assert_eq!(
        names(&r[1].1["data"]),
        [
            "k8 Paris",
            "k8 Paris Centre_k8 Paris",
            "k8 Paris North_Manual"
        ]
    );
}

#[actix_web::test]
async fn parentreplace_swaps_the_parent_for_the_given_text() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!(
        "{GET}/feature-collection/{{project}}?name=true&parent=true&parentreplace=City"
    )])
    .await
    else {
        return;
    };
    assert_eq!(
        names(&r[0].1["data"]),
        [
            "k8 Paris",
            "City Centre",
            // "Manual" is the parent and is not in the name.
            "k8 Paris North",
        ]
    );
}

#[actix_web::test]
async fn ignoremanualparent_uses_the_parent_geofence_instead() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/{{project}}?name=true&parent=true"),
        &format!(
            "{GET}/feature-collection/{{project}}?name=true&parent=true&ignoremanualparent=true"
        ),
        &format!(
            "{GET}/feature-collection/{{project}}?name=true&parentstart=-&ignoremanualparent=true"
        ),
    ])
    .await
    else {
        return;
    };
    assert_eq!(
        r[0].1["data"]["features"][2]["properties"]["parent"],
        "Manual"
    );
    assert_eq!(
        r[1].1["data"]["features"][2]["properties"]["parent"],
        "k8 Paris"
    );
    assert_eq!(
        names(&r[2].1["data"])[2],
        "k8 Paris-k8 Paris North",
        "the name modifiers use it too"
    );
}

#[actix_web::test]
async fn exclude_drops_geofences_by_name() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!(
        "{GET}/feature-collection/{{project}}?name=true&exclude=k8%20Paris,k8%20Paris%20North"
    )])
    .await
    else {
        return;
    };
    assert_eq!(names(&r[0].1["data"]), ["k8 Paris Centre"]);
}

#[actix_web::test]
async fn excludeparents_drops_the_children_of_those_parents() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[
        &format!("{GET}/feature-collection/{{project}}?name=true&excludeparents=k8%20Paris"),
        &format!(
            "{GET}/feature-collection/{{project}}?name=true&excludeparents=k8%20Paris&ignoremanualparent=true"
        ),
        &format!("{GET}/feature-collection/{{project}}?name=true&excludeparents=Manual"),
    ])
    .await
    else {
        return;
    };
    // "k8 Paris North" has the manual parent "Manual", so it stays.
    assert_eq!(names(&r[0].1["data"]), ["k8 Paris", "k8 Paris North"]);
    assert_eq!(names(&r[1].1["data"]), ["k8 Paris"]);
    assert_eq!(names(&r[2].1["data"]), ["k8 Paris", "k8 Paris Centre"]);
}

#[actix_web::test]
async fn excludeproperties_drops_geofences_that_have_the_property() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!(
        "{GET}/feature-collection/{{project}}?name=true&excludeproperties=k8secret"
    )])
    .await
    else {
        return;
    };
    assert_eq!(names(&r[0].1["data"]), ["k8 Paris", "k8 Paris Centre"]);
}

#[actix_web::test]
async fn name_args_rewrite_the_name() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!(
        "{GET}/feature-collection/{{project}}?name=true&lowercase=true&space=_&trimstart=3"
    )])
    .await
    else {
        return;
    };
    assert_eq!(
        names(&r[0].1["data"]),
        ["paris", "paris_centre", "paris_north"]
    );
}

#[actix_web::test]
async fn id_mode_and_group_add_their_properties() {
    let _s = Secret::set(TOKEN);
    let Some(r) = seeded(&[&format!(
        "{GET}/feature-collection/{{project}}?id=true&mode=true&group=true"
    )])
    .await
    else {
        return;
    };
    let features = r[0].1["data"]["features"].as_array().unwrap();
    assert!(features[0]["properties"]["id"].is_u64());
    assert_eq!(features[0]["properties"]["mode"], "unset");
    // `group` is the parent's name, and only set where there is a parent.
    assert!(features[0]["properties"].get("group").is_none());
    assert_eq!(features[1]["properties"]["group"], "k8 Paris");
}
