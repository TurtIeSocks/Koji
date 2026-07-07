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

/// Emit `project.geofences_changed` for every project whose membership diff
/// (before vs after) is non-empty. `before`/`after` map project_id → geofence
/// ids linked through it. One event per project per operation — a bulk
/// mutation touching N fences across a handful of projects still emits only
/// as many events as projects actually changed, never N.
pub(crate) async fn emit_membership_diff(
    db: &sea_orm::DatabaseConnection,
    before: &std::collections::HashMap<u32, std::collections::BTreeSet<u32>>,
    after: &std::collections::HashMap<u32, std::collections::BTreeSet<u32>>,
) {
    let projects: std::collections::BTreeSet<u32> =
        before.keys().chain(after.keys()).copied().collect();
    for pid in projects {
        let empty = std::collections::BTreeSet::new();
        let b = before.get(&pid).unwrap_or(&empty);
        let a = after.get(&pid).unwrap_or(&empty);
        let added: Vec<u32> = a.difference(b).copied().collect();
        let removed: Vec<u32> = b.difference(a).copied().collect();
        if added.is_empty() && removed.is_empty() {
            continue;
        }
        emit_event(
            db,
            "project.geofences_changed",
            serde_json::json!({ "projectId": pid, "addedIds": added, "removedIds": removed }),
        )
        .await;
    }
}
