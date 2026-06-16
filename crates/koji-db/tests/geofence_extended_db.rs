//! Extended DB-integration tests for geofence, geofence_project, and plugin_config.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test geofence_extended_db -- --nocapture`

use koji_db::db::{geofence, geofence_project, plugin_config, project};
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

async fn make_geofence(db: &DatabaseConnection, name: &str) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_geometry() }),
    )
    .await
    .expect("make_geofence");
    created["id"].as_u64().expect("has id") as u32
}

async fn make_project(db: &DatabaseConnection, name: &str) -> u32 {
    let created =
        project::Query::upsert_json_return(db, 0, json!({ "name": name, "scanner": false }))
            .await
            .expect("make_project");
    created["id"].as_u64().expect("has id") as u32
}

// ── geofence — get_one_json_with_related ────────────────────────────────────

#[tokio::test]
async fn geofence_get_one_with_related_has_routes_and_projects_fields() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-related");
    let fence_id = make_geofence(&db, &name).await;

    let got = geofence::Query::get_one_json_with_related(&db, fence_id.to_string()).await;

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete");

    let got = got.expect("get_one_json_with_related ok");
    assert_eq!(got["name"], json!(name));
    // Related arrays present even when empty.
    assert!(got.get("projects").is_some(), "projects field present");
    assert!(got.get("routes").is_some(), "routes field present");
    assert!(got.get("properties").is_some(), "properties field present");
}

// ── geofence — search ───────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_search_finds_by_name_substring() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-search");
    let fence_id = make_geofence(&db, &name).await;

    // search for a unique substring of the name
    let hits = geofence::Query::search(&db, name.clone())
        .await
        .expect("search");

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete");

    assert!(
        hits.iter().any(|r| r["name"] == json!(name)),
        "search finds the geofence"
    );
}

#[tokio::test]
async fn geofence_search_no_match_returns_empty() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let hits = geofence::Query::search(&db, "zzz-impossible-99999999999".to_string())
        .await
        .expect("search ok");
    assert!(hits.is_empty());
}

// ── geofence — get_all_no_fences (json_cache) ───────────────────────────────

#[tokio::test]
async fn geofence_json_cache_contains_new_geofence() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-cache");
    let fence_id = make_geofence(&db, &name).await;

    let cache = geofence::Query::get_json_cache(&db).await.expect("cache");

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete");

    assert!(
        cache.iter().any(|r| r["name"] == json!(name)),
        "json_cache contains the new geofence"
    );
    // Geometry column is NOT present in the no_fences projection.
    for r in &cache {
        assert!(r.get("geometry").is_none(), "cache entry has no geometry");
    }
}

// ── geofence — upsert (update path) ─────────────────────────────────────────

#[tokio::test]
async fn geofence_upsert_update_changes_mode() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let name = unique_name("gf-upd");
    let fence_id = make_geofence(&db, &name).await;

    // update by providing the real id and a different mode
    let updated = geofence::Query::upsert_json_return(
        &db,
        fence_id,
        json!({ "name": name, "mode": "fort", "geometry": polygon_geometry() }),
    )
    .await
    .expect("upsert update");

    // read back
    let got = geofence::Query::get_one_json(&db, fence_id.to_string())
        .await
        .expect("get after update");

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete");

    assert_eq!(updated["id"], json!(fence_id), "same row id");
    assert_eq!(got["mode"], json!("fort"), "mode updated");
}

// ── geofence — not found ─────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_get_one_not_found_by_id_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let result = geofence::Query::get_one(&db, "999999999".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn geofence_get_one_not_found_by_name_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let result = geofence::Query::get_one(&db, "zzz-impossible-geofence-99999".to_string()).await;
    assert!(result.is_err());
}

// ── geofence_project — upsert_related_by_geofence_id ────────────────────────

#[tokio::test]
async fn geofence_project_upsert_related_by_geofence_id() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-proj-gfid");
    let proj_name = unique_name("proj-for-gfid");

    let fence_id = make_geofence(&db, &fence_name).await;
    let proj_id = make_project(&db, &proj_name).await;

    // Link geofence → project
    geofence_project::Query::upsert_related_by_geofence_id(&db, &[json!(proj_id)], fence_id)
        .await
        .expect("upsert by geofence_id");

    // Verify link is reflected in the geofence's projects list
    let with_related = geofence::Query::get_one_json_with_related(&db, fence_id.to_string())
        .await
        .expect("get_one_with_related");

    let projects: Vec<u32> = with_related["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n as u32))
        .collect();

    // Unlink (pass empty list)
    geofence_project::Query::upsert_related_by_geofence_id(&db, &[], fence_id)
        .await
        .expect("unlink");

    // cleanup
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("delete project");

    assert!(
        projects.contains(&proj_id),
        "project_id appears in geofence's related projects"
    );
}

// ── geofence_project — upsert_related_by_project_id ─────────────────────────

#[tokio::test]
async fn geofence_project_upsert_related_by_project_id() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-proj-prid");
    let proj_name = unique_name("proj-for-prid");

    let fence_id = make_geofence(&db, &fence_name).await;
    let proj_id = make_project(&db, &proj_name).await;

    // Link project → geofence (mirror side)
    geofence_project::Query::upsert_related_by_project_id(&db, &[json!(fence_id)], proj_id)
        .await
        .expect("upsert by project_id");

    // Verify via the all() list
    let all = geofence_project::Query::get_all(&db)
        .await
        .expect("get_all");
    let found = all
        .iter()
        .any(|r| r.geofence_id == fence_id && r.project_id == proj_id);

    // Unlink
    geofence_project::Query::upsert_related_by_project_id(&db, &[], proj_id)
        .await
        .expect("unlink");

    // cleanup
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("delete project");

    assert!(found, "join row exists after upsert_related_by_project_id");
}

// ── geofence_project — delete ────────────────────────────────────────────────

#[tokio::test]
async fn geofence_project_delete_removes_link() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-del-link");
    let proj_name = unique_name("proj-del-link");

    let fence_id = make_geofence(&db, &fence_name).await;
    let proj_id = make_project(&db, &proj_name).await;

    // create link
    geofence_project::Query::create(
        &db,
        geofence_project::Model {
            id: 0,
            geofence_id: fence_id,
            project_id: proj_id,
        },
    )
    .await
    .expect("create link");

    // delete just the link (not the fence or project)
    let del = geofence_project::Query::delete(&db, Some(fence_id), Some(proj_id)).await;

    // cleanup
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("delete fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("delete project");

    del.expect("delete link ok");
}

#[tokio::test]
async fn geofence_project_delete_both_none_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let result = geofence_project::Query::delete(&db, None, None).await;
    assert!(result.is_err(), "delete with both None should error");
}

// ── plugin_config — insert, upsert (update), get_one, all, delete ────────────

#[tokio::test]
async fn plugin_config_full_lifecycle() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let kind = unique_name("kind");
    let name = unique_name("plugin");

    // insert
    let created = plugin_config::Query::upsert(
        &db,
        &kind,
        &name,
        Some(true),
        Some(json!({"threshold": 10})),
        Some("test plugin".to_string()),
    )
    .await
    .expect("plugin_config insert");
    assert!(created.enabled);
    assert_eq!(
        created.args_default.as_ref().unwrap()["threshold"],
        json!(10)
    );
    assert_eq!(created.description.as_deref(), Some("test plugin"));

    // get_one — present
    let got = plugin_config::Query::get_one(&db, &kind, &name)
        .await
        .expect("get_one ok");
    assert!(got.is_some(), "get_one finds the row");

    // all — contains our row
    let all = plugin_config::Query::all(&db).await.expect("all ok");
    let found = all.iter().any(|r| r.kind == kind && r.name == name);

    // update (upsert again — existing row)
    let updated = plugin_config::Query::upsert(
        &db,
        &kind,
        &name,
        Some(false), // flip enabled
        None,        // leave args_default
        None,        // leave description
    )
    .await
    .expect("plugin_config update");

    // delete
    let del = plugin_config::Query::delete(&db, &kind, &name)
        .await
        .expect("delete ok");

    // get_one — absent after delete
    let after = plugin_config::Query::get_one(&db, &kind, &name)
        .await
        .expect("get_one after delete ok");

    assert!(found, "all() contains the new plugin_config");
    assert!(!updated.enabled, "enabled updated to false");
    assert_eq!(del.rows_affected, 1, "one row deleted");
    assert!(after.is_none(), "get_one after delete returns None");
}

// ── plugin_config — get_one missing ─────────────────────────────────────────

#[tokio::test]
async fn plugin_config_get_one_missing_returns_none() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let result = plugin_config::Query::get_one(&db, "zzz-kind", "zzz-name")
        .await
        .expect("get_one ok");
    assert!(result.is_none(), "get_one of missing plugin returns None");
}
