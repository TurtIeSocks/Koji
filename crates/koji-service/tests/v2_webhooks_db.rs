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
        .set_json(serde_json::json!({"name": project_name, "golbat": false}))
        .to_request();
    // NOTE: after Task 9 lands, drop the `golbat` field from this body.
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
        // NOTE: after Task 9 lands, drop the `golbat` field from this body.
        .set_json(serde_json::json!({"name": project_name, "golbat": false}))
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
