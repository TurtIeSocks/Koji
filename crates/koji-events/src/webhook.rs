//! A generic [`Subscriber`] that delivers events to HTTP webhook endpoints
//! registered in the `webhook_subscription` table (events design spec,
//! "Crate surface" + "Dispatcher semantics").
//!
//! On `deliver` it loads the active `webhook_subscription` rows whose `topics`
//! match the event topic (`topics` is a JSON array of strings; an empty array
//! means "all topics"), then POSTs the raw JSON payload to each `url` with:
//!
//! - `Content-Type: application/json`
//! - `X-Koji-Event-Id: <event.id>` — the ULID, used by receivers to dedup
//!   re-deliveries (the dispatcher is per-event, so a retry re-hits
//!   already-succeeded subscribers).
//! - `X-Koji-Signature: sha256=<hex>` — when the row has a `secret`, an
//!   HMAC-SHA256 over the **raw request body**, hex-encoded.
//!
//! A non-2xx response or a transport error is a failure; *any* failure across
//! the matching rows makes `deliver` return `Err`, so the dispatcher retries the
//! whole event with backoff.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entity::webhook_subscription;
use crate::error::DeliverError;
use crate::subscriber::Subscriber;
use crate::types::Event;

/// The HTTP-event signature header name (`sha256=<hex hmac>`).
pub const SIGNATURE_HEADER: &str = "X-Koji-Signature";
/// The event-id header name (the event's ULID `public_id`, for receiver dedup).
pub const EVENT_ID_HEADER: &str = "X-Koji-Event-Id";

/// Delivers events to HTTP endpoints from the `webhook_subscription` registry.
///
/// Cheap to clone (both fields are handle-like).
#[derive(Clone)]
pub struct WebhookSubscriber {
    /// Shared HTTP client (connection pooling / timeouts configured by caller).
    http: reqwest::Client,
    /// Koji DB connection — reads the active subscription rows on each deliver.
    db: DatabaseConnection,
}

impl WebhookSubscriber {
    /// Build a subscriber over `db` with a caller-supplied `http` client (so
    /// timeouts / proxies are configured once at the edge).
    pub fn new(http: reqwest::Client, db: DatabaseConnection) -> Self {
        WebhookSubscriber { http, db }
    }

    /// Build a subscriber with a default [`reqwest::Client`].
    pub fn with_default_client(db: DatabaseConnection) -> Self {
        WebhookSubscriber {
            http: reqwest::Client::new(),
            db,
        }
    }

    /// Load every active subscription row. (Topic filtering happens in Rust —
    /// `topics` is a JSON array and an empty array means "all topics".)
    async fn active_subscriptions(&self) -> Result<Vec<webhook_subscription::Model>, DeliverError> {
        webhook_subscription::Entity::find()
            .filter(webhook_subscription::Column::Active.eq(true))
            .all(&self.db)
            .await
            .map_err(|e| DeliverError::Other(format!("loading webhook subscriptions: {e}")))
    }
}

/// Whether a subscription's `topics` JSON value matches `topic`.
///
/// Rules (spec): an empty array (or a non-array / null value) means "all
/// topics"; otherwise the array must contain `topic` as a string element.
fn topics_match(topics: &serde_json::Value, topic: &str) -> bool {
    match topics.as_array() {
        // Empty list ⇒ subscribe to everything.
        Some(arr) if arr.is_empty() => true,
        Some(arr) => arr.iter().any(|t| t.as_str() == Some(topic)),
        // Non-array (shouldn't happen given the column is a JSON array, but be
        // permissive): treat as "all".
        None => true,
    }
}

/// Compute the `X-Koji-Signature` value for `body` under `secret`:
/// `sha256=<hex hmac-sha256(secret, body)>`.
pub fn sign(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    // `new_from_slice` only errors for key sizes HMAC rejects; HMAC accepts any
    // key length, so this never fails in practice.
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts keys of any length");
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    format!("sha256={}", hex::encode(digest))
}

#[async_trait]
impl Subscriber for WebhookSubscriber {
    fn name(&self) -> &'static str {
        "webhook"
    }

    async fn deliver(&self, event: &Event) -> Result<(), DeliverError> {
        let subs = self.active_subscriptions().await?;

        // Serialize the payload once: the HMAC must be computed over the exact
        // bytes we send, so we reuse this String as the request body.
        let body = serde_json::to_string(&event.payload)
            .map_err(|e| DeliverError::Other(format!("serializing event payload: {e}")))?;

        let matching = subs
            .iter()
            .filter(|s| topics_match(&s.topics, &event.topic));

        let mut delivered = 0usize;
        for sub in matching {
            delivered += 1;
            let mut req = self
                .http
                .post(&sub.url)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .header(EVENT_ID_HEADER, &event.id)
                .body(body.clone());

            if let Some(secret) = sub.secret.as_deref() {
                req = req.header(SIGNATURE_HEADER, sign(secret, body.as_bytes()));
            }

            let resp = req
                .send()
                .await
                .map_err(|e| DeliverError::Http(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                return Err(DeliverError::Status {
                    status: status.as_u16(),
                });
            }
        }

        log::debug!(
            "[koji-events] webhook delivered event {} (topic={}) to {} subscription(s)",
            event.id,
            event.topic,
            delivered
        );
        Ok(())
    }
}
