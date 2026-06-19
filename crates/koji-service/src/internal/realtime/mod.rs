//! In-process realtime pub/sub hub + the `GET /internal/realtime` WS handler.
//! ponytail: ONE `tokio::broadcast` of `(topic, ServerEvent)` + a per-connection
//! subscribed-topic filter. Shard to a `DashMap<topic, …>` only if connection
//! count ever justifies it.
pub mod topics;
pub use topics::ServerEvent;

use serde::Deserialize;
use tokio::sync::broadcast;

const CHANNEL_CAP: usize = 1024;

#[derive(Clone)]
pub struct RealtimeHub {
    tx: broadcast::Sender<(String, ServerEvent)>,
}
impl RealtimeHub {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(CHANNEL_CAP);
        RealtimeHub { tx }
    }
    /// Broadcast an event on a topic. Lossy iff a slow subscriber lags > CHANNEL_CAP
    /// (broadcast drops oldest for that receiver — acceptable: client refetches on
    /// reconnect). A send with zero receivers is a no-op (returns Err, ignored).
    pub fn publish(&self, topic: &str, event: ServerEvent) {
        let _ = self.tx.send((topic.to_string(), event));
    }
    pub fn subscribe(&self) -> broadcast::Receiver<(String, ServerEvent)> {
        self.tx.subscribe()
    }
}
impl Default for RealtimeHub { fn default() -> Self { Self::new() } }

impl koji_jobs::JobEventSink for RealtimeHub {
    /// Publish a job status change to `jobs/{id}` (type `"status"`) AND to
    /// `jobs` (type `"updated"`). Fire-and-forget — a zero-subscriber send is a
    /// silent no-op (broadcast returns Err, which publish ignores).
    fn on_job_status(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
        self.publish(
            &topics::job_topic(id),
            topics::ServerEvent::new(
                "status",
                serde_json::json!({
                    "id": id,
                    "status": status,
                    "progress": progress,
                    "phase": phase,
                }),
            ),
        );
        self.publish(
            topics::jobs_topic(),
            topics::ServerEvent::new(
                "updated",
                serde_json::json!({ "id": id, "status": status }),
            ),
        );
    }

    /// Publish a progress tick to `jobs/{id}` (type `"progress"`).
    fn on_job_progress(&self, id: &str, status: &str, progress: f32, phase: Option<&str>) {
        self.publish(
            &topics::job_topic(id),
            topics::ServerEvent::new(
                "progress",
                serde_json::json!({
                    "id": id,
                    "status": status,
                    "progress": progress,
                    "phase": phase,
                }),
            ),
        );
    }
}

/// Client → server frames (contract §3). `#[serde(tag="op")]` internally-tagged.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum ClientFrame {
    Subscribe { topic: String },
    Unsubscribe { topic: String },
    Publish { topic: String, event: ServerEvent },
    Ping,
}
use actix_session::SessionExt;
use actix_web::{web, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use std::collections::HashSet;

/// Auth gate for the WS upgrade: mirror `public_validator` (session `logged_in`
/// OR empty `KOJI_SECRET` OR `?token=` query param == secret). Same-origin only.
fn ws_authorized(req: &HttpRequest) -> bool {
    let session = req.get_session();
    if session.get::<bool>("logged_in").ok().flatten().unwrap_or(false) {
        return true;
    }
    let secret = std::env::var("KOJI_SECRET").unwrap_or_default();
    if secret.is_empty() {
        return true;
    }
    // Bearer via `?token=` query param — browsers can't set Authorization headers
    // on WS upgrades, so the token is passed as a percent-encoded query value.
    // Parse + decode the query string properly so secrets containing `+`, `%`, `=`,
    // or spaces round-trip correctly before the constant-time compare.
    if let Some(tok) = url::form_urlencoded::parse(req.query_string().as_bytes())
        .find(|(k, _)| k == "token")
        .map(|(_, v)| v.into_owned())
    {
        if crate::utils::auth::ct_eq(&tok, &secret) {
            return true;
        }
    }
    false
}

pub async fn realtime_ws(
    req: HttpRequest,
    body: web::Payload,
    hub: web::Data<RealtimeHub>,
) -> Result<HttpResponse, actix_web::Error> {
    if !ws_authorized(&req) {
        return Ok(HttpResponse::Unauthorized().finish());
    }
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;
    let mut rx = hub.subscribe();
    actix_web::rt::spawn(async move {
        let mut topics: HashSet<String> = HashSet::new();
        loop {
            tokio::select! {
                // inbound client frames
                Some(Ok(msg)) = msg_stream.next() => {
                    match msg {
                        actix_ws::Message::Text(txt) => {
                            if let Ok(frame) = serde_json::from_str::<ClientFrame>(&txt) {
                                match frame {
                                    ClientFrame::Subscribe { topic } => { topics.insert(topic); }
                                    ClientFrame::Unsubscribe { topic } => { topics.remove(&topic); }
                                    ClientFrame::Ping => {
                                        if session.text(r#"{"op":"pong"}"#).await.is_err() { break; }
                                    }
                                    ClientFrame::Publish { topic, event } => {
                                        hub.publish(&topic, event);
                                    }
                                }
                            }
                        }
                        actix_ws::Message::Ping(bytes) => { let _ = session.pong(&bytes).await; }
                        actix_ws::Message::Close(_) => break,
                        _ => {}
                    }
                }
                // outbound hub events
                ev = rx.recv() => {
                    match ev {
                        Ok((topic, event)) => {
                            if topics.contains(&topic) {
                                let frame = serde_json::json!({
                                    "topic": topic,
                                    "type": event.r#type,
                                    "payload": event.payload,
                                    "meta": event.meta,
                                });
                                if session.text(frame.to_string()).await.is_err() { break; }
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
                else => break,
            }
        }
        let _ = session.close(None).await;
    });
    Ok(response)
}

#[cfg(test)]
mod hub_tests {
    use super::*;
    #[tokio::test]
    async fn publish_reaches_a_live_subscriber() {
        let hub = RealtimeHub::new();
        let mut rx = hub.subscribe();
        hub.publish("resource/geofence", topics::ServerEvent::new("created", serde_json::json!({"ids":[1]})));
        let (topic, ev) = rx.recv().await.unwrap();
        assert_eq!(topic, "resource/geofence");
        assert_eq!(ev.r#type, "created");
    }
    #[tokio::test]
    async fn publish_with_no_subscribers_does_not_panic() {
        let hub = RealtimeHub::new();
        hub.publish("jobs", topics::ServerEvent::new("updated", serde_json::json!({"id":"x","status":"queued"})));
    }
}
