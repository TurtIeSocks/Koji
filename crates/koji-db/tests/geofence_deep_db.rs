//! DB-integration tests for deep geofence query paths: project_as_feature,
//! project_as_koji, descendants, by_project, by_parent_koji, unique_parents,
//! upsert_from_geometry. All require multi-row fixtures (parent + children).
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test geofence_deep_db -- --nocapture`

use koji_core::{KojiGeometry, KojiGeometryCollection, KojiMeta, Mode, UnknownId};
use koji_core::Precision;
use koji_db::db::geofence::{Anchor, HierarchySpec};
use koji_db::db::{geofence, project};
use koji_db::query_args::ApiQueryArgs;
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

/// A small polygon slightly offset by the given amount so fixtures don't
/// overlap each other (no actual spatial checks depend on the exact values).
fn polygon_at(offset: Precision) -> serde_json::Value {
    json!({
        "type": "Polygon",
        "coordinates": [[[offset,offset],[offset+1.0,offset],[offset+1.0,offset+1.0],[offset,offset+1.0],[offset,offset]]]
    })
}

async fn make_geofence(db: &DatabaseConnection, name: &str, offset: Precision) -> u32 {
    let created = geofence::Query::upsert_json_return(
        db,
        0,
        json!({ "name": name, "mode": "unset", "geometry": polygon_at(offset) }),
    )
    .await
    .expect("make_geofence");
    created["id"].as_u64().expect("has id") as u32
}

async fn make_project(db: &DatabaseConnection, name: &str) -> u32 {
    let created =
        project::Query::upsert_json_return(db, 0, json!({ "name": name }))
            .await
            .expect("make_project");
    created["id"].as_u64().expect("has id") as u32
}

// ── by_project ────────────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_by_project_finds_linked_geofences() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-byproj");
    let proj_name = unique_name("proj-byproj");
    let fence_id = make_geofence(&db, &fence_name, 0.0).await;
    let proj_id = make_project(&db, &proj_name).await;

    // Link geofence → project
    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [proj_id] }), fence_id)
        .await
        .expect("link");

    // Lookup by project name
    let by_name = geofence::Query::by_project(&db, proj_name.clone())
        .await
        .expect("by_project (name)");

    // Lookup by project id
    let by_id = geofence::Query::by_project(&db, proj_id.to_string())
        .await
        .expect("by_project (id)");

    // cleanup: unlink then delete
    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [] }), fence_id)
        .await
        .expect("unlink");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("del proj");

    assert!(
        by_name.iter().any(|r| r["name"] == json!(fence_name)),
        "by_project(name) returns the linked geofence"
    );
    assert!(
        by_id.iter().any(|r| r["name"] == json!(fence_name)),
        "by_project(id) returns the linked geofence"
    );
}

// ── project_as_feature ────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_project_as_feature_returns_geojson_features() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-feat");
    let proj_name = unique_name("proj-feat");
    let fence_id = make_geofence(&db, &fence_name, 2.0).await;
    let proj_id = make_project(&db, &proj_name).await;

    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [proj_id] }), fence_id)
        .await
        .expect("link");

    let args = ApiQueryArgs {
        name: Some(true),
        id: Some(true),
        ..Default::default()
    };
    let features = geofence::Query::project_as_feature(&db, proj_name.clone(), &args)
        .await
        .expect("project_as_feature");

    // cleanup
    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [] }), fence_id)
        .await
        .expect("unlink");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("del proj");

    assert!(!features.is_empty(), "project_as_feature returns features");
    let found = features
        .iter()
        .any(|f| f.property("name").and_then(|v| v.as_str()) == Some(&fence_name));
    assert!(found, "feature with fence name is in result");
    // geometry is a Polygon
    assert!(
        features.iter().all(|f| f.geometry.is_some()),
        "all features have geometry"
    );
}

// ── project_as_koji ───────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_project_as_koji_returns_geometry_collection() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-koji");
    let proj_name = unique_name("proj-koji");
    let fence_id = make_geofence(&db, &fence_name, 4.0).await;
    let proj_id = make_project(&db, &proj_name).await;

    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [proj_id] }), fence_id)
        .await
        .expect("link");

    let args = ApiQueryArgs {
        name: Some(true),
        id: Some(true),
        ..Default::default()
    };
    let coll = geofence::Query::project_as_koji(&db, proj_name.clone(), &args)
        .await
        .expect("project_as_koji");

    // cleanup
    geofence::Query::upsert_related_projects(&db, &json!({ "projects": [] }), fence_id)
        .await
        .expect("unlink");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");
    project::Query::delete(&db, proj_id)
        .await
        .expect("del proj");

    assert!(!coll.items.is_empty(), "collection is non-empty");
    let found = coll
        .items
        .iter()
        .any(|kg| kg.meta.name.as_deref() == Some(&fence_name));
    assert!(found, "KojiGeometry with fence name is in collection");
    // geometry is a Polygon variant
    let all_polygon = coll
        .items
        .iter()
        .all(|kg| matches!(kg.geometry, geo::Geometry::Polygon(_)));
    assert!(all_polygon, "all geometries are Polygon");
}

// ── descendants ───────────────────────────────────────────────────────────────
//
// Insert parent + two children; verify descendants(Anchor::Id, Depth(1)) returns
// all three rows. Then verify ancestors_from_ancestry is reflected in meta.ancestors.

#[tokio::test]
async fn geofence_descendants_depth_returns_full_subtree() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let parent_name = unique_name("gf-parent");
    let child1_name = unique_name("gf-child1");
    let child2_name = unique_name("gf-child2");

    let parent_id = make_geofence(&db, &parent_name, 10.0).await;
    let child1_id = make_geofence(&db, &child1_name, 12.0).await;
    let child2_id = make_geofence(&db, &child2_name, 14.0).await;

    // Associate children to parent using assign on Column::Parent
    geofence::Query::assign(&db, child1_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign child1 parent");
    geofence::Query::assign(&db, child2_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign child2 parent");

    let coll = geofence::Query::descendants(&db, Anchor::Id(parent_id), HierarchySpec::Depth(1))
        .await
        .expect("descendants");

    // cleanup children first (parent last, no cascade on this soft link)
    geofence::Query::delete(&db, child1_id)
        .await
        .expect("del child1");
    geofence::Query::delete(&db, child2_id)
        .await
        .expect("del child2");
    geofence::Query::delete(&db, parent_id)
        .await
        .expect("del parent");

    let names: Vec<&str> = coll
        .items
        .iter()
        .filter_map(|kg| kg.meta.name.as_deref())
        .collect();
    assert!(
        names.contains(&parent_name.as_str()),
        "parent in descendants (depth 0)"
    );
    assert!(
        names.contains(&child1_name.as_str()),
        "child1 in descendants"
    );
    assert!(
        names.contains(&child2_name.as_str()),
        "child2 in descendants"
    );
    assert_eq!(coll.items.len(), 3, "parent + 2 children = 3 rows");

    // children have ancestors = [parent_name]
    let children: Vec<&KojiGeometry> = coll
        .items
        .iter()
        .filter(|kg| kg.meta.name.as_deref() != Some(&parent_name))
        .collect();
    assert!(
        children
            .iter()
            .all(|kg| kg.meta.ancestors == vec![parent_name.clone()]),
        "children's ancestors path contains parent name"
    );
}

#[tokio::test]
async fn geofence_descendants_level1_returns_only_children() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let parent_name = unique_name("gf-lv-par");
    let child1_name = unique_name("gf-lv-ch1");
    let child2_name = unique_name("gf-lv-ch2");

    let parent_id = make_geofence(&db, &parent_name, 20.0).await;
    let child1_id = make_geofence(&db, &child1_name, 22.0).await;
    let child2_id = make_geofence(&db, &child2_name, 24.0).await;

    geofence::Query::assign(&db, child1_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign child1");
    geofence::Query::assign(&db, child2_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign child2");

    let coll = geofence::Query::descendants(&db, Anchor::Id(parent_id), HierarchySpec::Level(1))
        .await
        .expect("descendants level 1");

    geofence::Query::delete(&db, child1_id)
        .await
        .expect("del child1");
    geofence::Query::delete(&db, child2_id)
        .await
        .expect("del child2");
    geofence::Query::delete(&db, parent_id)
        .await
        .expect("del parent");

    let names: Vec<&str> = coll
        .items
        .iter()
        .filter_map(|kg| kg.meta.name.as_deref())
        .collect();
    assert_eq!(coll.items.len(), 2, "Level(1) returns only the 2 children");
    assert!(names.contains(&child1_name.as_str()), "child1 present");
    assert!(names.contains(&child2_name.as_str()), "child2 present");
    assert!(
        !names.contains(&parent_name.as_str()),
        "parent excluded from Level(1)"
    );
}

// ── by_parent_koji ────────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_by_parent_koji_returns_direct_children() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let parent_name = unique_name("gf-bpk-par");
    let child_name = unique_name("gf-bpk-ch");

    let parent_id = make_geofence(&db, &parent_name, 30.0).await;
    let child_id = make_geofence(&db, &child_name, 32.0).await;

    geofence::Query::assign(&db, child_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign parent");

    // Lookup by numeric id
    let by_id = geofence::Query::by_parent_koji(&db, &UnknownId::Number(parent_id))
        .await
        .expect("by_parent_koji (id)");

    // Lookup by name string
    let by_name = geofence::Query::by_parent_koji(&db, &UnknownId::String(parent_name.clone()))
        .await
        .expect("by_parent_koji (name)");

    geofence::Query::delete(&db, child_id)
        .await
        .expect("del child");
    geofence::Query::delete(&db, parent_id)
        .await
        .expect("del parent");

    assert_eq!(by_id.items.len(), 1, "one direct child by id");
    assert_eq!(
        by_id.items[0].meta.name.as_deref(),
        Some(child_name.as_str())
    );

    assert_eq!(by_name.items.len(), 1, "one direct child by name");
    assert_eq!(
        by_name.items[0].meta.name.as_deref(),
        Some(child_name.as_str())
    );
}

#[tokio::test]
async fn geofence_by_parent_koji_missing_parent_name_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let result = geofence::Query::by_parent_koji(
        &db,
        &UnknownId::String("zzz-no-such-parent-999".to_string()),
    )
    .await;
    assert!(result.is_err(), "missing parent name → ModelError");
}

// ── unique_parents ────────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_unique_parents_includes_used_parent() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let parent_name = unique_name("gf-up-par");
    let child_name = unique_name("gf-up-ch");

    let parent_id = make_geofence(&db, &parent_name, 40.0).await;
    let child_id = make_geofence(&db, &child_name, 42.0).await;

    geofence::Query::assign(&db, child_id, "parent".to_string(), json!(parent_id))
        .await
        .expect("assign parent");

    let parents = geofence::Query::unique_parents(&db)
        .await
        .expect("unique_parents");

    geofence::Query::delete(&db, child_id)
        .await
        .expect("del child");
    geofence::Query::delete(&db, parent_id)
        .await
        .expect("del parent");

    assert!(
        parents.iter().any(|p| p["id"] == json!(parent_id)),
        "parent_id appears in unique_parents result"
    );
    assert!(
        parents.iter().any(|p| p["name"] == json!(parent_name)),
        "parent_name appears in unique_parents result"
    );
}

// ── upsert_from_geometry ──────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_upsert_from_geometry_inserts_and_associates_parent() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    use geo::{Geometry, LineString, Polygon, coord};

    let parent_name = unique_name("gf-ufg-par");
    let child_name = unique_name("gf-ufg-ch");

    let make_polygon = || {
        Geometry::Polygon(Polygon::new(
            LineString::new(vec![
                coord! { x: 0.0, y: 0.0 },
                coord! { x: 1.0, y: 0.0 },
                coord! { x: 1.0, y: 1.0 },
                coord! { x: 0.0, y: 1.0 },
                coord! { x: 0.0, y: 0.0 },
            ]),
            vec![],
        ))
    };

    // Parent item: name via typed KojiMeta
    let parent_item = KojiGeometry {
        geometry: make_polygon(),
        meta: KojiMeta {
            name: Some(parent_name.clone()),
            mode: Mode::Unset,
            ..Default::default()
        },
    };

    // Child item: parent reference in extra
    let mut extra = serde_json::Map::new();
    extra.insert("parent".to_string(), json!(parent_name.clone()));
    let child_item = KojiGeometry {
        geometry: make_polygon(),
        meta: KojiMeta {
            name: Some(child_name.clone()),
            mode: Mode::Pokemon,
            extra,
            ..Default::default()
        },
    };

    let coll = KojiGeometryCollection::new(vec![parent_item, child_item]);

    geofence::Query::upsert_from_geometry(&db, &coll)
        .await
        .expect("upsert_from_geometry");

    // Verify both rows exist
    let parent_model = geofence::Query::get_one(&db, parent_name.clone())
        .await
        .expect("parent exists");
    let child_model = geofence::Query::get_one(&db, child_name.clone())
        .await
        .expect("child exists");

    // cleanup
    geofence::Query::delete(&db, child_model.id)
        .await
        .expect("del child");
    geofence::Query::delete(&db, parent_model.id)
        .await
        .expect("del parent");

    assert_eq!(parent_model.name, parent_name);
    assert_eq!(child_model.name, child_name);
    // child should be linked to parent
    assert_eq!(
        child_model.parent,
        Some(parent_model.id),
        "child's parent column points to parent row"
    );
}
