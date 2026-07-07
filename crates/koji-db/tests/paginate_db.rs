//! DB-integration tests for `paginate` on all five entity types (geofence,
//! route, project, property, tile_server) plus `PaginateResults::into_parts`.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test paginate_db -- --nocapture`

use koji_db::db::{geofence, project, property, route, tile_server};
use koji_db::query_args::AdminReqParsed;
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;
use tokio::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::const_new(());

async fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().await
}

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

fn unique_name(tag: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("kojitest-{tag}-{nanos}")
}

fn polygon_geometry() -> serde_json::Value {
    json!({
        "type": "Polygon",
        "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
    })
}

/// Build a minimal `AdminReqParsed` for page/per_page with an optional name
/// filter. `page` is 0-indexed.
fn page_args(page: u64, per_page: u64, q: &str) -> AdminReqParsed {
    AdminReqParsed {
        page,
        per_page,
        sort_by: "name".to_string(),
        order: "ASC".to_string(),
        q: q.to_string(),
        geotype: None,
        project: None,
        mode: None,
        parent: None,
        geofenceid: None,
        pointsmin: None,
        pointsmax: None,
    }
}

// ── PaginateResults::into_parts (pure, no DB needed) ─────────────────────────

/// `into_parts` exposes private fields. We exercise it by calling a real
/// `paginate` against an empty filter so the result is deterministic (the
/// inserted row is the cleanup target). This validates `into_parts` returns
/// the correct tuple shape and total is non-zero.
#[tokio::test]
async fn paginate_results_into_parts_returns_tuple() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("parts");
    let created = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("insert geofence");
    let id = created["id"].as_u64().unwrap() as u32;

    let result = geofence::Query::paginate(&db, page_args(0, 25, &name))
        .await
        .expect("paginate ok");

    geofence::Query::delete(&db, id).await.expect("cleanup");

    let (results, total, has_next, _has_prev) = result.into_parts();
    assert!(total >= 1, "total is at least 1");
    assert!(!has_next, "one result on page 0 has no next");
    assert!(!results.is_empty(), "results non-empty");
    assert!(
        results.iter().any(|r| r["name"] == json!(name)),
        "paginated results contain the inserted geofence"
    );
}

// ── geofence::paginate ────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_paginate_page1_empty_and_page0_has_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-page");
    let created = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("insert");
    let id = created["id"].as_u64().unwrap() as u32;

    // Page 0 with per_page=1 and exact name filter → 1 result, total = 1
    let page0 = geofence::Query::paginate(&db, page_args(0, 1, &name))
        .await
        .expect("paginate page 0");

    // Page 1 → empty (only 1 row matches, already exhausted by page 0)
    let page1 = geofence::Query::paginate(&db, page_args(1, 1, &name))
        .await
        .expect("paginate page 1");

    geofence::Query::delete(&db, id).await.expect("cleanup");

    let (p0_results, p0_total, p0_next, _) = page0.into_parts();
    assert_eq!(p0_total, 1, "total = 1 for exact match");
    assert!(!p0_results.is_empty(), "page 0 has the row");
    assert!(!p0_next, "no next page when total fits on page 0");

    let (p1_results, _p1_total, _, _) = page1.into_parts();
    assert!(
        p1_results.is_empty(),
        "page 1 is empty when only 1 row matches"
    );
}

#[tokio::test]
async fn geofence_paginate_name_filter_excludes_non_matching() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-filter");
    let created = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("insert");
    let id = created["id"].as_u64().unwrap() as u32;

    // Filter that can never match our row
    let none = geofence::Query::paginate(&db, page_args(0, 25, "zzz-impossible-999"))
        .await
        .expect("paginate with no-match filter");

    geofence::Query::delete(&db, id).await.expect("cleanup");

    let (results, total, _, _) = none.into_parts();
    assert_eq!(total, 0, "no rows match the impossible filter");
    assert!(results.is_empty());
}

// ── route::paginate ───────────────────────────────────────────────────────────

#[tokio::test]
async fn route_paginate_finds_inserted_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("fence-rpag");
    let route_name = unique_name("route-rpag");

    let fence = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": fence_name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("fence insert");
    let fence_id = fence["id"].as_u64().unwrap() as u32;

    let rt = route::Query::upsert_json_return(
        &db,
        0,
        json!({
            "name": route_name,
            "geofence_id": fence_id,
            "mode": "pokemon",
            "geometry": { "type": "MultiPoint", "coordinates": [[1.0,2.0],[3.0,4.0]] }
        }),
    )
    .await
    .expect("route insert");
    let route_id = rt["id"].as_u64().unwrap() as u32;

    let page = route::Query::paginate(&db, page_args(0, 25, &route_name))
        .await
        .expect("paginate");

    route::Query::delete(&db, route_id)
        .await
        .expect("del route");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    let (results, total, _, _) = page.into_parts();
    assert_eq!(total, 1, "one route matches the name filter");
    assert!(
        results.iter().any(|r| r["name"] == json!(route_name)),
        "route appears in page results"
    );
}

#[tokio::test]
async fn route_paginate_geofenceid_filter_works() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("fence-rflt");
    let route_name = unique_name("route-rflt");

    let fence = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": fence_name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("fence");
    let fence_id = fence["id"].as_u64().unwrap() as u32;

    let rt = route::Query::upsert_json_return(
        &db,
        0,
        json!({
            "name": route_name,
            "geofence_id": fence_id,
            "mode": "unset",
            "geometry": { "type": "MultiPoint", "coordinates": [[0.0,0.0]] }
        }),
    )
    .await
    .expect("route");
    let route_id = rt["id"].as_u64().unwrap() as u32;

    // Filter by the specific geofence_id
    let mut args = page_args(0, 25, "");
    args.geofenceid = Some(fence_id);
    let page = route::Query::paginate(&db, args).await.expect("paginate");

    route::Query::delete(&db, route_id)
        .await
        .expect("del route");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    let (results, total, _, _) = page.into_parts();
    assert!(total >= 1, "at least one route under this geofence");
    assert!(
        results.iter().any(|r| r["name"] == json!(route_name)),
        "route under the filtered geofence appears"
    );
}

// ── project::paginate ─────────────────────────────────────────────────────────

#[tokio::test]
async fn project_paginate_finds_inserted_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("proj-pag");
    let created =
        project::Query::upsert_json_return(&db, 0, json!({ "name": name }))
            .await
            .expect("insert");
    let id = created["id"].as_u64().unwrap() as u32;

    let page = project::Query::paginate(&db, page_args(0, 25, &name))
        .await
        .expect("paginate");

    project::Query::delete(&db, id).await.expect("cleanup");

    let (results, total, _, _) = page.into_parts();
    assert_eq!(total, 1, "exactly one project matches the name filter");
    assert!(
        results.iter().any(|r| r["name"] == json!(name)),
        "project in page results"
    );
    // paginate includes related geofences array (empty here)
    assert!(
        results[0].get("geofences").is_some(),
        "project page result has geofences field"
    );
}

// ── property::paginate ────────────────────────────────────────────────────────

#[tokio::test]
async fn property_paginate_finds_inserted_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("prop-pag");
    let created = property::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "category": "string", "default_value": "hello" }),
    )
    .await
    .expect("insert");
    let id = created["id"].as_u64().unwrap() as u32;

    let page = property::Query::paginate(&db, page_args(0, 25, &name))
        .await
        .expect("paginate");

    property::Query::delete(&db, id).await.expect("cleanup");

    let (results, total, _, _) = page.into_parts();
    assert_eq!(total, 1, "exactly one property matches the name filter");
    assert!(
        results.iter().any(|r| r["name"] == json!(name)),
        "property in page results"
    );
    // paginate includes related geofences array
    assert!(
        results[0].get("geofences").is_some(),
        "property page result has geofences field"
    );
}

// ── tile_server::paginate ─────────────────────────────────────────────────────

#[tokio::test]
async fn tile_server_paginate_finds_inserted_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("tiles-pag");
    let created = tile_server::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "url": "https://tiles.test/{z}/{x}/{y}.png" }),
    )
    .await
    .expect("insert");
    let id = created["id"].as_u64().unwrap() as u32;

    let page = tile_server::Query::paginate(&db, page_args(0, 25, &name))
        .await
        .expect("paginate");

    tile_server::Query::delete(&db, id).await.expect("cleanup");

    let (results, total, _, _) = page.into_parts();
    assert_eq!(total, 1, "exactly one tile_server matches");
    assert!(
        results.iter().any(|r| r["name"] == json!(name)),
        "tile_server in page results"
    );
}
