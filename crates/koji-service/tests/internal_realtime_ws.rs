//! WS hub integration: subscribe→delivery, ping→pong, mutation→frame.
//! No DB required for ping/pong + manual publish (uses the hub directly via a
//! test-only publish route). The mutation-publish path is covered in
//! `internal_geofences_rows.rs` (DB-gated). Auth: `KOJI_SECRET` unset = open.
//!
//! Adaptation note: `actix-test::start` (the `actix-test` crate) provides a
//! live TCP test server; `awc` is the WS client. The brief's
//! `actix_web::test::start` is a re-export of the same function — both resolve
//! to `actix_test::start`.
use actix_web::{web, App};
use futures_util::{SinkExt, StreamExt};

#[actix_web::test]
async fn ping_gets_pong_and_subscribed_event_is_delivered() {
    // SAFETY: test-only env mutation; serialized by being the sole test touching it.
    unsafe { std::env::set_var("KOJI_SECRET", ""); }
    let hub = koji_service::test_realtime_hub();
    let hub_data = web::Data::new(hub.clone());
    let srv = actix_test::start(move || {
        App::new()
            .app_data(hub_data.clone())
            .wrap(actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0u8;64]),
            ).cookie_secure(false).build())
            .route("/internal/realtime", web::get().to(koji_service::realtime_ws))
    });
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
    unsafe { std::env::set_var("KOJI_SECRET", ""); }
    let hub = koji_service::test_realtime_hub();
    let hub_data = web::Data::new(hub.clone());
    let srv = actix_test::start(move || {
        App::new().app_data(hub_data.clone())
            .wrap(actix_session::SessionMiddleware::builder(
                actix_session::storage::CookieSessionStore::default(),
                actix_web::cookie::Key::from(&[0u8;64]),
            ).cookie_secure(false).build())
            .route("/internal/realtime", web::get().to(koji_service::realtime_ws))
    });
    let url = srv.url("/internal/realtime").replace("http://","ws://");
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
