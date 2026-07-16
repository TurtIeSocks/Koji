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
//! create → read (`get_one_json`) → list (`search`) → delete →
//! read-errors, for: **project**, **tile_server**, **property**, and
//! **geofence** (with a small valid GeoJSON polygon). `paginate` is exercised
//! indirectly elsewhere; its `PaginateResults` fields are private, so the list
//! assertion here uses the public `search` path instead.
//!
//! ## Isolation
//! Names are suffixed with a nanosecond timestamp so parallel + repeat runs
//! never collide; the four tests also hit four different tables. Each gathers
//! its observations, then deletes the row(s) it created via the typed
//! `Query::delete` (geofence children cascade), then asserts — so even a failing
//! assertion can't leak a row. `koji_test` stays clean and re-runnable.

use koji_db::db::{geofence, geofence_project, project, property, tile_server};
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
        json!({ "name": name, "description": "integration test" }),
    )
    .await
    .expect("project upsert should insert");
    let id = created["id"].as_u64().expect("created project has an id") as u32;

    // Gather every observation FIRST, then delete, then assert — so a failing
    // assertion can never leak the row (`#[tokio::test]` has no teardown hook).
    let got = project::Query::get_one_json(&db, id.to_string()).await;
    let listed = project::Query::search(&db, name.clone()).await;

    project::Query::delete(&db, id).await.expect("delete");
    let after = project::Query::get_one(&db, id.to_string()).await;

    // read (get_one_json)
    let got = got.expect("get_one_json should find the project");
    assert_eq!(got["name"], json!(name), "name round-trips through get_one");
    assert_eq!(got["description"], json!("integration test"));

    // list (search finds it)
    let listed = listed.expect("search");
    assert!(
        listed.iter().any(|p| p["name"] == json!(name)),
        "search should find the new project"
    );

    // read after delete now errors (NotFound-shaped)
    assert!(
        after.is_err(),
        "get_one after delete should error (does not exist)"
    );
}

/// Regression for the project show/edit bug: the list page counted
/// `record.geofences.length` off `paginate`'s hand-built `"geofences"` array,
/// but `get_one_json` (show/edit's GET) returned the bare `project::Model`
/// with no `geofences` key at all, so show/edit rendered nothing. Verifies
/// `get_one_json` AND `upsert_json_return` both carry the linked geofence ids
/// (the array-of-ids shape `ReferenceArrayInput`/`ReferenceArrayField`
/// expect — not `paginate`'s `[{id,name}]` object shape).
#[tokio::test]
async fn project_get_one_json_includes_linked_geofences() {
    let Some(db) = test_db().await else { return };
    let project_name = unique_name("project-geofences");
    let fence1_name = unique_name("fence-a");
    let fence2_name = unique_name("fence-b");

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

    let created_project =
        project::Query::upsert_json_return(&db, 0, json!({ "name": project_name }))
            .await
            .expect("project upsert should insert");
    let project_id = created_project["id"]
        .as_u64()
        .expect("created project has an id") as u32;

    let created_fence1 = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": fence1_name, "mode": "unset", "geometry": geometry }),
    )
    .await
    .expect("geofence 1 upsert should insert");
    let fence1_id = created_fence1["id"]
        .as_u64()
        .expect("created geofence 1 has an id") as u32;

    let created_fence2 = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": fence2_name, "mode": "unset", "geometry": geometry }),
    )
    .await
    .expect("geofence 2 upsert should insert");
    let fence2_id = created_fence2["id"]
        .as_u64()
        .expect("created geofence 2 has an id") as u32;

    geofence_project::Query::upsert_related_by_project_id(
        &db,
        &[json!(fence1_id), json!(fence2_id)],
        project_id,
    )
    .await
    .expect("link geofences to project");

    // Gather every observation FIRST, then delete, then assert (panic-safe
    // cleanup — see project_crud_round_trip above). `upsert_json_return` is
    // re-run with no `geofences` key in the body, mirroring what a PATCH that
    // only touches `name`/`description` sends — `upsert_related_geofences`
    // no-ops when the key is absent, so the pre-existing links must survive
    // AND the return value must still carry them.
    let got = project::Query::get_one_json(&db, project_id.to_string()).await;
    let upserted_again =
        project::Query::upsert_json_return(&db, project_id, json!({ "name": project_name })).await;

    // Cleanup: geofences first (their delete cascades the geofence_project
    // link rows via the FK restored in m20260714_000002), then the project.
    geofence::Query::delete(&db, fence1_id)
        .await
        .expect("delete fence1");
    geofence::Query::delete(&db, fence2_id)
        .await
        .expect("delete fence2");
    project::Query::delete(&db, project_id)
        .await
        .expect("delete project");

    let got = got.expect("get_one_json should succeed");
    let ids: std::collections::HashSet<u64> = got["geofences"]
        .as_array()
        .expect("get_one_json should carry a geofences array")
        .iter()
        .filter_map(|v| v.as_u64())
        .collect();
    assert_eq!(
        ids,
        std::collections::HashSet::from([fence1_id as u64, fence2_id as u64]),
        "get_one_json geofences should be exactly the 2 linked ids, got {:?}",
        got["geofences"]
    );

    let upserted_again = upserted_again.expect("upsert_json_return should succeed");
    let ids2: std::collections::HashSet<u64> = upserted_again["geofences"]
        .as_array()
        .expect("upsert_json_return should carry a geofences array")
        .iter()
        .filter_map(|v| v.as_u64())
        .collect();
    assert_eq!(
        ids2, ids,
        "upsert_json_return geofences should match get_one_json's, got {:?}",
        upserted_again["geofences"]
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
    let id = created["id"]
        .as_u64()
        .expect("created tile_server has an id") as u32;

    // Gather, then delete, then assert (panic-safe cleanup — see project test).
    let got = tile_server::Query::get_one_json(&db, id.to_string()).await;
    let listed = tile_server::Query::search(&db, name.clone()).await;

    tile_server::Query::delete(&db, id).await.expect("delete");
    let after = tile_server::Query::get_one(&db, id.to_string()).await;

    let got = got.expect("get_one_json");
    assert_eq!(got["name"], json!(name));
    assert_eq!(
        got["url"],
        json!("https://tiles.example.com/{z}/{x}/{y}.png")
    );

    let listed = listed.expect("search");
    assert!(
        listed.iter().any(|t| t["name"] == json!(name)),
        "search should find the new tile_server"
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
    let listed = property::Query::search(&db, name.clone()).await;

    property::Query::delete(&db, id).await.expect("delete");
    let after = property::Query::get_one(&db, id.to_string()).await;

    let got = got.expect("get_one_json");
    assert_eq!(got["name"], json!(name));
    assert_eq!(got["category"], json!("string"), "category round-trips");

    let listed = listed.expect("search");
    assert!(
        listed.iter().any(|p| p["name"] == json!(name)),
        "search should find the new property"
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

/// `get_koji_by_bbox`'s `mode`/`id_scope` filters (Task 7): two fences with
/// distinct modes, both inside one query bbox, one of them linked to a
/// project. Covers the four call shapes the `/v2/geofences?bbox&mode&
/// projects=` handler composes: no filters, `mode` alone, a populated
/// `id_scope`, and — the easy-to-get-wrong case — an explicitly EMPTY
/// `id_scope` (`Some(vec![])`), which must return zero rows (a project with
/// no linked fences is a real filter, not "no filter").
#[tokio::test]
async fn geofence_bbox_mode_and_id_scope_filters() {
    let Some(db) = test_db().await else { return };
    let pokemon_name = unique_name("bbox-pokemon");
    let fort_name = unique_name("bbox-fort");
    let project_name = unique_name("bbox-project");

    // Both geometries sit inside the query bbox used below. Deliberately far
    // from the origin (unlike this file's other fixtures, which cluster
    // around `[0,0]-[1,1]`) — this crate's tests share one live DB with no
    // per-run isolation, so a query bbox anywhere near the origin risks
    // picking up leaked rows from other test runs and making the exact-count
    // assertions below flaky.
    let pokemon_geometry = json!({
        "type": "Polygon",
        "coordinates": [[
            [172.0, 82.0], [172.0, 83.0], [173.0, 83.0], [173.0, 82.0], [172.0, 82.0]
        ]]
    });
    let fort_geometry = json!({
        "type": "Polygon",
        "coordinates": [[
            [175.0, 82.0], [175.0, 83.0], [176.0, 83.0], [176.0, 82.0], [175.0, 82.0]
        ]]
    });
    let bbox: [f64; 4] = [170.0, 80.0, 179.0, 85.0];

    let created_project =
        project::Query::upsert_json_return(&db, 0, json!({ "name": project_name }))
            .await
            .expect("project upsert should insert");
    let project_id = created_project["id"]
        .as_u64()
        .expect("created project has an id") as u32;

    let created_pokemon = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": pokemon_name, "mode": "pokemon", "geometry": pokemon_geometry }),
    )
    .await
    .expect("pokemon geofence upsert should insert");
    let pokemon_id = created_pokemon["id"]
        .as_u64()
        .expect("created pokemon geofence has an id") as u32;

    let created_fort = geofence::Query::upsert_json_return(
        &db,
        0,
        json!({ "name": fort_name, "mode": "fort", "geometry": fort_geometry }),
    )
    .await
    .expect("fort geofence upsert should insert");
    let fort_id = created_fort["id"]
        .as_u64()
        .expect("created fort geofence has an id") as u32;

    // Link only the pokemon fence to the project.
    geofence_project::Query::upsert_related_by_project_id(&db, &[json!(pokemon_id)], project_id)
        .await
        .expect("link pokemon fence to project");

    // Gather every observation FIRST, then delete, then assert (panic-safe
    // cleanup — see geofence_crud_round_trip above).
    let no_filter = geofence::Query::get_koji_by_bbox(&db, bbox, None, None).await;
    let mode_filtered = geofence::Query::get_koji_by_bbox(
        &db,
        bbox,
        Some(koji_db::db::sea_orm_active_enums::Mode::Pokemon),
        None,
    )
    .await;
    let id_scope_filtered =
        geofence::Query::get_koji_by_bbox(&db, bbox, None, Some(vec![pokemon_id])).await;
    let empty_id_scope_filtered =
        geofence::Query::get_koji_by_bbox(&db, bbox, None, Some(vec![])).await;

    geofence::Query::delete(&db, pokemon_id)
        .await
        .expect("delete pokemon fence");
    geofence::Query::delete(&db, fort_id)
        .await
        .expect("delete fort fence");
    project::Query::delete(&db, project_id)
        .await
        .expect("delete project");

    // (a) no filters — both fences.
    let no_filter = no_filter.expect("get_koji_by_bbox(None, None) should succeed");
    assert_eq!(
        no_filter.items.len(),
        2,
        "no mode/id_scope filter should return both fences"
    );

    // (b) mode=Pokemon — exactly the pokemon fence.
    let mode_filtered = mode_filtered.expect("get_koji_by_bbox(mode=Pokemon) should succeed");
    assert_eq!(
        mode_filtered.items.len(),
        1,
        "mode=Pokemon should return exactly 1 fence"
    );
    assert_eq!(mode_filtered.items[0].meta.id, Some(pokemon_id));

    // (c) id_scope=Some([pokemon_id]) — exactly the pokemon fence.
    let id_scope_filtered =
        id_scope_filtered.expect("get_koji_by_bbox(id_scope=Some([pokemon_id])) should succeed");
    assert_eq!(
        id_scope_filtered.items.len(),
        1,
        "id_scope=Some([pokemon_id]) should return exactly 1 fence"
    );
    assert_eq!(id_scope_filtered.items[0].meta.id, Some(pokemon_id));

    // (d) id_scope=Some([]) — a real, empty filter, distinct from None.
    let empty_id_scope_filtered =
        empty_id_scope_filtered.expect("get_koji_by_bbox(id_scope=Some(vec![])) should succeed");
    assert_eq!(
        empty_id_scope_filtered.items.len(),
        0,
        "id_scope=Some(vec![]) must return zero rows, not fall back to unfiltered"
    );
}
