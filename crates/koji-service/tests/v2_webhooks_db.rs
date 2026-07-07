//! DB-backed integration tests for the v2 `/api/v2/webhooks` resource (6th
//! `koji_resource!`).
// The `serial_guard()` pattern intentionally holds a `MutexGuard` across `.await`
// points to serialise concurrent test execution — safe in test context only.
#![allow(clippy::await_holding_lock)]
//! These run against a **live** MySQL `koji_test` database.
//!
//! ## Gating
//! Every test starts with `let Some(db) = test_db().await else { return };`.
//! `test_db()` reads `KOJI_DB_URL`; if unset it prints a skip note and returns
//! `None`. A plain `cargo test -p koji-service` **passes by skipping**.
//!
//! ## Running with the DB
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-service --test v2_webhooks_db -- --nocapture
//! ```

use actix_web::test;
use koji_db::KojiDb;
use koji_jobs::{JobId, JobQueue};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
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

/// Build a `KojiDb` pointing both `koji` and `golbat` at the same `KOJI_DB_URL`
/// (golbat is used only for reads; a standalone test DB is fine as a dummy).
async fn build_test_koji_db(koji_db: DatabaseConnection) -> KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    KojiDb {
        koji: koji_db,
        golbat,
    }
}

/// Minimal GeoJSON polygon (a triangle at (0,0)) — mirrors `v2_db.rs`'s helper
/// (test binaries don't share modules, so it's duplicated here).
fn triangle_geometry() -> serde_json::Value {
    serde_json::json!({
        "type": "Polygon",
        "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
    })
}

/// A route `MultiPoint` geometry — mirrors `v2_db.rs`'s helper.
fn route_geometry() -> serde_json::Value {
    serde_json::json!({
        "type": "MultiPoint",
        "coordinates": [[0.0,0.0],[1.0,1.0]]
    })
}

/// Delete a `geofence` row by id (best-effort cleanup).
async fn cleanup_geofence(db: &DatabaseConnection, id: u64) {
    use sea_orm::{ConnectionTrait, Statement};
    let _ = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "DELETE FROM `geofence` WHERE `id` = ?",
            [id.into()],
        ))
        .await;
}

/// Delete a `route` row by id (best-effort cleanup).
async fn cleanup_route(db: &DatabaseConnection, id: u64) {
    use sea_orm::{ConnectionTrait, Statement};
    let _ = db
        .execute(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "DELETE FROM `route` WHERE `id` = ?",
            [id.into()],
        ))
        .await;
}

// ═══════════════════════════════════════════════════════════════════════════
// WEBHOOKS — full CRUD cycle + ?project= filter + parity canary
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn webhooks_full_crud_cycle_and_project_filter() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("hook");
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create a project to scope against.
    let project_name = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": project_name}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let project_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // ── POST /api/v2/webhooks → 201 + Location ─────────────────────────
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(serde_json::json!({
            "name": name, "url": "http://127.0.0.1:1/reload",
            "mode": "ping", "method": "POST",
            "project_id": project_id,
            "headers": {"x-golbat-secret": "abc"}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201, "create webhook must return 201");
    let location = resp
        .headers()
        .get("Location")
        .expect("Location header missing");
    let location = location.to_str().unwrap();
    assert!(
        location.starts_with("/api/v2/webhooks/"),
        "Location: {location}"
    );
    let created = body_json(resp).await;
    let id = created["data"]["id"].as_u64().unwrap();

    // ── GET one (by id) ────────────────────────────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let got = body_json(resp).await;
    assert_eq!(got["data"]["mode"], "ping");
    assert_eq!(got["data"]["method"], "POST");
    assert_eq!(got["data"]["headers"]["x-golbat-secret"], "abc");
    assert_eq!(got["data"]["project_id"], project_id);
    assert_eq!(got["data"]["topics"], serde_json::json!([]));

    // ── LIST with ?project= filter ─────────────────────────────────────
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/webhooks?project={project_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let list = body_json(resp).await;
    assert!(
        list["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["id"].as_u64() == Some(id))
    );
    let req = test::TestRequest::get()
        .uri("/api/v2/webhooks?project=999999999")
        .to_request();
    let resp = test::call_service(&app, req).await;
    let list = body_json(resp).await;
    assert!(list["data"].as_array().unwrap().is_empty());

    // ── PATCH ──────────────────────────────────────────────────────────
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .set_json(serde_json::json!({"active": false}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let patched = body_json(resp).await;
    assert_eq!(patched["data"]["active"], false);
    assert_eq!(
        patched["data"]["url"], "http://127.0.0.1:1/reload",
        "PATCH must not clobber other fields"
    );

    // ── Parity canary: koji-events entity reads the same row ──────────
    let ev = koji_events::entity::webhook_subscription::Entity::find_by_id(id)
        .one(&db)
        .await
        .unwrap()
        .expect("koji-events entity must read the row koji-db wrote");
    assert_eq!(ev.mode, "ping");
    assert_eq!(ev.method, "POST");
    assert_eq!(ev.project_id, Some(project_id as u32));

    // ── DELETE → 204; project cascade cleanup ──────────────────────────
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204);
    // Cleanup the project (cascade already exercised implicitly elsewhere).
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
}

#[actix_web::test]
async fn webhook_cascade_dies_with_project() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("hook-cascade");
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Create a project to scope against.
    let project_name = unique_name("proj-cascade");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": project_name}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let project_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // Create a webhook scoped to that project.
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(serde_json::json!({
            "name": name, "url": "http://127.0.0.1:1/reload",
            "mode": "ping", "method": "POST",
            "project_id": project_id,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // Delete the project — the webhook row must cascade-delete with it.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 204, "project delete must return 204");

    // The webhook must now be gone.
    let req = test::TestRequest::get()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        404,
        "webhook must be cascade-deleted with its project"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// WEBHOOK TEST-FIRE — POST /api/v2/webhooks/{id}/test (synchronous single fire)
// ═══════════════════════════════════════════════════════════════════════════

// ═══════════════════════════════════════════════════════════════════════════
// PROJECT OUTBOX — PATCH/DELETE emit project.updated/project.deleted
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn project_patch_and_delete_emit_outbox_events() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    let name = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": name}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let new_name = unique_name("proj2");
    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .set_json(serde_json::json!({"name": new_name}))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);

    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 204);

    // Outbox rows exist for both topics with the project id in payload.
    use sea_orm::{ConnectionTrait, Statement};
    let fetch = |topic: &'static str| {
        let db = db.clone();
        async move {
            db.query_all(Statement::from_sql_and_values(
                sea_orm::DbBackend::MySql,
                "SELECT CAST(payload AS CHAR) AS p FROM event_outbox WHERE topic = ? \
                 AND JSON_EXTRACT(payload, '$.projectId') = ?",
                [topic.into(), project_id.into()],
            ))
            .await
            .unwrap()
        }
    };
    let updated = fetch("project.updated").await;
    let deleted = fetch("project.deleted").await;
    // Cleanup outbox rows before asserting (panic-safe).
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE JSON_EXTRACT(payload, '$.projectId') = ?",
        [project_id.into()],
    ))
    .await
    .unwrap();
    assert!(!updated.is_empty(), "PATCH must emit project.updated");
    assert!(!deleted.is_empty(), "DELETE must emit project.deleted");
}

// ═══════════════════════════════════════════════════════════════════════════
// GEOFENCE/ROUTE OUTBOX — create + update emit geofence.updated/route.updated
// with projectIds[] resolved from geofence_project
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn geofence_update_emits_event_with_project_ids() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // project + geofence linked to it (geofence create accepts "projects").
    let pname = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": pname}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let gname = unique_name("fence");
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": gname, "mode": "pokemon", "geometry": triangle_geometry(),
            "projects": [project_id]
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let geofence_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    // The create itself must have emitted geofence.updated w/ projectIds.
    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox WHERE topic = 'geofence.updated' \
             AND JSON_EXTRACT(payload, '$.geofenceId') = ?",
            [geofence_id.into()],
        ))
        .await
        .unwrap();
    // Cleanup (fence cascade-cleans geofence_project; outbox rows by hand).
    cleanup_geofence(&db, geofence_id).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE JSON_EXTRACT(payload, '$.geofenceId') = ?",
        [geofence_id.into()],
    ))
    .await
    .unwrap();

    assert!(!rows.is_empty(), "geofence create must emit geofence.updated");
    let payload: serde_json::Value =
        serde_json::from_str(rows[0].try_get::<String>("", "p").unwrap().as_str()).unwrap();
    assert!(
        payload["projectIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(project_id)),
        "payload must carry linked projectIds: {payload}"
    );
    assert_eq!(
        payload["name"],
        serde_json::json!(gname),
        "payload must pin the geofence name (guards the record[\"name\"] path)"
    );
}

#[actix_web::test]
async fn route_update_emits_event_with_project_ids() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // project + geofence linked to it, then a route under that geofence.
    let pname = unique_name("proj");
    let req = test::TestRequest::post()
        .uri("/api/v2/projects")
        .set_json(serde_json::json!({"name": pname}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let gname = unique_name("fence-for-route");
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": gname, "mode": "pokemon", "geometry": triangle_geometry(),
            "projects": [project_id]
        }))
        .to_request();
    let geofence_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let rname = unique_name("route");
    let req = test::TestRequest::post()
        .uri("/api/v2/routes")
        .set_json(serde_json::json!({
            "name": rname, "geofence_id": geofence_id,
            "mode": "pokemon", "geometry": route_geometry()
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 201);
    let route_id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox WHERE topic = 'route.updated' \
             AND JSON_EXTRACT(payload, '$.routeId') = ?",
            [route_id.into()],
        ))
        .await
        .unwrap();

    // Cleanup: route, then geofence (FK), then project, then outbox rows.
    cleanup_route(&db, route_id).await;
    cleanup_geofence(&db, geofence_id).await;
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE JSON_EXTRACT(payload, '$.routeId') = ?",
        [route_id.into()],
    ))
    .await
    .unwrap();

    assert!(!rows.is_empty(), "route create must emit route.updated");
    let payload: serde_json::Value =
        serde_json::from_str(rows[0].try_get::<String>("", "p").unwrap().as_str()).unwrap();
    assert_eq!(payload["geofenceId"].as_u64(), Some(geofence_id));
    assert!(
        payload["projectIds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_u64() == Some(project_id)),
        "payload must carry linked projectIds via parent geofence: {payload}"
    );
    assert_eq!(
        payload["name"],
        serde_json::json!(rname),
        "payload must pin the route name (guards the record[\"name\"] path)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// PROJECT MEMBERSHIP OUTBOX — project.geofences_changed (Task 8)
// ═══════════════════════════════════════════════════════════════════════════

#[actix_web::test]
async fn geofence_membership_change_emits_geofences_changed() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;

    // Two projects; fence starts in A, moves to B.
    let mut ids = vec![];
    for _ in 0..2 {
        let req = test::TestRequest::post()
            .uri("/api/v2/projects")
            .set_json(serde_json::json!({"name": unique_name("proj")}))
            .to_request();
        ids.push(
            body_json(test::call_service(&app, req).await).await["data"]["id"]
                .as_u64()
                .unwrap(),
        );
    }
    let (a, b) = (ids[0], ids[1]);
    let req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(serde_json::json!({
            "name": unique_name("fence"), "mode": "pokemon",
            "geometry": triangle_geometry(), "projects": [a]
        }))
        .to_request();
    let fence = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let req = test::TestRequest::patch()
        .uri(&format!("/api/v2/geofences/{fence}"))
        .set_json(serde_json::json!({"projects": [b]}))
        .to_request();
    assert_eq!(test::call_service(&app, req).await.status(), 200);

    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox \
             WHERE topic = 'project.geofences_changed' \
             AND JSON_EXTRACT(payload, '$.projectId') IN (?, ?)",
            [a.into(), b.into()],
        ))
        .await
        .unwrap();

    // Cleanup: fence, projects, outbox rows.
    cleanup_geofence(&db, fence).await;
    for pid in [a, b] {
        let req = test::TestRequest::delete()
            .uri(&format!("/api/v2/projects/{pid}"))
            .to_request();
        test::call_service(&app, req).await;
    }
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE topic = 'project.geofences_changed' \
         AND JSON_EXTRACT(payload, '$.projectId') IN (?, ?)",
        [a.into(), b.into()],
    ))
    .await
    .unwrap();

    let payloads: Vec<serde_json::Value> = rows
        .iter()
        .map(|r| serde_json::from_str(r.try_get::<String>("", "p").unwrap().as_str()).unwrap())
        .collect();
    // A lost the fence, B gained it (from the PATCH; the create also emitted one for A).
    assert!(
        payloads.iter().any(|p| p["projectId"].as_u64() == Some(a)
            && p["removedIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(fence))),
        "project A must see the fence in removedIds: {payloads:?}"
    );
    assert!(
        payloads.iter().any(|p| p["projectId"].as_u64() == Some(b)
            && p["addedIds"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v.as_u64() == Some(fence))),
        "project B must see the fence in addedIds: {payloads:?}"
    );
}

#[actix_web::test]
async fn bulk_import_emits_one_geofences_changed_event_per_project() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app_with_internal(koji_db, jobs)).await;

    // One project; import 3 geofences all linked to it in a single request.
    // (`test_db_app_with_internal` doesn't mount plain `/api/v2/projects` —
    // only `/internal/projects`, which forwards to the same resource.)
    let req = test::TestRequest::post()
        .uri("/internal/projects")
        .set_json(serde_json::json!({"name": unique_name("proj")}))
        .to_request();
    let project_id = body_json(test::call_service(&app, req).await).await["data"]["id"]
        .as_u64()
        .unwrap();

    let names: Vec<String> = (0..3).map(|_| unique_name("import-fence")).collect();
    let items: Vec<serde_json::Value> = names
        .iter()
        .map(|n| {
            serde_json::json!({
                "kind": "geofence",
                "name": n,
                "geometry": triangle_geometry(),
                "mode": "pokemon",
                "projects": [project_id]
            })
        })
        .collect();
    let req = test::TestRequest::post()
        .uri("/internal/import")
        .set_json(serde_json::json!({"dry_run": false, "items": items}))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);
    let result = body_json(resp).await;
    assert_eq!(result["data"]["summary"]["create"], 3);

    use sea_orm::{ConnectionTrait, Statement};
    let rows = db
        .query_all(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "SELECT CAST(payload AS CHAR) AS p FROM event_outbox \
             WHERE topic = 'project.geofences_changed' \
             AND JSON_EXTRACT(payload, '$.projectId') = ?",
            [project_id.into()],
        ))
        .await
        .unwrap();

    // Cleanup: fences (by name, since ids weren't captured individually), project, outbox.
    for n in &names {
        db.execute(Statement::from_sql_and_values(
            sea_orm::DbBackend::MySql,
            "DELETE FROM `geofence` WHERE `name` = ?",
            [n.clone().into()],
        ))
        .await
        .unwrap();
    }
    let req = test::TestRequest::delete()
        .uri(&format!("/internal/projects/{project_id}"))
        .to_request();
    test::call_service(&app, req).await;
    db.execute(Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM event_outbox WHERE topic = 'project.geofences_changed' \
         AND JSON_EXTRACT(payload, '$.projectId') = ?",
        [project_id.into()],
    ))
    .await
    .unwrap();

    assert_eq!(
        rows.len(),
        1,
        "bulk import of 3 fences into one project must emit exactly ONE project.geofences_changed row, not N: {rows:?}"
    );
    let payload: serde_json::Value =
        serde_json::from_str(rows[0].try_get::<String>("", "p").unwrap().as_str()).unwrap();
    assert_eq!(
        payload["addedIds"].as_array().unwrap().len(),
        3,
        "addedIds must carry all 3 imported fence ids: {payload}"
    );
}

#[actix_web::test]
async fn webhook_test_endpoint_404_on_missing() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks/999999999/test")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn webhook_test_endpoint_fires_ping_at_live_listener() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();

    // Real listener on an OS-assigned port; records method + headers.
    use std::sync::{Arc as StdArc, Mutex as StdMutex};
    #[derive(Clone, Default)]
    #[allow(clippy::type_complexity)]
    struct Seen(StdArc<StdMutex<Vec<(String, Option<String>, Option<String>)>>>);
    let seen = Seen::default();
    let seen_c = seen.clone();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = actix_web::HttpServer::new(move || {
        let seen = seen_c.clone();
        actix_web::App::new().default_service(actix_web::web::to(
            move |req: actix_web::HttpRequest| {
                let seen = seen.clone();
                async move {
                    let hdr = |n: &str| {
                        req.headers().get(n).and_then(|v| v.to_str().ok()).map(String::from)
                    };
                    seen.0.lock().unwrap().push((
                        req.method().to_string(),
                        hdr("x-golbat-secret"),
                        hdr("X-Koji-Event-Id"),
                    ));
                    actix_web::HttpResponse::Ok().finish()
                }
            },
        ))
    })
    .listen(listener)
    .unwrap()
    .workers(1)
    .run();
    let handle = server.handle();
    tokio::spawn(server);

    let koji_db = build_test_koji_db(db.clone()).await;
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app(koji_db, jobs)).await;
    let name = unique_name("hook");
    let req = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(serde_json::json!({
            "name": name, "url": format!("http://127.0.0.1:{port}/reload"),
            "mode": "ping", "method": "POST",
            "headers": {"x-golbat-secret": "abc"}
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let id = body_json(resp).await["data"]["id"].as_u64().unwrap();

    let req = test::TestRequest::post()
        .uri(&format!("/api/v2/webhooks/{id}/test"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    let status = resp.status().as_u16();
    let body = body_json(resp).await;

    // Cleanup before asserting.
    let req = test::TestRequest::delete()
        .uri(&format!("/api/v2/webhooks/{id}"))
        .to_request();
    test::call_service(&app, req).await;
    handle.stop(true).await;

    assert_eq!(status, 200);
    assert_eq!(body["data"]["delivered"], true);
    assert_eq!(body["data"]["upstream_status"], 200);
    let seen = seen.0.lock().unwrap();
    assert_eq!(seen.len(), 1, "exactly one ping");
    assert_eq!(seen[0].0, "POST", "honors method");
    assert_eq!(seen[0].1.as_deref(), Some("abc"), "custom header sent");
    assert!(seen[0].2.is_some(), "X-Koji-Event-Id sent");
}
