//! Outbox emission helper for request handlers: publish-or-warn, never fail
//! the request (a missed webhook ping is not data loss).

/// Publish `payload` to the `koji_events` outbox under `topic`. Errors are
/// logged (`log::warn!`) and swallowed — a handler calling this never fails
/// the request because of an outbox write failure.
pub(crate) async fn emit_event(
    db: &sea_orm::DatabaseConnection,
    topic: &str,
    payload: serde_json::Value,
) {
    if let Err(e) = koji_events::EventDispatcher::publish(db, topic, &payload).await {
        log::warn!("[outbox] publish {topic} failed: {e}");
    }
}
