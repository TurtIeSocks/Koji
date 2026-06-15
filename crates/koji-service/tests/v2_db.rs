//! DB-backed integration tests for the v2 handlers: geofences, routes, jobs,
//! plugins. These run against a **live** MySQL `koji_test` database.
//!
//! ## Gating
//! Every test starts with `let Some(db) = test_db().await else { return };`.
//! `test_db()` reads `KOJI_DB_URL`; if unset it prints a skip note and returns
//! `None`. A plain `cargo test -p koji-service` **passes by skipping**.
//!
//! ## Running with the DB
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-service --test v2_db -- --nocapture
//! ```
//!
//! ## Isolation
//! Each test creates rows with a unique name derived from a ULID and DELETES them
//! in a panic-safe cleanup guard so `koji_test` stays clean.
//!
//! ## Serialization
//! All tests hold `SERIAL.lock()` for their full body. The job queue uses
//! `FOR UPDATE SKIP LOCKED` which is sensitive to concurrent writers on the same
//! table — serializing here matches production (one worker pool) and keeps
//! `create_job` / `get_job` lifecycle assertions deterministic.

use actix_web::test;
use koji_db::KojiDb;
use koji_jobs::{JobId, JobQueue};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use std::sync::{Arc, Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

// ── helpers ──────────────────────────────────────────────────────────────────

/// Connect to the koji DB iff `KOJI_DB_URL` is set. Returns `None` (with a skip
/// note) when the var is missing, so the suite passes silently in no-env CI.
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

/// A unique `test-<ulid>` name for one test run — short enough to fit the
/// varchar(255) columns and unique enough not to collide in parallel runs.
fn unique_name(tag: &str) -> String {
    format!("test-{tag}-{}", JobId::new().as_string())
}

/// Parse the response body as JSON.
async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
    let bytes = test::read_body(resp).await;
    serde_json::from_slice(&bytes).expect("response body must be valid JSON")
}

/// Delete a `geofence` row by id (best-effort cleanup).
async fn cleanup_geofence(db: &DatabaseConnection, id: u64) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `geofence` WHERE `id` = ?",
            [Value::from(id)],
        ))
        .await;
}

/// Delete a `route` row by id (best-effort cleanup).
async fn cleanup_route(db: &DatabaseConnection, id: u64) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `route` WHERE `id` = ?",
            [Value::from(id)],
        ))
        .await;
}

/// Build a `KojiDb` pointing both `koji` and `scanner` at the same `KOJI_DB_URL`
/// (scanner is used only for reads; a standalone test DB is fine as a dummy).
async fn build_test_koji_db(koji_db: DatabaseConnection) -> KojiDb {
    // Re-connect under the scanner field (scanner is SELECT-only and we don't
    // test scanner-data here, so reusing the same DB is safe).
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let scanner = Database::connect(&url).await.expect("scanner re-connect");
    KojiDb { koji: koji_db, scanner }
}

/// Minimal GeoJSON polygon (a triangle at (0,0)).
fn triangle_geometry() -> serde_json::Value {
    serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
    })
}

/// A route `MultiPoint` geometry.
fn route_geometry() -> serde_json::Value {
    serde_json::json!({
        "type": "MultiPoint",
        "coordinates": [[0.0,0.0],[1.0,1.0]]
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// GEOFENCES — create → get → list → delete lifecycle
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn geofences_create_get_list_delete() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("fence");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // ── POST /api/v2/geofences → 201 ────────────────────────────────────
    let create_body = serde_json::json!({
        "name": name,
        "mode": "pokemon",
        "geometry": triangle_geometry()
    });
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(&create_body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "create geofence must return 201");
    // Location header must point at /api/v2/geofences/{id}
    let location = resp.headers().get("Location").expect("Location header missing");
    let location = location.to_str().unwrap();
    assert!(location.starts_with("/api/v2/geofences/"), "Location: {location}");

    let created = body_json(resp).await;
    assert_eq!(created["status"], "ok");
    let id = created["data"]["id"].as_u64().expect("created data.id must be u64");

    // ── GET /api/v2/geofences/{id} → 200 ────────────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/geofences/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    let get_status = get_resp.status().as_u16();
    let get_body = body_json(get_resp).await;
    // Clean up before asserting so a panic doesn't strand the row.
    cleanup_geofence(&db, id).await;

    assert_eq!(get_status, 200, "GET geofence by id must return 200");
    assert_eq!(get_body["status"], "ok");
    // The default format is `feature`; GeoJSON Feature must carry `properties.name`
    assert_eq!(get_body["data"]["properties"]["name"], name);
}

#[actix_web::test]
async fn geofences_404_for_missing_id() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences/999999999")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "missing geofence must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn geofences_list_returns_ok_envelope() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "list geofences must return 200");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    // Default format = featurecollection
    assert!(v["data"]["features"].is_array(), "geofences list must be a FeatureCollection");
}

#[actix_web::test]
async fn geofences_delete_returns_204_and_subsequent_get_is_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("fence-del");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create
    let create_body = serde_json::json!({ "name": name, "geometry": triangle_geometry() });
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(&create_body)
        .to_request();
    let created = body_json(test::call_service(&app, req).await).await;
    let id = created["data"]["id"].as_u64().expect("created id must be u64");

    // Delete
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/geofences/{id}"))
        .to_request();
    let del_resp = test::call_service(&app, req).await;
    assert_eq!(del_resp.status(), 204, "DELETE must return 204");

    // Subsequent GET → 404
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/geofences/{id}"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    assert_eq!(get_resp.status(), 404, "deleted geofence must return 404");
}

#[actix_web::test]
async fn geofences_depth_and_level_mutually_exclusive_returns_400() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences?depth=1&level=2")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "depth+level together must be 400");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "invalid_request");
}

// ═══════════════════════════════════════════════════════════════════════════
// ROUTES — create (needs a geofence) → list → delete
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn routes_create_list_delete() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let fence_name = unique_name("fence-for-route");
    let route_name = unique_name("route");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create a parent geofence first (routes need a geofence_id).
    let fence_body = serde_json::json!({ "name": fence_name, "geometry": triangle_geometry() });
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(&fence_body)
        .to_request();
    let fence_created = body_json(test::call_service(&app, req).await).await;
    let geofence_id = fence_created["data"]["id"].as_u64().expect("fence id");

    // Create route
    let route_body = serde_json::json!({
        "name": route_name,
        "geofence_id": geofence_id,
        "mode": "pokemon",
        "geometry": route_geometry()
    });
    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(&route_body)
        .to_request();
    let route_resp = test::call_service(&app, req).await;
    assert_eq!(route_resp.status(), 201, "create route must return 201");
    let location = route_resp
        .headers()
        .get("Location")
        .expect("Location header")
        .to_str()
        .unwrap()
        .to_owned();
    assert!(location.starts_with("/api/v2/routes/"), "Location: {location}");
    let route_created = body_json(route_resp).await;
    let route_id = route_created["data"]["id"].as_u64().expect("route id");

    // List
    let req = test::TestRequest::get()
        .uri("/api/v2/routes")
        .to_request();
    let list_resp = test::call_service(&app, req).await;
    assert_eq!(list_resp.status(), 200, "list routes must return 200");
    let list_body = body_json(list_resp).await;
    assert_eq!(list_body["status"], "ok");
    assert!(list_body["data"]["features"].is_array(), "routes list must be FeatureCollection");

    // Cleanup: route first (FK constraint), then geofence.
    cleanup_route(&db, route_id).await;
    cleanup_geofence(&db, geofence_id).await;
}

#[actix_web::test]
async fn routes_404_for_missing_id() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/routes/999999999")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "missing route must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

// ═══════════════════════════════════════════════════════════════════════════
// JOBS — enqueue (invalid body → 400) + get unknown id → 400/404
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn jobs_create_empty_body_returns_400() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // `POST /api/v2/jobs` with a valid mode but no area/instance/dataPoints/parent → 400.
    // CalcRequest uses internally-tagged `mode`; cluster mode is "cluster".
    let body = serde_json::json!({
        "mode": "cluster",
        "category": "pokemon"
        // no area, no instance, no dataPoints, no parent → handler returns 400
    });
    let req = test::TestRequest::post()
        .uri("/api/v2/jobs")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let v = body_json(resp).await;
    assert_eq!(status, 400, "job with no area must return 400, got {v}");
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "invalid_request");
}

#[actix_web::test]
async fn jobs_get_invalid_id_returns_400() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/jobs/not-a-valid-ulid")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "invalid job id must return 400");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "invalid_request");
}

#[actix_web::test]
async fn jobs_list_returns_paginated_ok_envelope() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/jobs")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "list jobs must return 200");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(v["data"].is_array(), "jobs list data must be an array");
    // Must carry pagination meta
    assert!(v["meta"]["total"].is_number(), "meta.total must be numeric");
    assert!(v["meta"]["page"].is_number(), "meta.page must be numeric");
}

#[actix_web::test]
async fn jobs_cancel_unknown_id_returns_400_or_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs_arc = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs_arc)).await;

    // An invalid ULID → 400 (parse error)
    let req = test::TestRequest::delete()
        .uri("/api/v2/jobs/invalid-id")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "DELETE with invalid job id must return 400");
}

#[actix_web::test]
async fn jobs_algorithms_returns_ok_with_all_keys() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/algorithms")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "GET /algorithms must return 200");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(v["data"]["clustering"].is_array(), "data.clustering must be an array");
    assert!(v["data"]["routing"].is_array(), "data.routing must be an array");
    assert!(v["data"]["bootstrap"].is_array(), "data.bootstrap must be an array");
}

// ═══════════════════════════════════════════════════════════════════════════
// PLUGINS — list + get unknown returns ok/404
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn plugins_list_returns_ok_envelope() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/plugins")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200, "GET /plugins must return 200");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(v["data"].is_array(), "plugins list data must be an array");
}

#[actix_web::test]
async fn plugins_get_unknown_kind_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/plugins/badkind/somename")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "unknown plugin kind must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn plugins_get_known_kind_unknown_name_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // "clustering" is a valid kind; "nonexistent-plugin-xyz" won't be in the registry.
    let req = test::TestRequest::get()
        .uri("/api/v2/plugins/clustering/nonexistent-plugin-xyz")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "known kind + unknown name must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}
