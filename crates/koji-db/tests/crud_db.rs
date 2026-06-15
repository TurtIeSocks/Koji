//! DB-integration tests for the koji-db `Query` CRUD round-trips, run against a
//! live MySQL `koji_test` (already migrated). These are the first DB-backed
//! tests for this crate — everything else here is pure.
//!
//! ## Gating (no `#[ignore]`)
//! Each test starts `let Some(db) = test_db().await else { return };`.
//! `test_db()` reads `KOJI_DB_URL` and connects, or prints a skip note and
//! returns `None` when the var is missing — so a plain `cargo test` (no env)
//! passes by skipping. With the env sourced the tests hit the DB:
//!
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-db --test crud_db -- --nocapture
//! ```
//!
//! ## Coverage
//! create → read (`get_one_json`) → list (`get_json_cache`) → delete →
//! read-errors, for: **project**, **tile_server**, **property**, and
//! **geofence** (with a small valid GeoJSON polygon). `paginate` is exercised
//! indirectly elsewhere; its `PaginateResults` fields are private, so the list
//! assertion here uses the public `get_json_cache` cache path instead.
//!
//! ## Isolation
//! Names are suffixed with a nanosecond timestamp so parallel + repeat runs
//! never collide; the four tests also hit four different tables. Each gathers
//! its observations, then deletes the row(s) it created via the typed
//! `Query::delete` (geofence children cascade), then asserts — so even a failing
//! assertion can't leak a row. `koji_test` stays clean and re-runnable.

use koji_db::db::{geofence, project, property, tile_server};
use sea_orm::{Database, DatabaseConnection};
use serde_json::json;

/// Connect to the koji DB iff `KOJI_DB_URL` is set; otherwise skip (return
/// `None`) so the suite passes with no env.
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

/// A unique name for one test row — `kojitest-<tag>-<nanos>`. Nanos keep this
/// crate dep-free (no ulid) while still avoiding collisions across parallel +
/// repeat runs.
fn unique_name(tag: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("kojitest-{tag}-{nanos}")
}

#[tokio::test]
async fn project_crud_round_trip() {
    let Some(db) = test_db().await else { return };
    let name = unique_name("project");

    // create (upsert with id=0 inserts)
    let created = project::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "scanner": false, "description": "integration test" }),
    )
    .await
    .expect("project upsert should insert");
    let id = created["id"].as_u64().expect("created project has an id") as u32;

    // Gather every observation FIRST, then delete, then assert — so a failing
    // assertion can never leak the row (`#[tokio::test]` has no teardown hook).
    let got = project::Query::get_one_json(&db, id.to_string()).await;
    let cache = project::Query::get_json_cache(&db).await;

    project::Query::delete(&db, id).await.expect("delete");
    let after = project::Query::get_one(&db, id.to_string()).await;

    // read (get_one_json)
    let got = got.expect("get_one_json should find the project");
    assert_eq!(got["name"], json!(name), "name round-trips through get_one");
    assert_eq!(got["scanner"], json!(false));

    // list (get_json_cache contains it)
    let cache = cache.expect("get_json_cache");
    assert!(
        cache.iter().any(|p| p["name"] == json!(name)),
        "get_json_cache should contain the new project"
    );

    // read after delete now errors (NotFound-shaped)
    assert!(
        after.is_err(),
        "get_one after delete should error (does not exist)"
    );
}

#[tokio::test]
async fn tile_server_crud_round_trip() {
    let Some(db) = test_db().await else { return };
    let name = unique_name("tiles");

    let created = tile_server::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "url": "https://tiles.example.com/{z}/{x}/{y}.png" }),
    )
    .await
    .expect("tile_server upsert should insert");
    let id = created["id"].as_u64().expect("created tile_server has an id") as u32;

    // Gather, then delete, then assert (panic-safe cleanup — see project test).
    let got = tile_server::Query::get_one_json(&db, id.to_string()).await;
    let cache = tile_server::Query::get_json_cache(&db).await;

    tile_server::Query::delete(&db, id).await.expect("delete");
    let after = tile_server::Query::get_one(&db, id.to_string()).await;

    let got = got.expect("get_one_json");
    assert_eq!(got["name"], json!(name));
    assert_eq!(got["url"], json!("https://tiles.example.com/{z}/{x}/{y}.png"));

    let cache = cache.expect("get_json_cache");
    assert!(
        cache.iter().any(|t| t["name"] == json!(name)),
        "get_json_cache should contain the new tile_server"
    );

    assert!(after.is_err(), "get_one after delete should error");
}

#[tokio::test]
async fn property_crud_round_trip() {
    let Some(db) = test_db().await else { return };
    let name = unique_name("prop");

    // `category` must be one of the Category enum strings; "string" is simplest.
    let created = property::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "category": "string", "default_value": "hello" }),
    )
    .await
    .expect("property upsert should insert");
    let id = created["id"].as_u64().expect("created property has an id") as u32;

    // Gather, then delete, then assert (panic-safe cleanup — see project test).
    let got = property::Query::get_one_json(&db, id.to_string()).await;
    let cache = property::Query::get_json_cache(&db).await;

    property::Query::delete(&db, id).await.expect("delete");
    let after = property::Query::get_one(&db, id.to_string()).await;

    let got = got.expect("get_one_json");
    assert_eq!(got["name"], json!(name));
    assert_eq!(got["category"], json!("string"), "category round-trips");

    let cache = cache.expect("get_json_cache");
    assert!(
        cache.iter().any(|p| p["name"] == json!(name)),
        "get_json_cache should contain the new property"
    );

    assert!(after.is_err(), "get_one after delete should error");
}

#[tokio::test]
async fn geofence_crud_round_trip() {
    let Some(db) = test_db().await else { return };
    let name = unique_name("fence");

    // A small valid GeoJSON Polygon geometry (closed ring, lon/lat order).
    let geometry = json!({
        "type": "Polygon",
        "coordinates": [[
            [0.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [1.0, 0.0],
            [0.0, 0.0]
        ]]
    });

    // create — geofence.upsert with id=0 inserts a fresh row keyed by unique name.
    let created = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": geometry }),
    )
    .await
    .expect("geofence upsert should insert");
    let id = created["id"].as_u64().expect("created geofence has an id") as u32;

    // Gather every observation FIRST, then delete, then assert — a failed
    // assertion must never leak the geofence (this is the row that *did* leak
    // before this restructure; children cascade on delete).
    let got = geofence::Query::get_one_json(&db, id.to_string()).await;
    let koji = geofence::Query::get_one_koji(&db, id.to_string()).await;

    geofence::Query::delete(&db, id).await.expect("delete");
    let after = geofence::Query::get_one(&db, id.to_string()).await;

    // read (get_one_json) — name + geometry present
    let got = got.expect("get_one_json");
    assert_eq!(got["name"], json!(name), "name round-trips");
    assert_eq!(
        got["geometry"]["type"],
        json!("Polygon"),
        "stored geometry is a Polygon"
    );

    // read as a KojiGeometry (the typed geometry projection). The inner
    // geo-types `Geometry` is the `Polygon` variant. (`KojiGeometry` isn't
    // `Serialize`, so we assert on its `Debug` shape — dep-free. geo-types
    // Debug-prints the WKT form, e.g. `POLYGON((0 0, ...))`, so match that
    // case-insensitively.)
    let koji = koji.expect("get_one_koji should project the geometry");
    let dbg = format!("{:?}", koji.geometry);
    assert!(
        dbg.to_lowercase().contains("polygon"),
        "get_one_koji yields a Polygon geometry, got {dbg}"
    );

    // read after delete now errors
    assert!(after.is_err(), "get_one after delete should error");
}
