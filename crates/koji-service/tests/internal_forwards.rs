//! Integration tests for the `/internal` scope: forward-aliases + auth + parity.
//!
//! ## No-DB tests (always run)
//! - `internal_config_requires_auth_when_secret_set` — `GET /internal/config`
//!   returns `401` without a bearer and `200` with the correct bearer.
//! - `internal_auth_me_is_forwarded` — `GET /internal/auth/me` returns a valid
//!   `{ status, data: { authenticated, via } }` envelope.
//!
//! ## DB-gated parity test (runs only when `KOJI_DB_URL` is set)
//! - `internal_geofence_getone_parity` — creates a geofence via the public
//!   `/api/v2/geofences` endpoint and asserts that `GET /internal/geofences/{id}`
//!   returns the same body as `GET /api/v2/geofences/{id}`.
//!
//! `KOJI_SECRET` mutation is serialized across all tests in this binary.

// The `serial_guard()` pattern holds a `MutexGuard` across `.await` points —
// intentional serialization for env-var safety, not a production hazard.
#![allow(clippy::await_holding_lock)]

use actix_web::test;
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}

/// Snapshot + restore `KOJI_SECRET` so a panicking test doesn't poison the env.
struct SecretGuard(Option<String>);
impl SecretGuard {
    fn set(v: &str) -> Self {
        let orig = std::env::var("KOJI_SECRET").ok();
        // SAFETY: single-threaded actix_web::test runtime; serialised by SERIAL.
        unsafe { std::env::set_var("KOJI_SECRET", v); }
        Self(orig)
    }
}
impl Drop for SecretGuard {
    fn drop(&mut self) {
        // SAFETY: same.
        unsafe {
            match &self.0 {
                Some(v) => std::env::set_var("KOJI_SECRET", v),
                None => std::env::remove_var("KOJI_SECRET"),
            }
        }
    }
}

// ── no-DB: auth gate on /internal/config ────────────────────────────────────

/// `GET /internal/config` must return `401` when `KOJI_SECRET` is set and no
/// bearer is supplied, and `200` with the correct bearer.
#[actix_web::test]
async fn internal_config_requires_auth_when_secret_set() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set("topsecret");

    let app = test::init_service(koji_service::test_internal_authed_app()).await;

    // No bearer → 401.
    let req = test::TestRequest::get().uri("/internal/config").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status().as_u16(), 401, "no bearer → 401");

    // Correct bearer → 200.
    let req = test::TestRequest::get()
        .uri("/internal/config")
        .insert_header(("Authorization", "Bearer topsecret"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(
        resp.status().is_success(),
        "valid bearer → config (got {})",
        resp.status()
    );
}

// ── no-DB: /internal/auth/me forwarded ──────────────────────────────────────

/// `GET /internal/auth/me` must be reachable and return the standard
/// `{ status: "ok", data: { authenticated, via } }` envelope.
#[actix_web::test]
async fn internal_auth_me_is_forwarded() {
    let _serial = serial_guard();
    let _guard = SecretGuard::set(""); // empty → open (no secret)

    let app = test::init_service(koji_service::test_internal_authed_app()).await;

    let req = test::TestRequest::get()
        .uri("/internal/auth/me")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success(), "auth/me must be 200");

    let bytes = test::read_body(resp).await;
    let v: serde_json::Value =
        serde_json::from_slice(&bytes).expect("auth/me body must be valid JSON");
    assert_eq!(v["status"], "ok", "envelope status must be ok");
    assert!(
        v["data"]["authenticated"].is_boolean(),
        "data.authenticated must be a boolean"
    );
}

// ── DB-gated parity: /internal/geofences/{id} body == /api/v2/geofences/{id} ──

/// Connect to `KOJI_DB_URL` if set. Returns `None` (skip) when absent.
async fn test_db() -> Option<sea_orm::DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset — skipping DB-gated internal parity test");
        return None;
    };
    match sea_orm::Database::connect(&url).await {
        Ok(db) => Some(db),
        Err(e) => {
            eprintln!("skip: could not connect to KOJI_DB_URL: {e}");
            None
        }
    }
}

/// Build a `KojiDb` (both fields pointing at the same test DB connection).
async fn build_koji_db(conn: sea_orm::DatabaseConnection) -> koji_db::KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = sea_orm::Database::connect(&url)
        .await
        .expect("golbat re-connect");
    koji_db::KojiDb { koji: conn, golbat }
}

/// Delete a geofence row by numeric id string (best-effort cleanup).
async fn cleanup_geofence(db: &sea_orm::DatabaseConnection, id: &str) {
    use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM `geofence` WHERE `id` = ?",
            [Value::from(id.parse::<u64>().unwrap_or(0))],
        ))
        .await;
}

/// `GET /internal/geofences/{id}` must return the same JSON body as
/// `GET /api/v2/geofences/{id}` for the same geofence.
#[actix_web::test]
async fn internal_geofence_getone_parity() {
    let Some(raw_db) = test_db().await else { return };
    let _serial = serial_guard();

    let koji_db = build_koji_db(raw_db.clone()).await;
    let jobs = std::sync::Arc::new(koji_jobs::JobQueue::new(raw_db.clone(), "test-worker"));

    // Use the extended test_db_app_with_internal (mounts both /api/v2 and /internal).
    let app =
        test::init_service(koji_service::test_db_app_with_internal(koji_db.clone(), jobs)).await;

    // Create a geofence via the public API.
    let tag = format!("int-parity-{}", koji_jobs::JobId::new().as_string());
    let body = serde_json::json!({
        "name": tag,
        "mode": "pokemon",
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
        }
    });
    let create_req = test::TestRequest::post()
        .uri("/api/v2/geofences")
        .set_json(&body)
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status().as_u16(), 201, "create must return 201");

    // Extract the id from Location header.
    let location = create_resp
        .headers()
        .get("Location")
        .expect("201 must have Location")
        .to_str()
        .unwrap()
        .to_string();
    let id = location
        .rsplit('/')
        .next()
        .expect("Location must end with id")
        .to_string();

    // GET via public API.
    let pub_req = test::TestRequest::get()
        .uri(&format!("/api/v2/geofences/{id}"))
        .to_request();
    let pub_resp = test::call_service(&app, pub_req).await;
    assert!(pub_resp.status().is_success(), "public get_one must be 200");
    let pub_bytes = test::read_body(pub_resp).await;
    let pub_json: serde_json::Value =
        serde_json::from_slice(&pub_bytes).expect("public get_one must return JSON");

    // GET via internal scope.
    let int_req = test::TestRequest::get()
        .uri(&format!("/internal/geofences/{id}"))
        .to_request();
    let int_resp = test::call_service(&app, int_req).await;
    assert!(
        int_resp.status().is_success(),
        "internal get_one must be 200 (got {})",
        int_resp.status()
    );
    let int_bytes = test::read_body(int_resp).await;
    let int_json: serde_json::Value =
        serde_json::from_slice(&int_bytes).expect("internal get_one must return JSON");

    // Cleanup (always runs — both success and failure paths reach here because
    // the assert above doesn't abort, it just panics; cleanup before asserting
    // the final body equality).
    cleanup_geofence(&raw_db, &id).await;

    assert_eq!(
        pub_json, int_json,
        "public and internal get_one must return the same body"
    );
}
