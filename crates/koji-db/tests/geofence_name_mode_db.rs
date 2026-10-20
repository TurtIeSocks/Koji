//! DB-integration tests for geofences keyed by `(name, mode)`.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test geofence_name_mode_db -- --nocapture`

use koji_core::UnknownId;
use koji_db::ModelError;
use koji_db::db::import::{ImportAction, ImportItem, ImportKind, OnCollision, import};
use koji_db::db::sea_orm_active_enums::Mode;
use koji_db::db::{geofence, route};
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, Set};
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

async fn make_fence(db: &DatabaseConnection, name: &str, mode: &str) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({ "name": name, "mode": mode, "geometry": polygon_geometry() }),
    )
    .await
    .expect("make_fence");
    created["id"].as_u64().expect("has id") as u32
}

async fn cleanup(db: &DatabaseConnection, ids: &[u32]) {
    for id in ids {
        geofence::Query::delete(db, *id).await.expect("delete");
    }
}

fn is_ambiguous(err: &ModelError, name: &str, modes: &[&str]) -> bool {
    matches!(err, ModelError::AmbiguousGeofence { name: n, modes: m }
        if n == name && m == modes)
}

// ── the key ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn same_name_saves_in_two_modes_and_a_repeat_pair_is_refused() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("boston");

    let insert = |mode: Mode| {
        geofence::ActiveModel {
            name: Set(name.clone()),
            mode: Set(mode),
            geometry: Set(polygon_geometry()),
            ..Default::default()
        }
        .insert(&db)
    };
    let unset = insert(Mode::Unset).await.expect("Boston/unset saves");
    let pokemon = insert(Mode::Pokemon).await.expect("Boston/pokemon saves");
    let repeat = insert(Mode::Pokemon).await;

    cleanup(&db, &[unset.id, pokemon.id]).await;
    assert!(repeat.is_err(), "a second Boston/pokemon must be refused");
}

#[tokio::test]
async fn upsert_keys_on_the_pair_so_each_mode_keeps_its_own_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("upsert-pair");

    let unset = make_fence(&db, &name, "unset").await;
    let pokemon = make_fence(&db, &name, "pokemon").await;
    let pokemon_again = make_fence(&db, &name, "pokemon").await;

    cleanup(&db, &[unset, pokemon]).await;
    assert_ne!(unset, pokemon, "a new mode is a new row");
    assert_eq!(pokemon, pokemon_again, "the same pair updates in place");
}

// ── lookups by name alone ───────────────────────────────────────────────────

#[tokio::test]
async fn get_one_by_shared_name_is_ambiguous_until_given_a_mode() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("get-one");
    let unset = make_fence(&db, &name, "unset").await;
    let pokemon = make_fence(&db, &name, "pokemon").await;

    let bare = geofence::Query::get_one(&db, name.clone()).await;
    let picked = geofence::Query::get_one_in_mode(&db, name.clone(), Some(Mode::Pokemon)).await;
    let by_id = geofence::Query::get_one_in_mode(&db, unset.to_string(), Some(Mode::Pokemon)).await;

    cleanup(&db, &[unset, pokemon]).await;
    let err = bare.expect_err("a name shared by two modes is ambiguous");
    assert!(is_ambiguous(&err, &name, &["pokemon", "unset"]), "{err:?}");
    assert_eq!(picked.expect("mode picks one").id, pokemon);
    assert_eq!(by_id.expect("a numeric id ignores mode").id, unset);
}

#[tokio::test]
async fn by_parent_koji_with_a_shared_parent_name_is_ambiguous() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("parent");
    let unset = make_fence(&db, &name, "unset").await;
    let pokemon = make_fence(&db, &name, "pokemon").await;
    let child = make_fence(&db, &unique_name("child"), "pokemon").await;
    geofence::Query::set_parent(&db, child, Some(pokemon))
        .await
        .unwrap();

    let by_name = geofence::Query::by_parent_koji(&db, &UnknownId::String(name.clone())).await;
    let by_id = geofence::Query::by_parent_koji(&db, &UnknownId::Number(pokemon)).await;

    cleanup(&db, &[child, unset, pokemon]).await;
    let err = by_name.expect_err("a parent name shared by two modes is ambiguous");
    assert!(is_ambiguous(&err, &name, &["pokemon", "unset"]), "{err:?}");
    assert_eq!(by_id.expect("by id still works").items.len(), 1);
}

#[tokio::test]
async fn renaming_one_of_two_same_name_fences_renames_only_its_routes() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("rename");
    let renamed = unique_name("renamed");
    let unset = make_fence(&db, &name, "unset").await;
    let pokemon = make_fence(&db, &name, "pokemon").await;
    let mut route_ids = vec![];
    for fence in [unset, pokemon] {
        let created = route::Query::upsert_json_return(
            &db,
            0,
            json!({
                "name": name,
                "geofence_id": fence,
                "mode": "pokemon",
                "geometry": { "type": "MultiPoint", "coordinates": [[1.0,2.0]] }
            }),
        )
        .await
        .expect("route upsert");
        route_ids.push(created["id"].as_u64().unwrap() as u32);
    }

    geofence::Query::upsert_json_return(
        &db,
        pokemon,
        json!({ "name": renamed, "mode": "pokemon", "geometry": polygon_geometry() }),
    )
    .await
    .expect("rename");
    let unset_route = route::Query::get_one(&db, route_ids[0].to_string()).await;
    let pokemon_route = route::Query::get_one(&db, route_ids[1].to_string()).await;

    cleanup(&db, &[unset, pokemon]).await;
    assert_eq!(
        unset_route.unwrap().name,
        name,
        "the other fence's route keeps its name"
    );
    assert_eq!(pokemon_route.unwrap().name, renamed);
}

// ── import ──────────────────────────────────────────────────────────────────

fn fence_item(name: &str, mode: &str) -> ImportItem {
    ImportItem {
        kind: ImportKind::Geofence,
        name: name.to_string(),
        geometry: polygon_geometry(),
        mode: Some(mode.to_string()),
        parent: None,
        projects: vec![],
        route_parent: None,
        on_collision: OnCollision::Skip,
    }
}

#[tokio::test]
async fn import_keeps_each_mode_and_upserts_by_the_pair() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;
    let name = unique_name("import");

    let first = import(
        &db,
        vec![fence_item(&name, "unset"), fence_item(&name, "pokemon")],
        false,
    )
    .await
    .unwrap();
    assert!(first.committed, "{:?}", first.results);
    let ids: Vec<u32> = first.results.iter().filter_map(|o| o.id).collect();

    // Retry: both pairs now exist, so both skip rather than duplicate or fail.
    let retry = import(
        &db,
        vec![fence_item(&name, "unset"), fence_item(&name, "pokemon")],
        false,
    )
    .await
    .unwrap();
    // A third mode under the same name is a new fence, not a collision.
    let third = import(&db, vec![fence_item(&name, "fort")], false)
        .await
        .unwrap();

    let mut all = ids.clone();
    all.extend(third.results.iter().filter_map(|o| o.id));
    cleanup(&db, &all).await;

    assert_eq!(ids.len(), 2);
    assert!(retry.results.iter().all(|o| o.action == ImportAction::Skip));
    assert_eq!(third.results[0].action, ImportAction::Create);
}
