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
