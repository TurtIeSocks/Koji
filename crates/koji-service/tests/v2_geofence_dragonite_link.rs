// The `serial_guard()` pattern intentionally holds a `MutexGuard` across `.await`
// points to serialise concurrent test execution — safe in test context only.
#![allow(clippy::await_holding_lock)]
//! DB-backed tests for Dragonite targets: a Dragonite is a `webhook_subscription`
//! row in `dragonite` mode, a geofence's Area is stored per row, and publish,
//! `link-dragonite` and `/test` work against local Dragonite stand-ins.
//!
//! Gated on `KOJI_DB_URL` like `tests/v2_db.rs` (skips when unset). The database
//! must hold no other active Dragonite row: a global one would reach the fences
//! these tests create.
//! ```sh
//! set -a; source ./.env.test; set +a
//! cargo test -p koji-service --test v2_geofence_dragonite_link -- --nocapture
//! ```

use actix_web::test;
use koji_db::KojiDb;
use koji_events::EventDispatcher;
use koji_jobs::{JobId, JobQueue};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use serde_json::{Value as Json, json};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
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

/// A Dragonite stand-in: `GET /v2/areas/` serves `pages` (one JSON array per
/// page, with `X-Page`/`X-Total-Pages`, counting pages from 0) and every
/// `PATCH` is recorded by its request line and answered with a bare area. A
/// request without `Bearer {bearer}` gets Dragonite's `401`.
struct Stub {
    base: String,
    patches: Arc<Mutex<Vec<String>>>,
}

impl Stub {
    async fn start(bearer: &str, pages: Vec<Json>) -> Stub {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let patches = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&patches);
        let want = format!("Bearer {bearer}").to_ascii_lowercase();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let (recorded, want, pages) = (Arc::clone(&recorded), want.clone(), pages.clone());
                tokio::spawn(async move {
                    let mut req = Vec::new();
                    let mut buf = [0u8; 4096];
                    // Read to the end of the head, then the declared body.
                    loop {
                        let n = sock.read(&mut buf).await.unwrap_or(0);
                        req.extend_from_slice(&buf[..n]);
                        let text = String::from_utf8_lossy(&req).to_ascii_lowercase();
                        if let Some(head_end) = text.find("\r\n\r\n") {
                            let declared = text[..head_end]
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            if req.len() >= head_end + 4 + declared {
                                break;
                            }
                        }
                        if n == 0 {
                            break;
                        }
                    }
                    let head = String::from_utf8_lossy(&req).to_string();
                    let line = head.lines().next().unwrap_or_default().to_string();
                    let authorized = head
                        .to_ascii_lowercase()
                        .lines()
                        .any(|l| l.strip_prefix("authorization:").map(str::trim) == Some(&want));
                    let (status, extra, body) = if !authorized {
                        (
                            "401 Unauthorized",
                            String::new(),
                            json!({ "detail": "bad bearer" }).to_string(),
                        )
                    } else if line.starts_with("PATCH") {
                        recorded.lock().unwrap().push(line.clone());
                        ("200 OK", String::new(), json!({ "id": 1 }).to_string())
                    } else {
                        let page = line
                            .split(['?', '&', ' '])
                            .find_map(|kv| kv.strip_prefix("page="))
                            .and_then(|v| v.parse::<usize>().ok())
                            .unwrap_or(0);
                        (
                            "200 OK",
                            format!("x-page: {page}\r\nx-total-pages: {}\r\n", pages.len()),
                            pages.get(page).cloned().unwrap_or(json!([])).to_string(),
                        )
                    };
                    let reply = format!(
                        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{extra}content-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    let _ = sock.write_all(reply.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        Stub { base, patches }
    }

    fn patches(&self) -> Vec<String> {
        self.patches.lock().unwrap().clone()
    }
}

macro_rules! app {
    ($db:expr) => {{
        let koji = koji_db($db.clone()).await;
        let jobs = Arc::new(JobQueue::new($db.clone(), "test-worker"));
        test::init_service(koji_service::test_db_app(koji, jobs)).await
    }};
}

/// Create a project through `$app`, returning its id.
macro_rules! create_project {
    ($app:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v2/projects")
            .set_json(json!({ "name": unique_name("proj") }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 201);
        body_json(resp).await["data"]["id"].as_u64().unwrap()
    }};
}

/// Create a webhook of `$mode` through `$app`, returning its id.
macro_rules! create_hook {
    ($app:expr, $mode:expr, $url:expr, $project:expr, $active:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v2/webhooks")
            .set_json(json!({
                "name": unique_name("hook"), "url": $url, "secret": "tok",
                "mode": $mode, "method": "POST", "project_id": $project, "active": $active,
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 201);
        body_json(resp).await["data"]["id"].as_u64().unwrap()
    }};
}

/// An active Dragonite row for `$url` in `$project` (`None` = every geofence).
macro_rules! create_dragonite {
    ($app:expr, $url:expr, $project:expr) => {
        create_hook!($app, "dragonite", $url, $project, true)
    };
}

/// Create a geofence in `$projects` through `$app`, returning its id.
macro_rules! create_fence {
    ($app:expr, $name:expr, $mode:expr, $projects:expr) => {{
        let req = test::TestRequest::post()
            .uri("/api/v2/geofences")
            .set_json(json!({
                "name": $name, "mode": $mode, "geometry": triangle(), "projects": $projects,
            }))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        assert_eq!(resp.status(), 201);
        body_json(resp).await["data"]["id"].as_u64().unwrap()
    }};
}

/// PATCH `$prefix/{id}` with `$body`; the status and the body.
macro_rules! patch {
    ($app:expr, $prefix:expr, $id:expr, $body:expr) => {{
        let req = test::TestRequest::patch()
            .uri(&format!("{}/{}", $prefix, $id))
            .set_json($body)
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let status = resp.status();
        (status.as_u16(), body_json(resp).await)
    }};
}

/// A geofence's `dragonite_links` as `(webhook, area)` pairs, from a record.
fn links(record: &Json) -> Vec<(u64, u64)> {
    let mut links: Vec<(u64, u64)> = record["data"]["dragonite_links"]
        .as_array()
        .unwrap_or_else(|| panic!("no dragonite_links: {record}"))
        .iter()
        .map(|l| {
            (
                l["webhook_id"].as_u64().unwrap(),
                l["dragonite_area_id"].as_u64().unwrap(),
            )
        })
        .collect();
    links.sort();
    links
}

/// `POST /geofences/{id}/publish` as `(status, body)`.
macro_rules! publish {
    ($app:expr, $id:expr) => {{
        let req = test::TestRequest::post()
            .uri(&format!("/api/v2/geofences/{}/publish", $id))
            .to_request();
        let resp = test::call_service(&$app, req).await;
        let status = resp.status();
        (status.as_u16(), body_json(resp).await)
    }};
}

/// Drop the outbox rows a publish answer names.
async fn forget_events(db: &DatabaseConnection, published: &Json) {
    for event in published["data"]["events"].as_array().into_iter().flatten() {
        if let Some(id) = event["event_id"].as_str() {
            cleanup(db, "event_outbox", "public_id", Value::from(id)).await;
        }
    }
}

#[actix_web::test]
async fn patch_sets_changes_and_clears_links_one_per_target() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let east = create_dragonite!(app, "http://127.0.0.1:1", Json::Null);
    let west = create_dragonite!(app, "http://127.0.0.1:1", Json::Null);
    let id = create_fence!(app, &unique_name("link-patch"), "pokemon", json!([]));
    let uri = "/api/v2/geofences";

    let (_, set) = patch!(
        app,
        uri,
        id,
        json!({ "dragonite_links": [
            { "webhook_id": east, "dragonite_area_id": 12 },
            { "webhook_id": west, "dragonite_area_id": 12 },
        ] })
    );
    // A PATCH that leaves the links out reads back what is stored.
    let (_, kept) = patch!(app, uri, id, json!({ "mode": "pokemon" }));
    // Naming one target leaves the other as it was.
    let (_, changed) = patch!(
        app,
        uri,
        id,
        json!({ "dragonite_links": [{ "webhook_id": east, "dragonite_area_id": 34 }] })
    );
    let (_, cleared) = patch!(
        app,
        uri,
        id,
        json!({ "dragonite_links": [{ "webhook_id": west, "dragonite_area_id": null }] })
    );

    for hook in [east, west] {
        cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    }
    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(links(&set), [(east, 12), (west, 12)]);
    assert_eq!(links(&kept), [(east, 12), (west, 12)]);
    assert_eq!(links(&changed), [(east, 34), (west, 12)]);
    assert_eq!(links(&cleared), [(east, 34)]);
}

#[actix_web::test]
async fn patch_refuses_a_missing_or_non_dragonite_target_and_a_taken_area() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let dragonite = create_dragonite!(app, "http://127.0.0.1:1", Json::Null);
    let ping = create_hook!(app, "ping", "http://127.0.0.1:1", Json::Null, true);
    let name = unique_name("link-refuse");
    let first = create_fence!(app, &name, "pokemon", json!([]));
    let same_mode = create_fence!(app, &unique_name("link-refuse-2"), "pokemon", json!([]));
    let other_mode = create_fence!(app, &unique_name("link-refuse-3"), "quest", json!([]));
    let uri = "/api/v2/geofences";
    let link = |hook: u64, area: u32| json!({ "dragonite_links": [{ "webhook_id": hook, "dragonite_area_id": area }] });

    let (ok, _) = patch!(app, uri, first, link(dragonite, 7));
    let (taken, taken_body) = patch!(app, uri, same_mode, link(dragonite, 7));
    let (quest_ok, _) = patch!(app, uri, other_mode, link(dragonite, 7));
    // A mode change that would make two fences share the Area is refused too.
    let (mode_clash, _) = patch!(app, uri, other_mode, json!({ "mode": "pokemon" }));
    let (not_dragonite, _) = patch!(app, uri, first, link(ping, 1));
    let (missing, _) = patch!(app, uri, first, link(u64::MAX / 2, 1));

    for hook in [dragonite, ping] {
        cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    }
    for fence in [first, same_mode, other_mode] {
        cleanup(&db, "geofence", "id", Value::from(fence)).await;
    }
    assert_eq!(ok, 200);
    assert_eq!(taken, 409, "{taken_body}");
    assert_eq!(quest_ok, 200, "the Area is free in another mode");
    assert_eq!(mode_clash, 409);
    assert_eq!(not_dragonite, 422);
    assert_eq!(missing, 422);
}

#[actix_web::test]
async fn internal_read_and_patch_carry_the_links() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let jobs = Arc::new(JobQueue::new(db.clone(), "test-worker"));
    let app = test::init_service(koji_service::test_db_app_with_internal(
        koji_db(db.clone()).await,
        jobs,
    ))
    .await;
    let hook = create_dragonite!(app, "http://127.0.0.1:1", Json::Null);
    let id = create_fence!(app, unique_name("link-internal"), "fort", json!([]));

    let (status, _) = patch!(
        app,
        "/internal/geofences",
        id,
        json!({ "dragonite_links": [{ "webhook_id": hook, "dragonite_area_id": 8 }] })
    );
    let read = test::TestRequest::get()
        .uri(&format!("/internal/geofences/{id}"))
        .to_request();
    let read = body_json(test::call_service(&app, read).await).await;

    cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(status, 200);
    assert_eq!(
        read["data"]["properties"]["dragonite_links"],
        json!([{ "webhook_id": hook, "dragonite_area_id": 8 }])
    );
}

#[actix_web::test]
async fn publish_sends_one_patch_to_each_linked_dragonite_at_its_own_area() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let east = Stub::start("tok", vec![]).await;
    let west = Stub::start("tok", vec![]).await;
    let app = app!(db);
    let east_hook = create_dragonite!(app, &east.base, Json::Null);
    let west_hook = create_dragonite!(app, &west.base, Json::Null);
    let id = create_fence!(app, &unique_name("publish-two"), "quest", json!([]));
    // Both Dragonites number their Areas from 1, so the ids overlap.
    let (linked, _) = patch!(
        app,
        "/api/v2/geofences",
        id,
        json!({ "dragonite_links": [
            { "webhook_id": east_hook, "dragonite_area_id": 12 },
            { "webhook_id": west_hook, "dragonite_area_id": 34 },
        ] })
    );

    let (status, published) = publish!(app, id);
    let dispatcher = Arc::new(EventDispatcher::new(
        db.clone(),
        vec![koji_service::dragonite_subscriber(db.clone())],
        "publish-two-test",
    ));
    let handle = dispatcher.spawn();
    for _ in 0..100 {
        if !east.patches().is_empty() && !west.patches().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    // Let a duplicate delivery show up before counting.
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.shutdown().await;

    forget_events(&db, &published).await;
    for hook in [east_hook, west_hook] {
        cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    }
    cleanup(&db, "geofence", "id", Value::from(id)).await;
    assert_eq!(linked, 200);
    assert_eq!(status, 202, "{published}");
    assert_eq!(published["data"]["events"].as_array().unwrap().len(), 2);
    assert_eq!(published["data"]["mode"], "quest");
    assert_eq!(east.patches(), ["PATCH /v2/areas/12 HTTP/1.1"]);
    assert_eq!(west.patches(), ["PATCH /v2/areas/34 HTTP/1.1"]);
}

#[actix_web::test]
async fn a_row_scoped_to_one_project_does_not_reach_a_geofence_outside_it() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let project_a = create_project!(app);
    let project_b = create_project!(app);
    let hook = create_dragonite!(app, "http://127.0.0.1:1", project_a);
    let inside = create_fence!(app, &unique_name("scope-in"), "pokemon", json!([project_a]));
    let outside = create_fence!(
        app,
        &unique_name("scope-out"),
        "pokemon",
        json!([project_b])
    );
    let uri = "/api/v2/geofences";
    for fence in [inside, outside] {
        // The link is stored on both; only reach differs.
        let (status, _) = patch!(
            app,
            uri,
            fence,
            json!({ "dragonite_links": [{ "webhook_id": hook, "dragonite_area_id": 5 }] })
        );
        assert_eq!(status, 200);
    }

    let (inside_status, inside_body) = publish!(app, inside);
    let (outside_status, outside_body) = publish!(app, outside);

    forget_events(&db, &inside_body).await;
    cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    for fence in [inside, outside] {
        cleanup(&db, "geofence", "id", Value::from(fence)).await;
    }
    for project in [project_a, project_b] {
        cleanup(&db, "project", "id", Value::from(project)).await;
    }
    assert_eq!(inside_status, 202, "{inside_body}");
    assert_eq!(outside_status, 422, "{outside_body}");
    assert!(
        outside_body["error"]["message"]
            .as_str()
            .unwrap()
            .contains("no active Dragonite webhook reaches"),
        "{outside_body}"
    );
}

#[actix_web::test]
async fn publish_names_which_of_the_row_or_the_link_is_missing() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let project = create_project!(app);
    let fence = create_fence!(app, &unique_name("missing"), "fort", json!([]));
    let message = |body: &Json| body["error"]["message"].as_str().unwrap().to_string();

    // No Dragonite row at all.
    let (no_row, no_row_body) = publish!(app, fence);
    // A row that reaches it (global) but holds no link for it.
    let global = create_dragonite!(app, "http://127.0.0.1:1", Json::Null);
    let (no_link, no_link_body) = publish!(app, fence);
    // A linked row that is inactive reaches nothing.
    let (linked, _) = patch!(
        app,
        "/api/v2/geofences",
        fence,
        json!({ "dragonite_links": [{ "webhook_id": global, "dragonite_area_id": 3 }] })
    );
    let (deactivated, _) = patch!(app, "/api/v2/webhooks", global, json!({ "active": false }));
    let (inactive, inactive_body) = publish!(app, fence);

    cleanup(&db, "webhook_subscription", "id", Value::from(global)).await;
    cleanup(&db, "geofence", "id", Value::from(fence)).await;
    cleanup(&db, "project", "id", Value::from(project)).await;
    assert_eq!(no_row, 422);
    assert!(message(&no_row_body).contains("no active Dragonite webhook reaches"));
    assert_eq!(no_link, 422);
    assert!(
        message(&no_link_body).contains("no Dragonite area link"),
        "{no_link_body}"
    );
    assert_eq!((linked, deactivated), (200, 200));
    assert_eq!(inactive, 422);
    assert!(message(&inactive_body).contains("no active Dragonite webhook reaches"));
}

#[actix_web::test]
async fn link_dragonite_links_by_name_only_inside_the_targets_reach() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let inside_name = unique_name("link-inside");
    let outside_name = unique_name("link-outside");
    let held_name = unique_name("link-held");
    let no_area = unique_name("link-no-area");
    let no_fence = unique_name("link-no-fence");
    let stub = Stub::start(
        "tok",
        vec![
            json!([{ "id": 101, "name": inside_name }, { "id": 102, "name": outside_name }]),
            json!([{ "id": 103, "name": no_fence }, { "id": 104, "name": held_name }]),
        ],
    )
    .await;
    let app = app!(db);
    let project_a = create_project!(app);
    let project_b = create_project!(app);
    let hook = create_dragonite!(app, &stub.base, project_a);
    let inside = create_fence!(app, &inside_name, "pokemon", json!([project_a]));
    let inside_quest = create_fence!(app, &inside_name, "quest", json!([project_a]));
    let outside = create_fence!(app, &outside_name, "pokemon", json!([project_b]));
    let orphan = create_fence!(app, &no_area, "pokemon", json!([project_a]));
    // Area 104 is already held in pokemon mode by a fence out of reach.
    let holder = create_fence!(
        app,
        &unique_name("link-holder"),
        "pokemon",
        json!([project_b])
    );
    let held = create_fence!(app, &held_name, "pokemon", json!([project_a]));
    let uri = "/api/v2/geofences";
    let (held_by, _) = patch!(
        app,
        uri,
        holder,
        json!({ "dragonite_links": [{ "webhook_id": hook, "dragonite_area_id": 104 }] })
    );
    assert_eq!(held_by, 200);

    let post = |dry_run: bool| {
        test::TestRequest::post()
            .uri("/api/v2/geofences/link-dragonite")
            .set_json(json!({ "webhook_id": hook, "dry_run": dry_run }))
            .to_request()
    };
    let dry = body_json(test::call_service(&app, post(true)).await).await;
    let after_dry = stored_links(&db, inside).await;
    let real = body_json(test::call_service(&app, post(false)).await).await;
    let after_real = (
        stored_links(&db, inside).await,
        stored_links(&db, inside_quest).await,
        stored_links(&db, outside).await,
        stored_links(&db, held).await,
    );
    let again = body_json(test::call_service(&app, post(false)).await).await;

    cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    for fence in [inside, inside_quest, outside, orphan, holder, held] {
        cleanup(&db, "geofence", "id", Value::from(fence)).await;
    }
    for project in [project_a, project_b] {
        cleanup(&db, "project", "id", Value::from(project)).await;
    }

    // Ids the answer lists under `key`, for our own geofences only.
    let ours = [inside, inside_quest, outside, orphan, holder, held];
    let ids = |answer: &Json, key: &str| -> Vec<u64> {
        let mut ids: Vec<u64> = answer["data"][key]
            .as_array()
            .unwrap_or_else(|| panic!("{key} missing: {answer}"))
            .iter()
            .filter_map(|e| e["geofence"].as_u64())
            .filter(|id| ours.contains(id))
            .collect();
        ids.sort();
        ids
    };
    for answer in [&dry, &real] {
        // Both modes of the name inside the project link; the outside fence is
        // never considered, so it is in no list.
        assert_eq!(ids(answer, "linked"), [inside, inside_quest]);
        assert_eq!(ids(answer, "area_taken"), [held]);
        assert_eq!(ids(answer, "geofences_without_area"), [orphan]);
        assert!(ids(answer, "conflicts").is_empty());
        assert!(!ids(answer, "geofences_without_area").contains(&outside));
        let areas: Vec<&str> = answer["data"]["areas_without_geofence"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|a| a["name"].as_str())
            .collect();
        // The Area named after the outside fence has no fence in reach.
        assert!(areas.contains(&outside_name.as_str()), "{areas:?}");
        assert!(areas.contains(&no_fence.as_str()), "{areas:?}");
    }
    assert_eq!(dry["data"]["dry_run"], true);
    assert_eq!(real["data"]["dry_run"], false);
    assert!(after_dry.is_empty(), "dry_run wrote a link: {after_dry:?}");
    assert_eq!(after_real.0, [(hook, 101)]);
    assert_eq!(after_real.1, [(hook, 101)]);
    assert!(after_real.2.is_empty(), "linked outside the reach");
    assert!(after_real.3.is_empty(), "linked a taken Area");
    assert_eq!(ids(&again, "already_linked"), [inside, inside_quest]);
    assert!(ids(&again, "linked").is_empty());
}

/// A geofence's stored links as `(webhook, area)` pairs.
async fn stored_links(db: &DatabaseConnection, geofence: u64) -> Vec<(u64, u64)> {
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT `webhook_id`, `dragonite_area_id` FROM `dragonite_link` \
             WHERE `geofence_id` = ? ORDER BY `webhook_id`",
            [Value::from(geofence)],
        ))
        .await
        .unwrap();
    rows.iter()
        .map(|r| {
            (
                r.try_get::<u64>("", "webhook_id").unwrap(),
                r.try_get::<u32>("", "dragonite_area_id").unwrap() as u64,
            )
        })
        .collect()
}

#[actix_web::test]
async fn link_dragonite_refuses_a_missing_or_non_dragonite_row() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let ping = create_hook!(app, "ping", "http://127.0.0.1:1", Json::Null, true);
    let post = |hook: u64| {
        test::TestRequest::post()
            .uri("/api/v2/geofences/link-dragonite")
            .set_json(json!({ "webhook_id": hook }))
            .to_request()
    };

    let not_dragonite = test::call_service(&app, post(ping)).await.status();
    let missing = test::call_service(&app, post(u64::MAX / 2)).await.status();

    cleanup(&db, "webhook_subscription", "id", Value::from(ping)).await;
    assert_eq!(not_dragonite, 422);
    assert_eq!(missing, 404);
}

#[actix_web::test]
async fn test_on_a_dragonite_row_answers_the_area_count_or_the_upstream_error() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let stub = Stub::start(
        "tok",
        vec![json!([{ "id": 1, "name": "a" }, { "id": 2, "name": "b" }])],
    )
    .await;
    let app = app!(db);
    let good = create_dragonite!(app, &stub.base, Json::Null);
    // Same Dragonite, wrong bearer: the stand-in answers its `401`.
    let wrong = test::TestRequest::post()
        .uri("/api/v2/webhooks")
        .set_json(json!({
            "name": unique_name("hook"), "url": stub.base, "secret": "nope",
            "mode": "dragonite", "method": "POST",
        }))
        .to_request();
    let wrong = body_json(test::call_service(&app, wrong).await).await["data"]["id"]
        .as_u64()
        .unwrap();
    let fire = |hook: u64| {
        test::TestRequest::post()
            .uri(&format!("/api/v2/webhooks/{hook}/test"))
            .to_request()
    };

    let counted = body_json(test::call_service(&app, fire(good)).await).await;
    let refused = body_json(test::call_service(&app, fire(wrong)).await).await;

    for hook in [good, wrong] {
        cleanup(&db, "webhook_subscription", "id", Value::from(hook)).await;
    }
    assert_eq!(counted["data"]["delivered"], true, "{counted}");
    assert_eq!(counted["data"]["area_count"], 2);
    assert_eq!(refused["data"]["delivered"], false, "{refused}");
    assert_eq!(refused["data"]["upstream_status"], 401);
    assert!(
        refused["data"]["error"]
            .as_str()
            .unwrap()
            .contains("bad bearer"),
        "{refused}"
    );
}

#[actix_web::test]
async fn a_geofence_named_link_dragonite_is_still_fetched_by_name() {
    let Some(db) = test_db().await else { return };
    let _serial = serial_guard();
    let app = app!(db);
    let id = create_fence!(app, "link-dragonite", "fort", json!([]));

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
