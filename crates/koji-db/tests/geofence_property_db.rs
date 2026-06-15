//! DB-integration tests for `geofence_property::Query` — full suite:
//! upsert, update_properties_by_geofence, add_db_property,
//! update_values_for_property.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test geofence_property_db -- --nocapture`

use koji_db::db::{geofence, geofence_property, property};
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
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

async fn make_property(db: &DatabaseConnection, name: &str) -> u32 {
    let created = property::Query::upsert_json_return(
        db,
        0,
        json!({ "name": name, "category": "string", "default_value": "default" }),
    )
    .await
    .expect("make_property");
    created["id"].as_u64().expect("has id") as u32
}

// ── geofence_property::upsert ─────────────────────────────────────────────────

#[tokio::test]
async fn geofence_property_upsert_creates_and_updates_link() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-gp-ups");
    let prop_name = unique_name("prop-gp-ups");
    let fence_id = make_geofence(&db, &fence_name).await;
    let prop_id = make_property(&db, &prop_name).await;

    // Insert the geofence_property link with a custom value
    let link = geofence_property::Query::upsert(
        &db,
        &json!({ "property_id": prop_id, "geofence_id": fence_id, "value": "hello" }),
        Some(fence_id),
    )
    .await
    .expect("upsert insert");

    let link_id = link.id;
    assert_eq!(link.geofence_id, fence_id);
    assert_eq!(link.property_id, prop_id);
    assert_eq!(link.value.as_deref(), Some("hello"), "value is set");

    // Upsert again (same fence+property) with a new value → update
    let updated = geofence_property::Query::upsert(
        &db,
        &json!({ "property_id": prop_id, "geofence_id": fence_id, "value": "world" }),
        Some(fence_id),
    )
    .await
    .expect("upsert update");

    // cleanup — geofence cascade deletes geofence_property rows
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");
    property::Query::delete(&db, prop_id)
        .await
        .expect("del prop");

    assert_eq!(updated.id, link_id, "same row id after update");
    assert_eq!(updated.value.as_deref(), Some("world"), "value updated");
}

#[tokio::test]
async fn geofence_property_upsert_invalid_property_is_err() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-gp-inv");
    let fence_id = make_geofence(&db, &fence_name).await;

    // property_id 999999999 should not exist
    let result = geofence_property::Query::upsert(
        &db,
        &json!({ "property_id": 999999999_u32, "geofence_id": fence_id }),
        Some(fence_id),
    )
    .await;

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    assert!(
        result.is_err(),
        "upsert with invalid property_id should error"
    );
}

// ── update_properties_by_geofence ─────────────────────────────────────────────

#[tokio::test]
async fn geofence_property_update_properties_by_geofence_replaces_links() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-gp-upbg");
    let prop1_name = unique_name("prop-gp-upbg1");
    let prop2_name = unique_name("prop-gp-upbg2");
    let fence_id = make_geofence(&db, &fence_name).await;
    let prop1_id = make_property(&db, &prop1_name).await;
    let prop2_id = make_property(&db, &prop2_name).await;

    // Set initial: link to prop1 with value "v1"
    geofence_property::Query::update_properties_by_geofence(
        &db,
        &[json!({ "property_id": prop1_id, "geofence_id": fence_id, "value": "v1" })],
        Some(fence_id),
    )
    .await
    .expect("initial set");

    // Replace: now link to prop2 instead (omitting prop1 → it gets deleted)
    let updated = geofence_property::Query::update_properties_by_geofence(
        &db,
        &[json!({ "property_id": prop2_id, "geofence_id": fence_id, "value": "v2" })],
        Some(fence_id),
    )
    .await
    .expect("replacement set");

    // Check the geofence's related_properties via get_one_json_with_related
    let with_related = geofence::Query::get_one_json_with_related(&db, fence_id.to_string())
        .await
        .expect("get with related");

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");
    property::Query::delete(&db, prop1_id)
        .await
        .expect("del prop1");
    property::Query::delete(&db, prop2_id)
        .await
        .expect("del prop2");

    assert_eq!(updated.len(), 1, "one link after replacement");

    let props = with_related["properties"].as_array().unwrap();
    // prop2 should be there; prop1 should NOT (it was removed by update_properties_by_geofence)
    // Note: upsert always adds a built-in "name" db-property so filter to custom props only
    let custom_prop_ids: Vec<u64> = props
        .iter()
        .filter_map(|p| p["property_id"].as_u64())
        .collect();
    assert!(
        custom_prop_ids.contains(&(prop2_id as u64)),
        "prop2 is linked"
    );
    assert!(
        !custom_prop_ids.contains(&(prop1_id as u64)),
        "prop1 was removed by update_properties_by_geofence"
    );
}

// ── add_db_property ───────────────────────────────────────────────────────────

#[tokio::test]
async fn geofence_property_add_db_property_idempotent() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence_name = unique_name("gf-addbdp");
    let fence_id = make_geofence(&db, &fence_name).await;

    // add_db_property creates or reuses the "parent" property row + link
    let first = geofence_property::Query::add_db_property(&db, fence_id, "parent")
        .await
        .expect("add_db_property first call");

    // Calling again must not error (idempotent)
    let second = geofence_property::Query::add_db_property(&db, fence_id, "parent")
        .await
        .expect("add_db_property second call");

    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    assert_eq!(
        first.property_id, second.property_id,
        "same property_id (idempotent)"
    );
    assert_eq!(first.geofence_id, fence_id, "geofence_id is correct");
    // "parent" is a database-category property → value must be NULL
    assert!(
        first.value.is_none(),
        "db-category property value is always NULL"
    );
}

// ── update_values_for_property ────────────────────────────────────────────────

#[tokio::test]
async fn geofence_property_update_values_for_property_changes_all_links() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    let fence1_name = unique_name("gf-uvfp1");
    let fence2_name = unique_name("gf-uvfp2");
    let prop_name = unique_name("prop-uvfp");

    let fence1_id = make_geofence(&db, &fence1_name).await;
    let fence2_id = make_geofence(&db, &fence2_name).await;
    let prop_id = make_property(&db, &prop_name).await;

    // Link both fences to the same property with value "old"
    geofence_property::Query::upsert(
        &db,
        &json!({ "property_id": prop_id, "geofence_id": fence1_id, "value": "old" }),
        Some(fence1_id),
    )
    .await
    .expect("link fence1");
    geofence_property::Query::upsert(
        &db,
        &json!({ "property_id": prop_id, "geofence_id": fence2_id, "value": "old" }),
        Some(fence2_id),
    )
    .await
    .expect("link fence2");

    // Update all links for this property to "new"
    let result = geofence_property::Query::update_values_for_property(
        &db,
        prop_id,
        &Some("new".to_string()),
    )
    .await
    .expect("update_values_for_property");

    // Read back both links
    let link1 = geofence_property::Entity::find()
        .filter(geofence_property::Column::GeofenceId.eq(fence1_id))
        .filter(geofence_property::Column::PropertyId.eq(prop_id))
        .one(&db)
        .await
        .expect("read link1");

    let link2 = geofence_property::Entity::find()
        .filter(geofence_property::Column::GeofenceId.eq(fence2_id))
        .filter(geofence_property::Column::PropertyId.eq(prop_id))
        .one(&db)
        .await
        .expect("read link2");

    geofence::Query::delete(&db, fence1_id)
        .await
        .expect("del fence1");
    geofence::Query::delete(&db, fence2_id)
        .await
        .expect("del fence2");
    property::Query::delete(&db, prop_id)
        .await
        .expect("del prop");

    assert!(result.rows_affected >= 2, "at least 2 rows updated");
    assert_eq!(
        link1.unwrap().value.as_deref(),
        Some("new"),
        "fence1 link value updated"
    );
    assert_eq!(
        link2.unwrap().value.as_deref(),
        Some("new"),
        "fence2 link value updated"
    );
}
