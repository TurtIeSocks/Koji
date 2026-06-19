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

/// Client → server frames (contract §3). `#[serde(tag="op")]` internally-tagged.
#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum ClientFrame {
    Subscribe { topic: String },
    Unsubscribe { topic: String },
    Publish { topic: String, event: ServerEvent },
    Ping,
}
// `ServerEvent` needs Deserialize for the `publish` frame; add `#[derive(Deserialize)]`
// to it in topics.rs alongside Serialize (and `#[serde(default)]` on payload/meta).

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
