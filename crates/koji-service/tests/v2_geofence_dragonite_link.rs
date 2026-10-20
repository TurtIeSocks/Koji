// The `serial_guard()` pattern intentionally holds a `MutexGuard` across `.await`
// points to serialise concurrent test execution — safe in test context only.
#![allow(clippy::await_holding_lock)]
//! DB-backed tests for linking Geofences to Dragonite Areas: `PATCH
//! dragonite_area_id`, the bulk `POST /api/v2/geofences/link-dragonite` against
//! a local Dragonite stand-in, and publish on a Geofence linked that way.
//!
//! Gated on `KOJI_DB_URL` like `tests/v2_db.rs` (skips when unset):
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-service --test v2_geofence_dragonite_link -- --nocapture
//! ```

use actix_web::test;
use koji_db::KojiDb;
use koji_dragonite::DragoniteClient;
use koji_jobs::{JobId, JobQueue};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use serde_json::{Value as Json, json};
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
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

async fn koji_db(db: DatabaseConnection) -> KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    KojiDb { koji: db, golbat }
}

fn unique_name(tag: &str) -> String {
    format!("test-{tag}-{}", JobId::new().as_string())
}

async fn body_json(resp: actix_web::dev::ServiceResponse) -> Json {
    let bytes = test::read_body(resp).await;
    serde_json::from_slice(&bytes).expect("response body must be valid JSON")
}

async fn cleanup(db: &DatabaseConnection, table: &str, column: &str, value: Value) {
    let _ = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("DELETE FROM `{table}` WHERE `{column}` = ?"),
            [value],
        ))
        .await;
}

fn triangle() -> Json {
    json!({ "type": "Polygon", "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]] })
}

/// Serve `GET /v2/areas/` from `pages`, one JSON array per page, answering
/// `X-Page` and `X-Total-Pages` as Dragonite does, counting pages from 0.
/// Returns the base URL.
async fn dragonite_stand_in(pages: Vec<Json>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        loop {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = sock.read(&mut buf).await.unwrap_or(0);
            let head = String::from_utf8_lossy(&buf[..n]).to_string();
            let line = head.lines().next().unwrap_or_default();
            let page = line
                .split(['?', '&', ' '])
                .find_map(|kv| kv.strip_prefix("page="))
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(0);
            let body = pages.get(page).cloned().unwrap_or(json!([])).to_string();
            let reply = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\nx-page: {page}\r\nx-total-pages: {}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                pages.len(),
                body.len()
            );
            let _ = sock.write_all(reply.as_bytes()).await;
            let _ = sock.shutdown().await;
        }
    });
    base
}

macro_rules! app {
    ($db:expr, $dragonite:expr) => {{
        let koji = koji_db($db.clone()).await;
        let jobs = Arc::new(JobQueue::new($db.clone(), "test-worker"));
        test::init_service(koji_service::test_db_app_with_dragonite(
            koji,
            jobs,
            DragoniteClient::new($dragonite, "tok"),
        ))
        .await
    }};
}

/// Create a geofence through `$app`, returning its id.
macro_rules! create_fence {
    ($app:expr, $name:expr, $mode:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v2/geofences")
            .set_json(json!({ "name": $name, "mode": $mode, "geometry": triangle() }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 201);
        body_json(resp).await["data"]["id"].as_u64().unwrap()
    }};
}

/// PATCH `$prefix/{id}` with `$body`, returning the stored `dragonite_area_id`.
macro_rules! patch_link {
    ($app:expr, $prefix:expr, $id:expr, $body:expr) => {{
        let req = test::TestRequest::patch()
            .uri(&format!("{}/{}", $prefix, $id))
            .set_json($body)
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 200);
        body_json(resp).await["data"]["dragonite_area_id"].clone()
    }};
}

#[actix_web::test]
async fn patch_sets_changes_and_clears_dragonite_area_id() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db, "http://127.0.0.1:1");
    let id = create_fence!(app, &unique_name("link-patch"), "pokemon");
    let uri = "/api/v2/geofences";

    let set = patch_link!(app, uri, id, json!({ "dragonite_area_id": 12 }));
    // A PATCH that leaves the link out reads back what is stored.
    let kept = patch_link!(app, uri, id, json!({ "mode": "pokemon" }));
    let changed = patch_link!(app, uri, id, json!({ "dragonite_area_id": 34 }));
    let cleared = patch_link!(app, uri, id, json!({ "dragonite_area_id": null }));
    let after_clear = patch_link!(app, uri, id, json!({ "mode": "pokemon" }));

    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(set, 12);
    assert_eq!(kept, 12);
    assert_eq!(changed, 34);
    assert!(cleared.is_null(), "{cleared}");
    assert!(after_clear.is_null(), "{after_clear}");
}

#[actix_web::test]
async fn internal_patch_sets_dragonite_area_id() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app_with_internal(
        koji_db(db.clone()).await,
        jobs,
    ))
    .await;
    let id = create_fence!(app, unique_name("link-internal"), "fort");

    let set = patch_link!(
        app,
        "/internal/geofences",
        id,
        json!({ "dragonite_area_id": 8 })
    );

    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(set, 8);
}

#[actix_web::test]
async fn link_dragonite_links_by_name_across_two_pages_and_dry_run_changes_nothing() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let both = unique_name("link-both");
    let held = unique_name("link-held");
    let no_area = unique_name("link-no-area");
    let no_fence = unique_name("link-no-fence");
    let dragonite = dragonite_stand_in(vec![
        json!([{ "id": 101, "name": both }, { "id": 102, "name": held }]),
        json!([{ "id": 103, "name": no_fence }]),
    ])
    .await;
    let app = app!(db, dragonite);
    let pokemon = create_fence!(app, &both, "pokemon");
    let quest = create_fence!(app, &both, "quest");
    let conflicted = create_fence!(app, &held, "pokemon");
    let orphan = create_fence!(app, &no_area, "pokemon");
    let uri = "/api/v2/geofences";
    let held_link = patch_link!(app, uri, conflicted, json!({ "dragonite_area_id": 999 }));
    assert_eq!(held_link, 999);

    let post = |dry_run: bool| {
        test::TestRequest::post()
            .uri("/api/v2/geofences/link-dragonite")
            .set_json(json!({ "dry_run": dry_run }))
            .to_request()
    };
    let dry = body_json(test::call_service(&app, post(true)).await).await;
    let after_dry = patch_link!(app, uri, pokemon, json!({ "mode": "pokemon" }));
    let real = body_json(test::call_service(&app, post(false)).await).await;
    let linked_pokemon = patch_link!(app, uri, pokemon, json!({ "mode": "pokemon" }));
    let linked_quest = patch_link!(app, uri, quest, json!({ "mode": "quest" }));
    let still_held = patch_link!(app, uri, conflicted, json!({ "mode": "pokemon" }));
    let again = body_json(test::call_service(&app, post(false)).await).await;

    for id in [pokemon, quest, conflicted, orphan] {
        cleanup(&db, "geofence", "id", Value::from(id)).await;
    }

    // Ids the answer lists under `key`, for our own geofences only.
    let ids = |answer: &Json, key: &str| -> Vec<u64> {
        let mut ids: Vec<u64> = answer["data"][key]
            .as_array()
            .unwrap_or_else(|| panic!("{key} missing: {answer}"))
            .iter()
            .filter_map(|e| e["geofence"].as_u64())
            .filter(|id| [pokemon, quest, conflicted, orphan].contains(id))
            .collect();
        ids.sort();
        ids
    };
    for answer in [&dry, &real] {
        assert_eq!(ids(answer, "linked"), [pokemon, quest]);
        assert_eq!(ids(answer, "conflicts"), [conflicted]);
        assert_eq!(ids(answer, "geofences_without_area"), [orphan]);
        assert!(ids(answer, "already_linked").is_empty());
        let areas: Vec<&str> = answer["data"]["areas_without_geofence"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        assert!(areas.contains(&no_fence.as_str()), "{areas:?}");
        let conflict = answer["data"]["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["geofence"] == conflicted)
            .unwrap();
        assert_eq!(conflict["dragonite_area_id"], 999);
        assert_eq!(conflict["wanted_dragonite_area_id"], 102);
    }
    assert_eq!(dry["data"]["dry_run"], true);
    assert_eq!(real["data"]["dry_run"], false);
    assert!(after_dry.is_null(), "dry_run wrote a link: {after_dry}");
    assert_eq!(linked_pokemon, 101);
    assert_eq!(linked_quest, 101);
    assert_eq!(still_held, 999);
    assert_eq!(ids(&again, "already_linked"), [pokemon, quest]);
    assert!(ids(&again, "linked").is_empty());
}

#[actix_web::test]
async fn publish_works_on_a_geofence_linked_by_name() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let name = unique_name("link-publish");
    let dragonite = dragonite_stand_in(vec![json!([{ "id": 104, "name": name }])]).await;
    let app = app!(db, dragonite);
    let id = create_fence!(app, &name, "quest");

    let link = test::TestRequest::post()
        .uri("/api/v2/geofences/link-dragonite")
        .set_json(json!({}))
        .to_request();
    let linked = test::call_service(&app, link).await.status();
    let publish = test::TestRequest::post()
        .uri(&format!("/api/v2/geofences/{id}/publish"))
        .to_request();
    let resp = test::call_service(&app, publish).await;
    let status = resp.status();
    let published = body_json(resp).await;

    cleanup(&db, "geofence", "id", Value::from(id)).await;
    if let Some(event_id) = published["data"]["event_id"].as_str() {
        cleanup(&db, "event_outbox", "public_id", Value::from(event_id)).await;
    }
    assert_eq!(linked, 200);
    assert_eq!(status, 202, "{published}");
    assert_eq!(published["data"]["dragonite_area_id"], 104);
    assert_eq!(published["data"]["mode"], "quest");
}

#[actix_web::test]
async fn a_geofence_named_link_dragonite_is_still_fetched_by_name() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db, "http://127.0.0.1:1");
    let id = create_fence!(app, "link-dragonite", "fort");

    let get = test::TestRequest::get()
        .uri("/api/v2/geofences/link-dragonite?mode=fort")
        .to_request();
    let resp = test::call_service(&app, get).await;
    let status = resp.status();
    let fetched = body_json(resp).await;

    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(status, 200, "{fetched}");
    assert_eq!(fetched["data"]["properties"]["name"], "link-dragonite");
}
