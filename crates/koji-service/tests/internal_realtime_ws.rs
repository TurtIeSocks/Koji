//! WS hub integration: subscribe→delivery, ping→pong, mutation→frame.
//! No DB required for ping/pong + manual publish (uses the hub directly via a
//! test-only publish route). The mutation-publish path is covered in
//! `internal_geofences_rows.rs` (DB-gated). Auth: `KOJI_SECRET` unset = open.
//!
//! Adaptation note: `actix-test::start` (the `actix-test` crate) provides a
//! live TCP test server; `awc` is the WS client. The brief's
//! `actix_web::test::start` is a re-export of the same function — both resolve
//! to `actix_test::start`.
//!
//! Env-var isolation: `KOJI_SECRET` is process-global. All tests that mutate it
//! hold `ENV_LOCK` for their duration so concurrent test threads don't race.
use actix_web::{web, App};
use futures_util::{SinkExt, StreamExt};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection};
use std::sync::Mutex;

/// Serializes all env-var mutations across the test suite to prevent races.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Macro to build the standard test App — avoids repeating the 12-line builder.
macro_rules! realtime_test_app {
    ($hub_data:expr) => {{
        let hd = $hub_data;
        App::new()
            .app_data(hd)
            .wrap(
                actix_session::SessionMiddleware::builder(
                    actix_session::storage::CookieSessionStore::default(),
                    actix_web::cookie::Key::from(&[0u8; 64]),
                )
                .cookie_secure(false)
                .build(),
            )
            .route(
                "/internal/realtime",
                web::get().to(koji_service::realtime_ws),
            )
    }};
}

#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn ping_gets_pong_and_subscribed_event_is_delivered() {
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test-only env mutation; serialized by ENV_LOCK.
    unsafe { std::env::set_var("KOJI_SECRET", ""); }
    let hub = koji_service::test_realtime_hub();
    let hub_data = web::Data::new(hub.clone());
    let srv = actix_test::start(move || realtime_test_app!(hub_data.clone()));
    let url = srv.url("/internal/realtime").replace("http://", "ws://");
    let (_resp, mut conn) = awc::Client::new().ws(url).connect().await.unwrap();

    // ping → pong
    conn.send(awc::ws::Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
    let msg = conn.next().await.unwrap().unwrap();
    assert!(matches!(msg, awc::ws::Frame::Text(ref b) if b.starts_with(b"{\"op\":\"pong\"")));

    // subscribe then publish on the hub → event frame delivered
    conn.send(awc::ws::Message::Text(r#"{"op":"subscribe","topic":"resource/geofence"}"#.into())).await.unwrap();
    // give the reader task a tick to register the subscription
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    hub.publish("resource/geofence", koji_service::test_server_event("created", serde_json::json!({"ids":[1]})));
    let msg = conn.next().await.unwrap().unwrap();
    let awc::ws::Frame::Text(bytes) = msg else { panic!("expected text frame") };
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["topic"], "resource/geofence");
    assert_eq!(v["type"], "created");
    assert_eq!(v["payload"]["ids"], serde_json::json!([1]));
}

#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn unsubscribed_topic_is_not_delivered() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("KOJI_SECRET", ""); }
    let hub = koji_service::test_realtime_hub();
    let hub_data = web::Data::new(hub.clone());
    let srv = actix_test::start(move || realtime_test_app!(hub_data.clone()));
    let url = srv.url("/internal/realtime").replace("http://", "ws://");
    let (_r, mut conn) = awc::Client::new().ws(url).connect().await.unwrap();
    conn.send(awc::ws::Message::Text(r#"{"op":"subscribe","topic":"jobs"}"#.into())).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    hub.publish("resource/geofence", koji_service::test_server_event("created", serde_json::json!({"ids":[1]})));
    // round-trip a ping; the only frame we should see back is the pong, not the event.
    conn.send(awc::ws::Message::Text(r#"{"op":"ping"}"#.into())).await.unwrap();
    let msg = conn.next().await.unwrap().unwrap();
    let awc::ws::Frame::Text(bytes) = msg else { panic!("text") };
    assert!(bytes.starts_with(b"{\"op\":\"pong\""), "got {:?}", bytes);
}

#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn no_token_is_rejected_when_secret_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("KOJI_SECRET", "topsecret"); }
    let hub_data = web::Data::new(koji_service::test_realtime_hub());
    let srv = actix_test::start(move || realtime_test_app!(hub_data.clone()));

    // No token → server returns 401 before WS upgrade; awc returns Err.
    let url = srv.url("/internal/realtime").replace("http://", "ws://");
    let result = awc::Client::new().ws(url).connect().await;
    assert!(
        result.is_err(),
        "expected WS connect to fail without token, got: {:?}",
        result.ok().map(|(r, _)| r.status())
    );

    unsafe { std::env::remove_var("KOJI_SECRET"); }
}

#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn wrong_token_is_rejected_when_secret_set() {
    let _guard = ENV_LOCK.lock().unwrap();
    unsafe { std::env::set_var("KOJI_SECRET", "topsecret"); }
    let hub_data = web::Data::new(koji_service::test_realtime_hub());
    let srv = actix_test::start(move || realtime_test_app!(hub_data.clone()));

    let url = srv
        .url("/internal/realtime?token=wrongvalue")
        .replace("http://", "ws://");
    let result = awc::Client::new().ws(url).connect().await;
    assert!(
        result.is_err(),
        "expected WS connect to fail with wrong token, got ok"
    );

    unsafe { std::env::remove_var("KOJI_SECRET"); }
}

#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn url_encoded_token_is_accepted() {
    let _guard = ENV_LOCK.lock().unwrap();
    // Secret contains chars that need percent-encoding when placed in a query string:
    // `+` (encodes to `%2B`), space (encodes to `+`), `=` (encodes to `%3D`).
    let secret = "top+sec ret=value";
    unsafe { std::env::set_var("KOJI_SECRET", secret); }
    let hub_data = web::Data::new(koji_service::test_realtime_hub());
    let srv = actix_test::start(move || realtime_test_app!(hub_data.clone()));

    // Percent-encode the secret correctly before embedding in the URL.
    let encoded: String = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("token", secret)
        .finish();
    let url = srv
        .url(&format!("/internal/realtime?{encoded}"))
        .replace("http://", "ws://");
    let result = awc::Client::new().ws(url).connect().await;
    assert!(
        result.is_ok(),
        "expected WS connect to succeed with URL-encoded token, got: {:?}",
        result.err()
    );

    unsafe { std::env::remove_var("KOJI_SECRET"); }
}

// ---------------------------------------------------------------------------
// DB-gated: mutation → WS event
// ---------------------------------------------------------------------------

/// Returns `Some(conn)` when `KOJI_DB_URL` is set, otherwise `None`
/// (so DB-gated tests skip cleanly in CI without a database).
async fn db_or_skip() -> Option<DatabaseConnection> {
    let url = std::env::var("KOJI_DB_URL").ok()?;
    Database::connect(&url).await.ok()
}

/// Build a `KojiDb` pointing both `koji` and `golbat` at the same `KOJI_DB_URL`
/// (golbat is used for reads only; the test DB is a fine dummy).
async fn build_test_koji_db(koji_db: DatabaseConnection) -> koji_db::KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    koji_db::KojiDb {
        koji: koji_db,
        golbat,
    }
}

/// Returns a short random slug for unique test record names.
fn uuid_slug() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos()
        .to_string()
}

/// DB-gated: a POST /internal/geofences that succeeds must publish a
/// `resource/geofence` `created` event to subscribers.
///
/// Boots a live actix server with the full /internal scope + the shared hub.
/// Connects a WS subscriber on `resource/geofence`, fires the POST, and
/// asserts the created-event frame arrives with the new id.
#[actix_web::test]
#[allow(clippy::await_holding_lock)]
async fn creating_a_geofence_publishes_resource_event() {
    let Some(conn) = db_or_skip().await else { return; };
    let _guard = ENV_LOCK.lock().unwrap();
    // SAFETY: test-only env mutation — serialized by ENV_LOCK so no KOJI_SECRET
    // set by a concurrent auth test bleeds into the WS upgrade.
    unsafe { std::env::set_var("KOJI_SECRET", ""); }
    let db = build_test_koji_db(conn.clone()).await;
    let hub = koji_service::test_realtime_hub();
    let hub2 = hub.clone();
    let dbc = db.clone();
    let srv = actix_test::start(move || koji_service::test_internal_live_app(dbc.clone(), hub2.clone()));
    let ws_url = srv.url("/internal/realtime").replace("http://", "ws://");
    let (_r, mut ws) = awc::Client::new().ws(ws_url).connect().await.unwrap();
    ws.send(awc::ws::Message::Text(
        r#"{"op":"subscribe","topic":"resource/geofence"}"#.into(),
    ))
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // POST a minimal geofence via /internal/geofences — proving the route
    // now correctly handles POST (not 405) and publishes the WS event.
    let name = format!("ws-rt-test-{}", uuid_slug());
    let body = serde_json::json!({
        "name": name,
        "geometry": {
            "type": "Polygon",
            "coordinates": [[[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,0.0]]]
        }
    });
    let resp = awc::Client::new()
        .post(srv.url("/internal/geofences"))
        .send_json(&body)
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 201, "POST /internal/geofences must return 201, not 405");

    // Expect the WS frame
    let frame = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        ws.next(),
    )
    .await
    .expect("timed out waiting for WS frame")
    .unwrap()
    .unwrap();
    let awc::ws::Frame::Text(bytes) = frame else {
        panic!("expected text frame, got {:?}", frame);
    };
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["topic"], "resource/geofence");
    assert_eq!(v["type"], "created");
    let ids = v["payload"]["ids"].as_array().expect("payload.ids must be array");
    assert_eq!(ids.len(), 1, "exactly one id in created event");
    let id = ids[0].as_i64().unwrap();

    // Cleanup
    let _ = conn.execute(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::MySql,
        "DELETE FROM geofence WHERE id = ?",
        [sea_orm::Value::from(id)],
    )).await;
}
