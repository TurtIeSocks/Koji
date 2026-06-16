//! DB-integration tests for `route::Query` CRUD round-trips.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test route_db -- --nocapture`

use koji_db::db::{geofence, route};
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;
use tokio::sync::{Mutex, MutexGuard};

/// Process-wide serialisation guard — route tests share the route table.
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

/// Small polygon geometry reused across tests.
fn polygon_geometry() -> serde_json::Value {
    json!({
        "type": "Polygon",
        "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0],[0.0,0.0]]]
    })
}

/// Create a throw-away geofence and return its id. Must be deleted by the caller.
async fn make_geofence(db: &DatabaseConnection, name: &str) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("make_geofence: upsert failed");
    created["id"].as_u64().expect("has id") as u32
}

// ── route crud round-trip ────────────────────────────────────────────────────

#[tokio::test]
async fn route_crud_round_trip() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("fence-for-route");
    let route_name = unique_name("route");
    let fence_id = make_geofence(&db, &fence_name).await;

    // create via upsert (id = 0 → insert)
    let created = route::Query::upsert_json_return(
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
    .expect("route upsert insert");
    let id = created["id"].as_u64().expect("route has id") as u32;

    // gather observations
    let got = route::Query::get_one_json(&db, id.to_string()).await;
    let cache = route::Query::get_json_cache(&db).await;
    let search_hits = route::Query::search(&db, route_name.clone()).await;

    // cleanup
    route::Query::delete(&db, id).await.expect("route delete");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("geofence delete");

    let after = route::Query::get_one(&db, id.to_string()).await;

    // assert
    let got = got.expect("get_one_json finds the route");
    assert_eq!(got["name"], json!(route_name));
    assert_eq!(got["geofence_id"], json!(fence_id));

    let cache = cache.expect("get_json_cache ok");
    assert!(
        cache.iter().any(|r| r["name"] == json!(route_name)),
        "cache contains new route"
    );

    let search_hits = search_hits.expect("search ok");
    assert!(
        search_hits.iter().any(|r| r["name"] == json!(route_name)),
        "search finds the route by name"
    );

    assert!(after.is_err(), "get_one after delete should error");
}

// ── route update round-trip ──────────────────────────────────────────────────

#[tokio::test]
async fn route_update_round_trip() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("fence-for-update");
    let route_name = unique_name("route-upd");
    let fence_id = make_geofence(&db, &fence_name).await;

    let created = route::Query::upsert_json_return(
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
    .expect("route insert");
    let id = created["id"].as_u64().expect("id") as u32;

    // update via upsert with the real id
    let updated_name = format!("{}-v2", route_name);
    let updated = route::Query::upsert_json_return(
        &db,
        id,
        json!({
            "name": updated_name,
            "geofence_id": fence_id,
            "mode": "pokemon",
            "geometry": { "type": "MultiPoint", "coordinates": [[5.0,6.0],[7.0,8.0]] }
        }),
    )
    .await
    .expect("route update");

    // cleanup before assert
    route::Query::delete(&db, id).await.expect("delete");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("geofence delete");

    assert_eq!(updated["name"], json!(updated_name));
    assert_eq!(updated["id"], json!(id)); // same row
}

// ── route not-found ──────────────────────────────────────────────────────────

#[tokio::test]
async fn route_get_one_not_found_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    // id 0 will never exist in a valid dataset
    let result = route::Query::get_one(&db, "999999999".to_string()).await;
    assert!(
        result.is_err(),
        "get_one of non-existent route should error"
    );
}

// ── route by_geofence ────────────────────────────────────────────────────────
//
// by_geofence(name) resolves via the GEOFENCE name (left-join geofence table),
// not the route's own name column.
// - numeric arg       → filter by Column::GeofenceId (route found)
// - geofence name arg → left_join geofence, filter geofence.name (route found)
// - route name arg    → should NOT return anything (negative assertion)

#[tokio::test]
async fn route_by_geofence_id_returns_related_routes() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("fence-by-gf");
    let route_name = unique_name("route-by-gf");
    let fence_id = make_geofence(&db, &fence_name).await;

    let created = route::Query::upsert_json_return(
        &db,
        0,
        json!({
            "name": route_name,
            "geofence_id": fence_id,
            "mode": "unset",
            "geometry": { "type": "MultiPoint", "coordinates": [[1.0,1.0]] }
        }),
    )
    .await
    .expect("route insert");
    let route_id = created["id"].as_u64().expect("id") as u32;

    // by numeric geofence id → filter by GeofenceId column
    let by_id = route::Query::by_geofence(&db, fence_id.to_string())
        .await
        .expect("by_geofence by id");

    // by geofence name → left-join geofence table, filter geofence.name
    let by_fence_name = route::Query::by_geofence(&db, fence_name.clone())
        .await
        .expect("by_geofence by fence name");

    // by route name (different from fence name) → should NOT match
    let by_route_name = route::Query::by_geofence(&db, route_name.clone())
        .await
        .expect("by_geofence by route name (should be empty)");

    // cleanup
    route::Query::delete(&db, route_id)
        .await
        .expect("delete route");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete geofence");

    assert!(
        by_id.iter().any(|r| r["name"] == json!(route_name)),
        "by_geofence(numeric fence id) returns the route"
    );
    assert!(
        by_fence_name.iter().any(|r| r["name"] == json!(route_name)),
        "by_geofence(geofence name) returns the route under that geofence"
    );
    assert!(
        !by_route_name.iter().any(|r| r["name"] == json!(route_name)),
        "by_geofence(route name) must NOT return the route — it resolves geofence names"
    );
}

// ── route search empty ───────────────────────────────────────────────────────

#[tokio::test]
async fn route_search_no_match_returns_empty() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    // Use a name that cannot exist in the db (UUID-like, very long).
    let hits = route::Query::search(&db, "zzz-impossible-route-name-99999999999999".to_string())
        .await
        .expect("search ok");
    assert!(hits.is_empty(), "search with no match returns empty vec");
}
