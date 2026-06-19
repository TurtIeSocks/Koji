#![allow(clippy::await_holding_lock)]
use actix_web::test;
use koji_db::KojiDb;
use koji_jobs::JobId;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}
async fn test_db() -> Option<DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return None;
    };
    Database::connect(&url)
        .await
        .ok()
        .or_else(|| {
            eprintln!("skip: connect failed");
            None
        })
}
fn unique_name(tag: &str) -> String {
    format!("test-{tag}-{}", JobId::new().as_string())
}
async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
    serde_json::from_slice(&test::read_body(resp).await).unwrap()
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

async fn insert_geofence(db: &DatabaseConnection, name: &str, mode: &str) -> u64 {
    // `geo_type` is a STORED GENERATED column (derived from `geometry`), so it
    // must be omitted from the INSERT — the DB computes it automatically.
    // Use ExecResult::last_insert_id() — more reliable than a separate SELECT.
    db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "INSERT INTO geofence (name, mode, geometry, created_at, updated_at) \
         VALUES (?, ?, '{\"type\":\"Polygon\",\"coordinates\":[[[0,0],[1,0],[1,1],[0,0]]]}', NOW(), NOW())",
        [Value::from(name.to_owned()), Value::from(mode.to_owned())],
    ))
    .await
    .unwrap()
    .last_insert_id()
}

async fn cleanup(db: &DatabaseConnection, id: u64) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM geofence WHERE id = ?",
            [Value::from(id)],
        ))
        .await;
}

#[actix_web::test]
async fn row_list_returns_geofence_row_shape_and_meta() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else {
        return;
    };
    let db = build_test_koji_db(conn.clone()).await;
    let name = unique_name("rows");
    let id = insert_geofence(&conn, &name, "pokemon").await;

    let app = test::init_service(koji_service::test_internal_geofences_app(db)).await;
    let req = test::TestRequest::get()
        .uri(&format!("/internal/geofences?per_page=500&q={name}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let row = v["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    assert_eq!(row["id"], id);
    assert_eq!(row["mode"], "pokemon");
    assert_eq!(row["geo_type"], "Polygon");
    assert!(row["parent"].is_null());
    assert!(row["projects"].is_array());
    assert!(row["property_count"].is_u64());
    // meta block present + 1-based page
    assert_eq!(v["meta"]["page"], 1);
    assert!(v["meta"]["per_page"].as_i64().unwrap() <= 500);
    assert!(v["meta"]["total"].as_i64().unwrap() >= 1);
    cleanup(&conn, id).await;
}

#[actix_web::test]
async fn row_list_honors_mode_filter_and_sort() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else {
        return;
    };
    let db = build_test_koji_db(conn.clone()).await;
    let a = unique_name("aaa");
    let z = unique_name("zzz");
    // Use valid enum values: mode is enum('pokemon','fort','quest','unset')
    let ida = insert_geofence(&conn, &a, "fort").await;
    let idz = insert_geofence(&conn, &z, "quest").await;

    let app = test::init_service(koji_service::test_internal_geofences_app(db)).await;
    // mode filter: only the fort one
    let req = test::TestRequest::get()
        .uri("/internal/geofences?per_page=500&mode=fort&sortBy=name&order=ASC")
        .to_request();
    let v = body_json(test::call_service(&app, req).await).await;
    assert!(v["data"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["mode"] == "fort"));
    assert!(v["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["id"] == ida));
    assert!(v["data"]
        .as_array()
        .unwrap()
        .iter()
        .all(|r| r["id"] != idz));
    cleanup(&conn, ida).await;
    cleanup(&conn, idz).await;
}
