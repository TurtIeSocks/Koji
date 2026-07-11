//! DB-integration tests for deep route query paths: upsert_from_geometry.
//! Gates on `KOJI_DB_URL`; skips cleanly with no env.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p koji-db --test route_deep_db -- --nocapture`

use koji_core::{KojiGeometry, KojiGeometryCollection, KojiMeta, Mode};
use koji_db::db::{geofence, route};
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

async fn make_route(db: &DatabaseConnection, name: &str, fence_id: u32) -> u32 {
    let created = route::Query::upsert_json_return(
        db,
        0,
        json!({
            "name": name,
            "geofence_id": fence_id,
            "mode": "pokemon",
            "geometry": { "type": "MultiPoint", "coordinates": [[1.0,2.0],[3.0,4.0]] }
        }),
    )
    .await
    .expect("make_route");
    created["id"].as_u64().expect("has id") as u32
}

// ── upsert_from_geometry (route) ──────────────────────────────────────────────

#[tokio::test]
async fn route_upsert_from_geometry_inserts_and_returns_counts() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    use geo::{Geometry, MultiPoint, Point};

    let fence_name = unique_name("fence-rufg");
    let route_name = unique_name("route-rufg");
    let fence_id = make_geofence(&db, &fence_name).await;

    // Build a KojiGeometry for the route — geofence_id in extra
    let mut extra = serde_json::Map::new();
    extra.insert("geofence_id".to_string(), json!(fence_id));

    let route_item = KojiGeometry {
        geometry: Geometry::MultiPoint(MultiPoint::from(vec![
            Point::new(1.0, 2.0),
            Point::new(3.0, 4.0),
        ])),
        meta: KojiMeta {
            name: Some(route_name.clone()),
            mode: Mode::Pokemon,
            extra,
            ..Default::default()
        },
    };

    let coll = KojiGeometryCollection::new(vec![route_item]);
    let (inserts, updates) = route::Query::upsert_from_geometry(&db, &coll)
        .await
        .expect("upsert_from_geometry");

    // Verify the route exists
    let got = route::Query::get_one(&db, route_name.clone()).await;

    // cleanup
    if let Ok(ref model) = got {
        route::Query::delete(&db, model.id)
            .await
            .expect("del route");
    }
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    assert_eq!(inserts, 1, "one insert");
    assert_eq!(updates, 0, "no updates on first run");

    let model = got.expect("route exists after upsert_from_geometry");
    assert_eq!(model.name, route_name, "route name persisted");
    assert_eq!(model.geofence_id, fence_id, "geofence_id linked");
}

#[tokio::test]
async fn route_upsert_from_geometry_update_existing_row() {
    let Some(db) = test_db().await else { return };
    let _g = serial_guard().await;

    use geo::{Geometry, MultiPoint, Point};

    let fence_name = unique_name("fence-rufg2");
    let route_name = unique_name("route-rufg2");
    let fence_id = make_geofence(&db, &fence_name).await;

    // First insert via upsert_json_return
    let created = route::Query::upsert_json_return(
        &db,
        0,
        json!({
            "name": route_name,
            "geofence_id": fence_id,
            "mode": "pokemon",
            "geometry": { "type": "MultiPoint", "coordinates": [[0.0,0.0]] }
        }),
    )
    .await
    .expect("first insert");
    let route_id = created["id"].as_u64().unwrap() as u32;

    // Second call via upsert_from_geometry — same name+mode → update
    let mut extra = serde_json::Map::new();
    extra.insert("geofence_id".to_string(), json!(fence_id));
    let route_item = KojiGeometry {
        geometry: Geometry::MultiPoint(MultiPoint::from(vec![Point::new(5.0, 6.0)])),
        meta: KojiMeta {
            name: Some(route_name.clone()),
            mode: Mode::Pokemon,
            extra,
            ..Default::default()
        },
    };
    let coll = KojiGeometryCollection::new(vec![route_item]);
    let (inserts, updates) = route::Query::upsert_from_geometry(&db, &coll)
        .await
        .expect("second upsert");

    route::Query::delete(&db, route_id)
        .await
        .expect("del route");
    geofence::Query::delete(&db, fence_id)
        .await
        .expect("del fence");

    assert_eq!(inserts, 0, "no inserts on re-run of existing name+mode");
    assert_eq!(updates, 1, "one update for existing row");
}
