//! DB-backed integration tests for the v2 handlers: geofences, routes, jobs,
// The `serial_guard()` pattern intentionally holds a `MutexGuard` across `.await`
// points to serialise concurrent test execution — safe in test context only.
#![allow(clippy::await_holding_lock)]
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

/// Build a `KojiDb` pointing both `koji` and `golbat` at the same `KOJI_DB_URL`
/// (golbat is used only for reads; a standalone test DB is fine as a dummy).
async fn build_test_koji_db(koji_db: DatabaseConnection) -> KojiDb {
    // Re-connect under the golbat field (golbat is SELECT-only and we don't
    // test golbat-data here, so reusing the same DB is safe).
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    KojiDb {
        koji: koji_db,
        golbat,
    }
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

/// A triangle far from `triangle_geometry()` (bbox `[50,50]-[51,51]`) — used to
/// prove a `?bbox=` filter drops geographically distant fences.
fn far_triangle_geometry() -> serde_json::Value {
    serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[50.0,50.0],[51.0,50.0],[51.0,51.0],[50.0,50.0]]]
    })
}

/// `properties.name` of every feature in a FeatureCollection envelope response.
fn feature_names(v: &serde_json::Value) -> Vec<String> {
    v["data"]["features"]
        .as_array()
        .expect("data.features must be an array")
        .iter()
        .map(|f| f["properties"]["name"].as_str().unwrap().to_string())
        .collect()
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
    let location = resp
        .headers()
        .get("Location")
        .expect("Location header missing");
    let location = location.to_str().unwrap();
    assert!(
        location.starts_with("/api/v2/geofences/"),
        "Location: {location}"
    );

    let created = body_json(resp).await;
    assert_eq!(created["status"], "ok");
    let id = created["data"]["id"]
        .as_u64()
        .expect("created data.id must be u64");

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
    assert!(
        v["data"]["features"].is_array(),
        "geofences list must be a FeatureCollection"
    );
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
    let id = created["data"]["id"]
        .as_u64()
        .expect("created id must be u64");

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
// GEOFENCES — `?ids=` and `?bbox=` scoped list (Task 1)
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn geofences_ids_and_bbox_filters_scope_the_list() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let near_name = unique_name("fence-near");
    let far_name = unique_name("fence-far");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Seed two geofences: one near the origin, one far away.
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": near_name, "mode": "pokemon", "geometry": triangle_geometry()
        }))
        .to_request();
    let near_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .expect("near fence id");

    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": far_name, "mode": "pokemon", "geometry": far_triangle_geometry()
        }))
        .to_request();
    let far_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .expect("far fence id");

    // ── ?ids=<near> → exactly that one feature ──────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/geofences?ids={near_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let ids_status = resp.status();
    let ids_body = body_json(resp).await;

    // ── ?bbox= around the near fence → only the near fence, not the far one ─
    let req = test::TestRequest::get()
        .uri("/api/v2/geofences?bbox=-5,-5,5,5")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let bbox_status = resp.status();
    let bbox_body = body_json(resp).await;

    // ── no params → both fences present (back-compat, unchanged behavior) ──
    let req = test::TestRequest::get()
        .uri("/api/v2/geofences")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let list_status = resp.status();
    let list_body = body_json(resp).await;

    // Cleanup before asserting so a failing assert doesn't strand the rows.
    cleanup_geofence(&db, near_id).await;
    cleanup_geofence(&db, far_id).await;

    assert_eq!(ids_status, 200, "?ids= must return 200");
    assert_eq!(
        feature_names(&ids_body),
        vec![near_name.clone()],
        "?ids=<near> must return exactly that fence"
    );

    assert_eq!(bbox_status, 200, "?bbox= must return 200");
    let bbox_names = feature_names(&bbox_body);
    assert!(
        bbox_names.contains(&near_name),
        "?bbox= around the near fence must include it: {bbox_names:?}"
    );
    assert!(
        !bbox_names.contains(&far_name),
        "?bbox= around the near fence must exclude the far one: {bbox_names:?}"
    );

    assert_eq!(list_status, 200);
    let list_names = feature_names(&list_body);
    assert!(
        list_names.contains(&near_name) && list_names.contains(&far_name),
        "no params must still return every fence (back-compat): {list_names:?}"
    );
}

/// Parity test (Task B): `?bbox=` now filters at the DB via the persisted
/// `min_lat`/`min_lng`/`max_lat`/`max_lng` columns
/// (`geofence::Query::get_koji_by_bbox`) instead of the in-memory
/// `features_intersecting_bbox` scan. Seeds four geofences with known,
/// separated geometries against query box `[0,0,10,10]`:
/// - `inside`   — bbox `[2,2]-[3,3]`, fully inside the box.
/// - `touching` — a `Point` sitting exactly on the box's `(10,10)` corner;
///   the AABB overlap test is non-strict (`<=`/`>=`), so a boundary-straddling
///   geometry still counts as overlapping (pinned in the pure
///   `features_intersecting_bbox_keeps_boundary_straddling_feature` unit test
///   this mirrors).
/// - `outside`  — bbox `[20,20]-[21,21]`, fully outside the box.
/// - `asymmetric_axis` — bbox lng ∈ `[2,3]` / lat ∈ `[20,21]`: overlaps the
///   query box on the **lng** axis but not the **lat** axis, so it must be
///   excluded. Every other fixture here sits on the lat==lng diagonal (e.g.
///   `inside`'s bbox has `min_lat == min_lng == 2`), which means a predicate
///   bug that reads the wrong column — `Column::MinLng` where
///   `Column::MinLat` belongs, or vice versa — would silently compute the
///   *same* number for those rows and slip past undetected. This fixture's
///   lat and lng ranges differ, so such a column swap changes the outcome
///   (wrongly *includes* it) instead of being numerically invisible.
///
/// Asserts the exact expected subset comes back: `inside` and `touching`
/// present, `outside` and `asymmetric_axis` absent — i.e. the DB filter
/// agrees with the in-memory overlap semantics it replaced, on axes as well
/// as on inclusion/exclusion.
#[actix_web::test]
async fn geofences_bbox_db_filter_matches_overlap_semantics_parity() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let inside_name = unique_name("fence-bbox-inside");
    let touching_name = unique_name("fence-bbox-touching");
    let outside_name = unique_name("fence-bbox-outside");
    let asymmetric_name = unique_name("fence-bbox-asymmetric-axis");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let inside_geometry = serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[2.0,2.0],[3.0,2.0],[3.0,3.0],[2.0,2.0]]]
    });
    // A single point exactly on the query box's top-right corner — boundary
    // straddling, not "inside" by any margin.
    let touching_geometry = serde_json::json!({
        "type": "Point",
        "coordinates": [10.0, 10.0]
    });
    let outside_geometry = serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[20.0,20.0],[21.0,20.0],[21.0,21.0],[20.0,20.0]]]
    });
    // lng ∈ [2,3] overlaps the query box; lat ∈ [20,21] does not — an
    // axis-swapped predicate could wrongly include this one. See the doc
    // comment above for why the other (lat==lng diagonal) fixtures can't
    // catch that class of bug.
    let asymmetric_geometry = serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[2.0,20.0],[3.0,20.0],[3.0,21.0],[2.0,20.0]]]
    });

    let mut seeded_ids = Vec::new();
    for (name, geometry) in [
        (&inside_name, inside_geometry),
        (&touching_name, touching_geometry),
        (&outside_name, outside_geometry),
        (&asymmetric_name, asymmetric_geometry),
    ] {
        let req = test::TestRequest::post()
            .uri("/api/v2/geofences")
            .set_json(serde_json::json!({
                "name": name, "mode": "pokemon", "geometry": geometry
            }))
            .to_request();
        let id = body_json(test::call_service(&app, req).await).await["data"]["id"]
            .as_u64()
            .expect("seeded fence id");
        seeded_ids.push(id);
    }

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences?bbox=0,0,10,10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let bbox_status = resp.status();
    let bbox_body = body_json(resp).await;

    // Cleanup before asserting so a failing assert doesn't strand the rows.
    for id in seeded_ids {
        cleanup_geofence(&db, id).await;
    }

    assert_eq!(bbox_status, 200, "?bbox= must return 200");
    // Scope the response down to just this test's four known fence names, so
    // the assertion is exact regardless of whatever else lives in the shared
    // test DB (mirrors the contains/!contains pattern the sibling test above
    // uses for the same reason).
    let bbox_names: std::collections::BTreeSet<String> = feature_names(&bbox_body)
        .into_iter()
        .filter(|n| {
            [&inside_name, &touching_name, &outside_name, &asymmetric_name].contains(&n)
        })
        .collect();
    let expected: std::collections::BTreeSet<String> =
        [inside_name.clone(), touching_name.clone()].into_iter().collect();
    assert_eq!(
        bbox_names, expected,
        "?bbox=0,0,10,10 must return exactly {{inside, touching}} and exclude {{outside, asymmetric_axis}}"
    );
}

#[actix_web::test]
async fn geofences_ids_malformed_segment_returns_400() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences?ids=1,not-a-number")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "a non-numeric id segment must be 400");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "invalid_request");
}

#[actix_web::test]
async fn geofences_bbox_wrong_arity_returns_400() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/geofences?bbox=1,2,3")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400, "a 3-number bbox must be 400");
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
    assert!(
        location.starts_with("/api/v2/routes/"),
        "Location: {location}"
    );
    let route_created = body_json(route_resp).await;
    let route_id = route_created["data"]["id"].as_u64().expect("route id");

    // List
    let req = test::TestRequest::get().uri("/api/v2/routes").to_request();
    let list_resp = test::call_service(&app, req).await;
    assert_eq!(list_resp.status(), 200, "list routes must return 200");
    let list_body = body_json(list_resp).await;
    assert_eq!(list_body["status"], "ok");
    assert!(
        list_body["data"]["features"].is_array(),
        "routes list must be FeatureCollection"
    );

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

    let req = test::TestRequest::get().uri("/api/v2/jobs").to_request();
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
    assert_eq!(
        resp.status(),
        400,
        "DELETE with invalid job id must return 400"
    );
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
    assert!(
        v["data"]["clustering"].is_array(),
        "data.clustering must be an array"
    );
    assert!(
        v["data"]["routing"].is_array(),
        "data.routing must be an array"
    );
    assert!(
        v["data"]["bootstrap"].is_array(),
        "data.bootstrap must be an array"
    );
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

    let req = test::TestRequest::get().uri("/api/v2/plugins").to_request();
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
    assert_eq!(
        resp.status(),
        404,
        "known kind + unknown name must return 404"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn plugins_patch_unknown_kind_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::patch()
        .uri("/api/v2/plugins/badkind/anyname")
        .set_json(serde_json::json!({}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "PATCH with unknown kind must return 404"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn plugins_patch_known_kind_nonexistent_plugin_returns_422() {
    // Trying to PATCH a valid kind but a plugin not installed on disk → 422
    // (enforcement of the "drop a plugin.toml first" gate).
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::patch()
        .uri("/api/v2/plugins/clustering/plugin-that-does-not-exist-on-disk")
        .set_json(serde_json::json!({ "enabled": false }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        422,
        "patching non-installed plugin must return 422"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "unprocessable");
}

#[actix_web::test]
async fn plugins_delete_unknown_kind_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::delete()
        .uri("/api/v2/plugins/notakind/anyname")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "DELETE with unknown kind must return 404"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn plugins_delete_known_kind_nonexistent_plugin_returns_200_rows_0() {
    // Deleting an overlay that doesn't exist (no row in plugin_config) → 200
    // with rows_affected 0.  The handler does NOT return 404 — it treats it as
    // an idempotent no-op.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::delete()
        .uri("/api/v2/plugins/routing/plugin-that-has-no-overlay")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "DELETE with no overlay row must return 200"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["rows_affected"], 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// GEOFENCES — PATCH (update) + format query
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn geofences_patch_name_only_returns_200() {
    // FIXED: handler now fetches the existing row, merges the partial PATCH
    // fields over it, then calls upsert_json_return with the complete JSON —
    // so geometry is always present and partial PATCH returns 200.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("fence-patch");
    let updated_name = unique_name("fence-patch-updated");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": name, "geometry": triangle_geometry() }))
        .to_request();
    let created = body_json(test::call_service(&app, req).await).await;
    let id = created["data"]["id"].as_u64().expect("created id");

    // PATCH name only (no geometry) → after fix returns 200 + changed name
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/geofences/{id}"))
        .set_json(serde_json::json!({ "name": updated_name }))
        .to_request();
    let patch_resp = test::call_service(&app, req).await;
    let patch_status = patch_resp.status().as_u16();
    let patch_body = body_json(patch_resp).await;
    cleanup_geofence(&db, id).await;

    assert_eq!(
        patch_status, 200,
        "partial PATCH (name only) must return 200 after merge fix"
    );
    assert_eq!(patch_body["status"], "ok");
    assert_eq!(
        patch_body["data"]["name"], updated_name,
        "name must be updated"
    );
}

#[actix_web::test]
async fn geofences_patch_with_full_body_returns_200() {
    // Providing both `name` AND `geometry` works around the merge-bug above.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("fence-patch-full");
    let updated_name = unique_name("fence-patch-full-upd");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": name, "geometry": triangle_geometry() }))
        .to_request();
    let created = body_json(test::call_service(&app, req).await).await;
    let id = created["data"]["id"].as_u64().expect("created id");

    // PATCH with name + geometry (workaround) → 200
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/geofences/{id}"))
        .set_json(serde_json::json!({ "name": updated_name, "geometry": triangle_geometry() }))
        .to_request();
    let patch_resp = test::call_service(&app, req).await;
    let patch_status = patch_resp.status().as_u16();
    let patch_body = body_json(patch_resp).await;
    cleanup_geofence(&db, id).await;

    assert_eq!(patch_status, 200, "PATCH with full body must return 200");
    assert_eq!(patch_body["status"], "ok");
    assert_eq!(patch_body["data"]["name"], updated_name, "name not updated");
}

#[actix_web::test]
async fn geofences_patch_missing_id_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::patch()
        .uri("/api/v2/geofences/999999998")
        .set_json(serde_json::json!({ "name": "nobody" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "PATCH on missing geofence must return 404"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn geofences_get_format_feature_returns_geojson_feature() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("fence-fmt");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": name, "geometry": triangle_geometry() }))
        .to_request();
    let created = body_json(test::call_service(&app, req).await).await;
    let id = created["data"]["id"].as_u64().expect("created id");

    // GET with ?format=feature (default for get_one, but explicit here to cover the code path)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/geofences/{id}?format=feature"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    let get_status = get_resp.status().as_u16();
    let get_body = body_json(get_resp).await;
    cleanup_geofence(&db, id).await;

    assert_eq!(get_status, 200, "GET ?format=feature must return 200");
    assert_eq!(get_body["status"], "ok");
    assert_eq!(
        get_body["data"]["type"], "Feature",
        "data must be a GeoJSON Feature"
    );
    assert_eq!(get_body["data"]["properties"]["name"], name);
}

#[actix_web::test]
async fn geofences_list_format_featurecollection_is_default() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // No ?format= → default featurecollection
    let req = test::TestRequest::get()
        .uri("/api/v2/geofences")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert_eq!(v["data"]["type"], "FeatureCollection");
    assert!(v["data"]["features"].is_array());
}

// ═══════════════════════════════════════════════════════════════════════════
// ROUTES — PATCH + format query + 404 paths
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn routes_patch_name_only_returns_200() {
    // FIXED: handler now fetches the existing row, merges the partial PATCH
    // fields over it, then calls upsert_json_return with the complete JSON —
    // so name/geofence_id/geometry are always present and partial PATCH returns 200.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let fence_name = unique_name("fence-for-route-patch");
    let route_name = unique_name("route-patch");
    let updated_name = unique_name("route-patch-updated");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create geofence + route
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": fence_name, "geometry": triangle_geometry() }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await;
    let geofence_id = fence["data"]["id"].as_u64().expect("fence id");

    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(serde_json::json!({
            "name": route_name,
            "geofence_id": geofence_id,
            "geometry": route_geometry()
        }))
        .to_request();
    let route = body_json(test::call_service(&app, req).await).await;
    let route_id = route["data"]["id"].as_u64().expect("route id");

    // PATCH name only (no geometry) → after fix returns 200 + changed name
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/routes/{route_id}"))
        .set_json(serde_json::json!({ "name": updated_name }))
        .to_request();
    let patch_resp = test::call_service(&app, req).await;
    let patch_status = patch_resp.status().as_u16();
    let patch_body = body_json(patch_resp).await;

    cleanup_route(&db, route_id).await;
    cleanup_geofence(&db, geofence_id).await;

    assert_eq!(
        patch_status, 200,
        "partial PATCH (name only) must return 200 after merge fix"
    );
    assert_eq!(patch_body["status"], "ok");
    assert_eq!(
        patch_body["data"]["name"], updated_name,
        "name must be updated"
    );
}

#[actix_web::test]
async fn routes_patch_with_full_body_returns_200() {
    // Providing name + geofence_id + geometry works around the merge-bug.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let fence_name = unique_name("fence-for-route-patch-full");
    let route_name = unique_name("route-patch-full");
    let updated_name = unique_name("route-patch-full-upd");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create geofence + route
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": fence_name, "geometry": triangle_geometry() }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await;
    let geofence_id = fence["data"]["id"].as_u64().expect("fence id");

    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(serde_json::json!({
            "name": route_name,
            "geofence_id": geofence_id,
            "geometry": route_geometry()
        }))
        .to_request();
    let route = body_json(test::call_service(&app, req).await).await;
    let route_id = route["data"]["id"].as_u64().expect("route id");

    // PATCH with full body (workaround) → 200
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/routes/{route_id}"))
        .set_json(serde_json::json!({
            "name": updated_name,
            "geofence_id": geofence_id,
            "geometry": route_geometry()
        }))
        .to_request();
    let patch_resp = test::call_service(&app, req).await;
    let patch_status = patch_resp.status().as_u16();
    let patch_body = body_json(patch_resp).await;

    cleanup_route(&db, route_id).await;
    cleanup_geofence(&db, geofence_id).await;

    assert_eq!(
        patch_status, 200,
        "PATCH route with full body must return 200"
    );
    assert_eq!(patch_body["status"], "ok");
    assert_eq!(
        patch_body["data"]["name"], updated_name,
        "route name not updated"
    );
}

#[actix_web::test]
async fn routes_patch_missing_id_returns_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::patch()
        .uri("/api/v2/routes/999999998")
        .set_json(serde_json::json!({ "name": "ghost" }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "PATCH on missing route must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn routes_delete_returns_204_and_get_is_404() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let fence_name = unique_name("fence-for-route-del");
    let route_name = unique_name("route-del");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create geofence + route
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": fence_name, "geometry": triangle_geometry() }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await;
    let geofence_id = fence["data"]["id"].as_u64().expect("fence id");

    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(serde_json::json!({
            "name": route_name,
            "geofence_id": geofence_id,
            "geometry": route_geometry()
        }))
        .to_request();
    let route = body_json(test::call_service(&app, req).await).await;
    let route_id = route["data"]["id"].as_u64().expect("route id");

    // DELETE
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/routes/{route_id}"))
        .to_request();
    let del_resp = test::call_service(&app, req).await;
    assert_eq!(del_resp.status(), 204, "DELETE must return 204");

    // Subsequent GET → 404
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/routes/{route_id}"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    assert_eq!(get_resp.status(), 404, "deleted route must return 404");

    // Cleanup geofence (route already gone)
    cleanup_geofence(&db, geofence_id).await;
}

#[actix_web::test]
async fn routes_get_format_feature_returns_geojson_feature() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let fence_name = unique_name("fence-fmt-rt");
    let route_name = unique_name("route-fmt");

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create geofence + route
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({ "name": fence_name, "geometry": triangle_geometry() }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await;
    let geofence_id = fence["data"]["id"].as_u64().expect("fence id");

    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(serde_json::json!({
            "name": route_name,
            "geofence_id": geofence_id,
            "geometry": route_geometry()
        }))
        .to_request();
    let route = body_json(test::call_service(&app, req).await).await;
    let route_id = route["data"]["id"].as_u64().expect("route id");

    // GET with ?format=feature (the default for get_one)
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/routes/{route_id}?format=feature"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    let get_status = get_resp.status().as_u16();
    let get_body = body_json(get_resp).await;

    cleanup_route(&db, route_id).await;
    cleanup_geofence(&db, geofence_id).await;

    assert_eq!(get_status, 200, "GET route ?format=feature must return 200");
    assert_eq!(get_body["status"], "ok");
    assert_eq!(
        get_body["data"]["type"], "Feature",
        "data must be a GeoJSON Feature"
    );
    assert_eq!(get_body["data"]["properties"]["name"], route_name);
}

// ═══════════════════════════════════════════════════════════════════════════
// JOBS — enqueue happy path + get by id + cancel + serde-flatten BUG doc
// ═══════════════════════════════════════════════════════════════════════════

/// Cleanup helper: delete a job row by public_id (the ULID string returned by the API).
async fn cleanup_job(db: &DatabaseConnection, id: &str) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `job` WHERE `public_id` = ?",
            [Value::from(id)],
        ))
        .await;
}

#[actix_web::test]
async fn jobs_get_unknown_valid_ulid_returns_404() {
    // A well-formed ULID that doesn't exist in the DB → 404 (not 400).
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // A known-good ULID format that won't exist in the test DB.
    let req = test::TestRequest::get()
        .uri("/api/v2/jobs/01JXZZZZZZZZZZZZZZZZZZZZZZ")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "valid ULID not in DB must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn jobs_cancel_valid_ulid_not_in_db_returns_404() {
    // FIXED: the cancel handler now calls `jobs.get(id)` before cancelling;
    // a missing id returns 404 instead of the old silent 202.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::delete()
        .uri("/api/v2/jobs/01JXZZZZZZZZZZZZZZZZZZZZZZ")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404, "cancel on missing job must return 404");
    let v = body_json(resp).await;
    assert_eq!(v["status"], "error");
    assert_eq!(v["error"]["code"], "not_found");
}

#[actix_web::test]
async fn jobs_create_convert_job_returns_202_with_job_id() {
    // POST a minimal convert (cluster) job that has a valid area → 202 + job_id.
    // Cleanup the enqueued job row after asserting.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // A minimal cluster job with a small FeatureCollection area; no golbat data
    // needed for the enqueue path (data_points resolution hits the golbat DB,
    // but since we supply data_points directly the handler skips that branch).
    let body = serde_json::json!({
        "mode": "cluster",
        "category": "pokestop",
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
                }
            }]
        },
        "dataPoints": [[0.5, 0.5]]
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/jobs")
        .set_json(&body)
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let location = resp
        .headers()
        .get("Location")
        .map(|v| v.to_str().unwrap_or("").to_owned());
    let v = body_json(resp).await;

    // Clean up the job row before asserting so a panic doesn't strand it.
    if let Some(id) = v["data"]["job_id"].as_str() {
        cleanup_job(&db, id).await;
    }

    assert_eq!(status, 202, "enqueue cluster job must return 202, got {v}");
    assert_eq!(v["status"], "ok");
    let job_id = v["data"]["job_id"]
        .as_str()
        .expect("data.job_id must be a string");
    assert!(!job_id.is_empty(), "job_id must not be empty");
    assert!(
        location
            .as_deref()
            .is_some_and(|l| l.starts_with("/api/v2/jobs/")),
        "Location header must point at /api/v2/jobs/..., got {location:?}"
    );
}

#[actix_web::test]
async fn jobs_create_then_get_by_id_returns_200() {
    // Enqueue a job then GET /jobs/{id} → 200 with a matching job record.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let body = serde_json::json!({
        "mode": "cluster",
        "category": "pokestop",
        "area": {
            "type": "FeatureCollection",
            "features": [{
                "type": "Feature",
                "properties": {},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
                }
            }]
        },
        "dataPoints": [[0.5, 0.5]]
    });

    let req = test::TestRequest::post()
        .uri("/api/v2/jobs")
        .set_json(&body)
        .to_request();
    let create_resp = body_json(test::call_service(&app, req).await).await;
    let job_id = create_resp["data"]["job_id"]
        .as_str()
        .expect("data.job_id")
        .to_owned();

    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/jobs/{job_id}"))
        .to_request();
    let get_resp = test::call_service(&app, req).await;
    let get_status = get_resp.status().as_u16();
    let get_body = body_json(get_resp).await;

    cleanup_job(&db, &job_id).await;

    assert_eq!(get_status, 200, "GET job by id must return 200");
    assert_eq!(get_body["status"], "ok");
    // The record must echo the job id back.
    assert_eq!(
        get_body["data"]["id"].as_str().unwrap_or(""),
        job_id,
        "returned job id must match"
    );
}

#[actix_web::test]
async fn jobs_list_paginated_query_returns_200() {
    // FIXED: `JobListQuery` now has explicit `page: Option<i64>` and
    // `per_page: Option<i64>` fields instead of `#[serde(flatten)] Pagination`,
    // so `serde_urlencoded` can deserialize `?page=1&per_page=10` without error.
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let req = test::TestRequest::get()
        .uri("/api/v2/jobs?page=1&per_page=10")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        200,
        "paginated query must return 200 after serde flatten fix"
    );
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    assert!(v["meta"].is_object(), "response must include a meta block");
}
