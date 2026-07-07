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
    Database::connect(&url).await.ok().or_else(|| {
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
async fn build_test_koji_db(koji_db: DatabaseConnection) -> KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    KojiDb {
        koji: koji_db,
        golbat,
    }
}

/// Insert a geofence (needed as FK) and then a route under it.
async fn insert_geofence_and_route(
    db: &DatabaseConnection,
    route_name: &str,
    mode: &str,
) -> (u64, u64) {
    let fence_id = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO geofence (name, mode, geometry, created_at, updated_at) \
             VALUES (?, ?, '{\"type\":\"Polygon\",\"coordinates\":[[[0,0],[1,0],[1,1],[0,0]]]}', NOW(), NOW())",
            [Value::from(format!("fence-for-{route_name}")), Value::from("unset")],
        ))
        .await
        .unwrap()
        .last_insert_id();

    let route_id = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO route (geofence_id, name, mode, geometry, created_at, updated_at) \
             VALUES (?, ?, ?, '{\"type\":\"MultiPoint\",\"coordinates\":[[1,2],[3,4]]}', NOW(), NOW())",
            [Value::from(fence_id), Value::from(route_name.to_owned()), Value::from(mode)],
        ))
        .await
        .unwrap()
        .last_insert_id();

    (fence_id, route_id)
}

async fn cleanup_route(db: &DatabaseConnection, route_id: u64, fence_id: u64) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM route WHERE id = ?",
            [Value::from(route_id)],
        ))
        .await;
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "DELETE FROM geofence WHERE id = ?",
            [Value::from(fence_id)],
        ))
        .await;
}

#[actix_web::test]
async fn route_row_list_returns_row_shape_and_meta() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else {
        return;
    };
    let db = build_test_koji_db(conn.clone()).await;
    let name = unique_name("route-rows");
    let (fence_id, route_id) = insert_geofence_and_route(&conn, &name, "pokemon").await;

    let app = test::init_service(koji_service::test_internal_routes_app(db)).await;
    let req = test::TestRequest::get()
        .uri(&format!("/internal/routes?per_page=500&q={name}"))
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
    assert_eq!(row["id"], route_id);
    assert_eq!(row["mode"], "pokemon");
    assert!(row["geofence_id"].as_i64().unwrap() > 0);
    assert!(row["points"].as_u64().unwrap() >= 2);
    assert_eq!(v["meta"]["page"], 1);
    assert!(v["meta"]["total"].as_i64().unwrap() >= 1);
    cleanup_route(&conn, route_id, fence_id).await;
}

#[actix_web::test]
async fn route_row_list_middle_page_has_prev_and_has_next() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else {
        return;
    };
    let db = build_test_koji_db(conn.clone()).await;
    let tag = format!("mid-{}", JobId::new().as_string());
    let mut pairs = Vec::new();
    for suffix in ["a", "b", "c"] {
        pairs
            .push(insert_geofence_and_route(&conn, &format!("test-{tag}-{suffix}"), "unset").await);
    }

    let app = test::init_service(koji_service::test_internal_routes_app(db)).await;
    let req = test::TestRequest::get()
        .uri(&format!(
            "/internal/routes?per_page=1&page=2&q={tag}&sortBy=name&order=ASC"
        ))
        .to_request();
    let v = body_json(test::call_service(&app, req).await).await;
    assert_eq!(v["meta"]["page"], 2);
    assert_eq!(v["meta"]["total"], 3);
    assert_eq!(
        v["meta"]["has_prev"], true,
        "middle page must report has_prev=true"
    );
    assert_eq!(
        v["meta"]["has_next"], true,
        "middle page must report has_next=true"
    );
    for (fid, rid) in pairs {
        cleanup_route(&conn, rid, fid).await;
    }
}
