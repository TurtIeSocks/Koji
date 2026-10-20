//! Dragonite event subscriber + the domain-event payloads it consumes.
//!
//! This is the composition-root half of the events→Dragonite path (architecture
//! §7). The generic outbox/dispatcher/`Subscriber` machinery lives in
//! `koji-events` (which knows nothing of Dragonite); the typed `/v2/areas`
//! client lives in `koji-dragonite`. [`DragoniteSubscriber`] joins them: it
//! claims `area.*` events and PATCHes the linked Dragonite area.
//!
//! ## Payload contract
//! A Dragonite is a `webhook_subscription` row in `dragonite` mode. Producers
//! (the `POST /geofences/:id/publish` and route publish endpoints) emit one
//! event per such row that reaches the geofence and holds a link for it. The
//! payload names the row (`webhook_id`), that row's Area (`dragonite_area_id`)
//! and the target [`AreaMode`]. The subscriber reads the row at delivery, so a
//! changed URL or bearer applies to events already queued, builds its client
//! from it, and maps the payload onto a PATCH:
//!
//! - [`TOPIC_ROUTE_UPDATED`] → [`RouteUpdated`] → `area_route_patch`
//! - [`TOPIC_GEOFENCE_UPDATED`] → [`GeofenceUpdated`] → `area_geofence_patch`

use async_trait::async_trait;
use geojson::Feature;
use koji_core::SingleVec;
use koji_db::db::sea_orm_active_enums::Mode;
use koji_db::db::{dragonite_link, sea_orm_active_enums::WebhookMode, webhook};
use koji_dragonite::{
    AreaMode, DragoniteClient, DragoniteError, area_geofence_patch, area_route_patch,
};
use koji_events::{DeliverError, Event, EventDispatcher, Subscriber};
use sea_orm::{ConnectionTrait, DatabaseConnection, EntityTrait};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::utils::error::ServiceError;

/// Topic for "a mode's route was (re)calculated" — payload [`RouteUpdated`].
pub(crate) const TOPIC_ROUTE_UPDATED: &str = "area.route_updated";
/// Topic for "a mode's geofence was published/updated" — payload [`GeofenceUpdated`].
pub(crate) const TOPIC_GEOFENCE_UPDATED: &str = "area.geofence_updated";

/// Map a Koji route/geofence [`Mode`] to the Dragonite [`AreaMode`] it
/// publishes to. Shared by route and geofence publish. The 12→4 collapse
/// already folded the historical variants, so this is a direct 1:1 mapping
/// (`unset` → `Base`, the area-root `geofence` column).
pub(crate) fn area_mode_for(mode: &Mode) -> AreaMode {
    match mode {
        Mode::Quest => AreaMode::Quest,
        Mode::Pokemon => AreaMode::Pokemon,
        Mode::Fort => AreaMode::Fort,
        Mode::Unset => AreaMode::Base,
    }
}

/// Payload for [`TOPIC_ROUTE_UPDATED`]. `route` is a Koji [`SingleVec`]
/// (`[lat, lon]` points). `Base` has no route slot, so the subscriber drops it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RouteUpdated {
    /// The Dragonite row this event is for.
    pub webhook_id: u64,
    pub dragonite_area_id: i64,
    pub mode: AreaMode,
    pub route: SingleVec,
}

/// Payload for [`TOPIC_GEOFENCE_UPDATED`]. `geofence` is the GeoJSON `Feature`
/// to set on `mode`'s fence (`Base` → the area-root fence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeofenceUpdated {
    /// The Dragonite row this event is for.
    pub webhook_id: u64,
    pub dragonite_area_id: i64,
    pub mode: AreaMode,
    pub geofence: Feature,
}

/// The client for a Dragonite row: `url` is the base URL, `secret` the bearer
/// (none is sent as an empty bearer, which Dragonite refuses).
pub(crate) fn client_for(target: &webhook::Model) -> DragoniteClient {
    client_from(&target.url, target.secret.as_deref())
}

/// [`client_for`] over a row's `url` and `secret`.
pub(crate) fn client_from(url: &str, secret: Option<&str>) -> DragoniteClient {
    DragoniteClient::new(url, secret.unwrap_or_default())
}

/// The Dragonite rows to publish `geofence_id` (called `name`) to, each with its
/// link: the active rows that reach the geofence and hold a link for it.
///
/// `422` when there are none, saying which of the two is missing: no Dragonite
/// row reaches the geofence, or the rows that do hold no link for it.
pub(crate) async fn linked_targets<C: ConnectionTrait>(
    db: &C,
    geofence_id: u32,
    name: &str,
) -> Result<Vec<(webhook::Model, dragonite_link::Model)>, ServiceError> {
    let found = dragonite_link::Query::publish_targets(db, geofence_id).await?;
    if found.reaching.is_empty() {
        return Err(ServiceError::Unprocessable {
            field: Some("dragonite_webhook".to_string()),
            message: no_target_message(name),
        });
    }
    if found.linked.is_empty() {
        return Err(ServiceError::Unprocessable {
            field: Some("dragonite_links".to_string()),
            message: no_link_message(name, &found.reaching),
        });
    }
    Ok(found.linked)
}

/// Why geofence `name` sends nothing when no Dragonite row reaches it.
pub(crate) fn no_target_message(name: &str) -> String {
    format!("no active Dragonite webhook reaches geofence `{name}`")
}

/// Why geofence `name` sends nothing when the rows `reaching` it hold no link.
pub(crate) fn no_link_message(name: &str, reaching: &[webhook::Model]) -> String {
    let names = reaching
        .iter()
        .map(|t| format!("`{}` (id {})", t.name, t.id))
        .collect::<Vec<_>>()
        .join(", ");
    format!("geofence `{name}` has no Dragonite area link on the webhooks that reach it: {names}")
}

/// Append `payload_for(target, link)` as one `topic` event per linked target and
/// answer them as `[{ webhook_id, event_id, dragonite_area_id }]`.
pub(crate) async fn publish_to<P, F>(
    db: &DatabaseConnection,
    topic: &str,
    targets: &[(webhook::Model, dragonite_link::Model)],
    payload_for: F,
) -> Result<serde_json::Value, ServiceError>
where
    P: Serialize,
    F: Fn(&webhook::Model, &dragonite_link::Model) -> P,
{
    let mut events = Vec::with_capacity(targets.len());
    for (target, link) in targets {
        let event_id = EventDispatcher::publish(db, topic, &payload_for(target, link))
            .await
            .map_err(ServiceError::internal)?;
        events.push(json!({
            "webhook_id": target.id,
            "event_id": event_id.to_string(),
            "dragonite_area_id": link.dragonite_area_id,
        }));
    }
    Ok(json!(events))
}

/// Pushes Koji's calculated routes/geofences to the Dragonite each event names,
/// at its `/v2/areas/{id}`.
pub(crate) struct DragoniteSubscriber {
    db: DatabaseConnection,
}

impl DragoniteSubscriber {
    pub(crate) fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    /// The client for the row `webhook_id`, or `None` when the row is gone, no
    /// longer a Dragonite, or inactive: an inactive row reaches nothing, so its
    /// queued events are dropped, not retried.
    async fn client(&self, webhook_id: u64) -> Result<Option<DragoniteClient>, DeliverError> {
        let target = webhook::Entity::find_by_id(webhook_id)
            .one(&self.db)
            .await
            .map_err(|e| DeliverError::Other(format!("loading webhook {webhook_id}: {e}")))?;
        Ok(target
            .filter(|t| t.active && t.mode == WebhookMode::Dragonite)
            .map(|t| client_for(&t)))
    }
}

/// Map a [`DragoniteError`] onto the subscriber-side [`DeliverError`] so the
/// dispatcher's backoff/dead-letter logic sees the right category. A non-2xx
/// carries its status, `detail` and `errors` in the message, which the
/// dispatcher records as the outbox row's `last_error`.
fn to_deliver_error(e: DragoniteError) -> DeliverError {
    match e {
        DragoniteError::Http(err) => DeliverError::Http(err.to_string()),
        e @ DragoniteError::HttpStatus { .. } => DeliverError::Other(e.to_string()),
        DragoniteError::Decode(m) => DeliverError::Other(format!("decode: {m}")),
    }
}

#[async_trait]
impl Subscriber for DragoniteSubscriber {
    fn name(&self) -> &'static str {
        "dragonite"
    }

    fn interested_in(&self, topic: &str) -> bool {
        topic == TOPIC_ROUTE_UPDATED || topic == TOPIC_GEOFENCE_UPDATED
    }

    async fn deliver(&self, event: &Event) -> Result<(), DeliverError> {
        match event.topic.as_str() {
            TOPIC_ROUTE_UPDATED => {
                let p: RouteUpdated =
                    serde_json::from_value(event.payload.clone()).map_err(|e| {
                        DeliverError::Other(format!("invalid {TOPIC_ROUTE_UPDATED} payload: {e}"))
                    })?;
                let Some(client) = self.client(p.webhook_id).await? else {
                    log::info!(
                        "[koji] dropping {TOPIC_ROUTE_UPDATED} for webhook {}: not an active Dragonite",
                        p.webhook_id
                    );
                    return Ok(());
                };
                let patch = area_route_patch(p.mode, &p.route);
                client
                    .patch_area(p.dragonite_area_id, &patch)
                    .await
                    .map_err(to_deliver_error)?;
                Ok(())
            }
            TOPIC_GEOFENCE_UPDATED => {
                let p: GeofenceUpdated =
                    serde_json::from_value(event.payload.clone()).map_err(|e| {
                        DeliverError::Other(format!(
                            "invalid {TOPIC_GEOFENCE_UPDATED} payload: {e}"
                        ))
                    })?;
                let Some(client) = self.client(p.webhook_id).await? else {
                    log::info!(
                        "[koji] dropping {TOPIC_GEOFENCE_UPDATED} for webhook {}: not an active Dragonite",
                        p.webhook_id
                    );
                    return Ok(());
                };
                let patch = area_geofence_patch(p.mode, &p.geofence);
                client
                    .patch_area(p.dragonite_area_id, &patch)
                    .await
                    .map_err(to_deliver_error)?;
                Ok(())
            }
            // interested_in gates the topics above; anything else is a no-op.
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_dragonite::DragoniteError;

    #[test]
    fn route_payload_round_trips() {
        let p = RouteUpdated {
            webhook_id: 3,
            dragonite_area_id: 42,
            mode: AreaMode::Quest,
            route: vec![[40.1, -75.2], [40.3, -75.4]],
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["webhook_id"], 3);
        assert_eq!(v["dragonite_area_id"], 42);
        assert_eq!(v["mode"], "quest");
        let back: RouteUpdated = serde_json::from_value(v).unwrap();
        assert_eq!(back.route.len(), 2);
        assert_eq!(back.dragonite_area_id, 42);
        assert_eq!(back.webhook_id, 3);
    }

    #[test]
    fn http_status_error_keeps_status_detail_and_errors_in_the_message() {
        let e = DragoniteError::HttpStatus {
            status: 422,
            detail: Some("validation failed".to_string()),
            errors: vec![koji_dragonite::ProblemError {
                message: Some("expected boolean".to_string()),
                location: Some("body.enabled".to_string()),
                value: Some(serde_json::json!("x")),
            }],
            body: String::new(),
        };
        let DeliverError::Other(message) = to_deliver_error(e) else {
            panic!("expected Other");
        };
        assert!(message.contains("422"), "{message}");
        assert!(message.contains("validation failed"), "{message}");
        assert!(
            message.contains("body.enabled: expected boolean"),
            "{message}"
        );
    }

    #[test]
    fn http_status_error_without_a_problem_document_keeps_the_body() {
        let e = DragoniteError::HttpStatus {
            status: 404,
            detail: None,
            errors: Vec::new(),
            body: "not found".to_string(),
        };
        let DeliverError::Other(message) = to_deliver_error(e) else {
            panic!("expected Other");
        };
        assert!(message.contains("404"), "{message}");
        assert!(message.contains("not found"), "{message}");
    }

    #[test]
    fn decode_error_maps_to_other() {
        let e = DragoniteError::Decode("bad body".to_string());
        assert!(matches!(to_deliver_error(e), DeliverError::Other(_)));
    }

    #[test]
    fn interested_only_in_area_topics() {
        let s = DragoniteSubscriber::new(DatabaseConnection::Disconnected);
        assert!(s.interested_in(TOPIC_ROUTE_UPDATED));
        assert!(s.interested_in(TOPIC_GEOFENCE_UPDATED));
        assert!(!s.interested_in("webhook.test"));
    }

    // ── outbox delivery against a Dragonite stand-in ─────────────────────────

    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use koji_events::EventDispatcher;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, DbBackend, Set, Statement, Value};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// What Dragonite 1.19.0 answers a `PATCH /v2/areas/2` with: the bare area.
    const BARE_AREA: &str = r#"{"$schema":"https://dragonite.example/schemas/ApiAreaV2.json","id":2,"name":"CT Ansonia","enabled":false,"pokemon_mode":{},"quest_mode":{},"fort_mode":{}}"#;

    /// Answer every request on a loopback port with `200` and the bare area,
    /// counting them. Returns the base URL and the request counter.
    async fn dragonite_stand_in() -> (String, Arc<AtomicUsize>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let counter = Arc::clone(&counter);
                tokio::spawn(async move {
                    let mut req = Vec::new();
                    let mut buf = [0u8; 4096];
                    // Read to the end of the head, then the declared body.
                    loop {
                        let n = sock.read(&mut buf).await.unwrap_or(0);
                        req.extend_from_slice(&buf[..n]);
                        let text = String::from_utf8_lossy(&req).to_ascii_lowercase();
                        if let Some(head_end) = text.find("\r\n\r\n") {
                            let want = text[..head_end]
                                .lines()
                                .find_map(|l| l.strip_prefix("content-length:"))
                                .and_then(|v| v.trim().parse::<usize>().ok())
                                .unwrap_or(0);
                            if req.len() >= head_end + 4 + want {
                                break;
                            }
                        }
                        if n == 0 {
                            break;
                        }
                    }
                    counter.fetch_add(1, Ordering::SeqCst);
                    let reply = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{BARE_AREA}",
                        BARE_AREA.len()
                    );
                    let _ = sock.write_all(reply.as_bytes()).await;
                    let _ = sock.shutdown().await;
                });
            }
        });
        (base, hits)
    }

    /// A geofence publish whose PATCH gets the bare area is delivered on the
    /// first attempt: one PATCH, `delivered`, no retry, no recorded error.
    /// Gated on `KOJI_DB_URL` like the other DB tests (skips when unset).
    #[tokio::test]
    async fn geofence_publish_answered_with_the_bare_area_is_delivered_first_try() {
        let Ok(url) = std::env::var("KOJI_DB_URL") else {
            eprintln!("skip: KOJI_DB_URL unset");
            return;
        };
        let Ok(db) = Database::connect(&url).await else {
            eprintln!("skip: could not connect to KOJI_DB_URL");
            return;
        };
        let (base, hits) = dragonite_stand_in().await;

        let target = webhook::ActiveModel {
            name: Set("dragonite stub".into()),
            url: Set(base),
            secret: Set(Some("tok".into())),
            topics: Set(serde_json::json!([])),
            active: Set(true),
            mode: Set(WebhookMode::Dragonite),
            method: Set(koji_db::db::sea_orm_active_enums::WebhookMethod::Post),
            created_at: Set(Default::default()),
            updated_at: Set(Default::default()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert the Dragonite row");

        let payload = GeofenceUpdated {
            webhook_id: target.id,
            dragonite_area_id: 2,
            mode: AreaMode::Pokemon,
            geofence: Feature {
                bbox: None,
                geometry: Some(
                    serde_json::from_value(serde_json::json!({
                        "type": "Polygon",
                        "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
                    }))
                    .unwrap(),
                ),
                id: None,
                properties: None,
                foreign_members: None,
            },
        };
        let event_id = EventDispatcher::publish(&db, TOPIC_GEOFENCE_UPDATED, &payload)
            .await
            .expect("append to the outbox");

        let dispatcher = Arc::new(EventDispatcher::new(
            db.clone(),
            vec![Arc::new(DragoniteSubscriber::new(db.clone()))],
            "dragonite-test",
        ));
        let handle = dispatcher.spawn();

        let read_row = || async {
            let row = db
                .query_one(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "SELECT `status`, `attempts`, `last_error` FROM `event_outbox` \
                     WHERE `public_id` = ?",
                    [Value::from(event_id.as_string())],
                ))
                .await
                .unwrap()
                .expect("outbox row");
            (
                row.try_get::<String>("", "status").unwrap(),
                row.try_get::<i32>("", "attempts").unwrap(),
                row.try_get::<Option<String>>("", "last_error").unwrap(),
            )
        };
        let mut seen = read_row().await;
        for _ in 0..100 {
            if seen.0 != "pending" && seen.0 != "delivering" {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            seen = read_row().await;
        }
        handle.shutdown().await;
        let _ = webhook::Entity::delete_by_id(target.id).exec(&db).await;
        let _ = db
            .execute(Statement::from_sql_and_values(
                DbBackend::MySql,
                "DELETE FROM `event_outbox` WHERE `public_id` = ?",
                [Value::from(event_id.as_string())],
            ))
            .await;

        assert_eq!(seen, ("delivered".to_string(), 0, None));
        assert_eq!(hits.load(Ordering::SeqCst), 1, "one PATCH, no retry");
    }
}
