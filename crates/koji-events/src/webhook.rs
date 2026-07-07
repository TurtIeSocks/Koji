//! A generic [`Subscriber`] that delivers events to HTTP webhook endpoints
//! registered in the `webhook_subscription` table (events design spec,
//! "Crate surface" + "Dispatcher semantics").
//!
//! On `deliver` it loads the active `webhook_subscription` rows whose `topics`
//! match the event topic (`topics` is a JSON array of strings; an empty array
//! means "all topics") AND whose `project_id` scope matches the event's
//! `projectId`/`projectIds` (NULL = global subscription, always matches).
//!
//! Each matching subscription is delivered in its configured `mode`:
//!
//! **Event mode** (`mode == "event"`): POSTs the raw JSON payload to the
//! subscription's `url` with:
//! - `Content-Type: application/json`
//! - `X-Koji-Event-Id: <event.id>` — the ULID, used by receivers to dedup
//!   re-deliveries (the dispatcher is per-event, so a retry re-hits
//!   already-succeeded subscribers).
//! - `X-Koji-Signature: sha256=<hex>` — when the row has a `secret`, an
//!   HMAC-SHA256 over the **raw request body**, hex-encoded.
//!
//! **Ping mode** (`mode == "ping"`): Sends an HTTP request (GET or POST per the
//! subscription's `method` field) with an empty body, no signature, but still
//! includes `X-Koji-Event-Id`.
//!
//! Custom headers (`headers` JSON object) are applied in both modes: string
//! values are sent, non-string values are skipped.
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

    /// Deliver `event` to ONE subscription, mode-aware. Returns the upstream
    /// HTTP status on success. Public so the service's `/test` endpoint can
    /// fire a single subscription outside the dispatcher loop.
    pub async fn deliver_to(
        &self,
        sub: &webhook_subscription::Model,
        event: &Event,
    ) -> Result<u16, DeliverError> {
        let mut req = if sub.mode == "ping" {
            // Legacy reload ping: consumer-chosen method, empty body, no HMAC.
            let method = if sub.method.eq_ignore_ascii_case("POST") {
                reqwest::Method::POST
            } else {
                reqwest::Method::GET
            };
            self.http.request(method, &sub.url)
        } else {
            // Signed event POST: HMAC over the exact bytes sent.
            let body = serde_json::to_string(&event.payload)
                .map_err(|e| DeliverError::Other(format!("serializing event payload: {e}")))?;
            let mut req = self
                .http
                .post(&sub.url)
                .header(reqwest::header::CONTENT_TYPE, "application/json");
            if let Some(secret) = sub.secret.as_deref() {
                req = req.header(SIGNATURE_HEADER, sign(secret, body.as_bytes()));
            }
            req.body(body)
        };
        req = req.header(EVENT_ID_HEADER, &event.id);
        // Custom headers apply in BOTH modes (ping auth, proxies in front of
        // event consumers). Non-string values are skipped, not stringified.
        if let Some(headers) = sub.headers.as_ref().and_then(|h| h.as_object()) {
            for (k, v) in headers {
                if let Some(v) = v.as_str() {
                    req = req.header(k.as_str(), v);
                }
            }
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
        Ok(status.as_u16())
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

/// Whether a subscription's project scope matches the event payload.
///
/// `None` = global subscription → always true. `Some(pid)` matches when the
/// payload's `projectIds` array contains `pid`, or its scalar `projectId`
/// equals `pid`. Events with neither key never match a scoped subscription.
fn project_matches(sub_project_id: Option<u32>, payload: &serde_json::Value) -> bool {
    let Some(pid) = sub_project_id else {
        return true;
    };
    let pid = pid as u64;
    if payload.get("projectId").and_then(serde_json::Value::as_u64) == Some(pid) {
        return true;
    }
    payload
        .get("projectIds")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|arr| arr.iter().any(|v| v.as_u64() == Some(pid)))
}

/// Compute the `X-Koji-Signature` value for `body` under `secret`:
/// `sha256=<hex hmac-sha256(secret, body)>`.
// pub(crate) is enough for tests inside the module; re-exported via `use
// crate::webhook::sign` in lib.rs tests.
pub(crate) fn sign(secret: &str, body: &[u8]) -> String {
    use hmac::{Hmac, KeyInit, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;
    // `new_from_slice` only errors for key sizes HMAC rejects; HMAC accepts any
    // key length, so this never fails in practice.
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
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
        let matching = subs.iter().filter(|s| {
            topics_match(&s.topics, &event.topic) && project_matches(s.project_id, &event.payload)
        });
        let mut delivered = 0usize;
        for sub in matching {
            delivered += 1;
            // ponytail: all-or-nothing retry per event across subscribers (one flaky
            // consumer re-pings the rest); per-subscription delivery rows if that
            // ever matters — pings are idempotent, so it doesn't today.
            self.deliver_to(sub, event).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- HMAC signing -------------------------------------------------------

    /// Canonical RFC 4231 / GitHub-webhook-style vector: HMAC-SHA256("key",
    /// "The quick brown fox jumps over the lazy dog").
    /// Expected value cross-checked with Python `hmac.new(b"key", msg,
    /// hashlib.sha256).hexdigest()`.
    #[test]
    fn sign_known_vector() {
        let sig = sign("key", b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            sig,
            "sha256=f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    /// Cross-checked with Python: HMAC-SHA256("secret", json_body).
    #[test]
    fn sign_json_body_known_vector() {
        let body = br#"{"id":"01HZQ","topic":"area.updated","payload":{}}"#;
        let sig = sign("secret", body);
        assert_eq!(
            sig,
            "sha256=74531c84486f8fe6c8a526196c1a6ec50128fa7a1b582df39ca00f05b9fd1b74"
        );
    }

    #[test]
    fn sign_empty_key_known_vector() {
        let sig = sign("", b"empty-key-test");
        assert_eq!(
            sig,
            "sha256=a8d7bcbc5e8637e13d5fe915f8e8e0258e5c158a195098b7e0e0afb6cf241c24"
        );
    }

    #[test]
    fn sign_empty_body_known_vector() {
        let sig = sign("x", b"");
        assert_eq!(
            sig,
            "sha256=f27e6527d6b8408430a666b746070c307f542bb54ee7e6dcb303f3e52c0b09fb"
        );
    }

    #[test]
    fn sign_is_deterministic() {
        let body = b"some event payload";
        let a = sign("my-secret", body);
        let b = sign("my-secret", body);
        assert_eq!(a, b);
    }

    #[test]
    fn sign_output_has_sha256_prefix() {
        let sig = sign("k", b"payload");
        assert!(sig.starts_with("sha256="), "must carry algo prefix: {sig}");
    }

    #[test]
    fn sign_output_has_64_hex_chars_after_prefix() {
        let sig = sign("k", b"payload");
        let hex = sig.strip_prefix("sha256=").unwrap();
        assert_eq!(hex.len(), 64);
        assert!(hex.bytes().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sign_different_key_gives_different_sig() {
        let a = sign("key-one", b"same body");
        let b = sign("key-two", b"same body");
        assert_ne!(a, b, "distinct keys must produce distinct MACs");
    }

    #[test]
    fn sign_different_body_gives_different_sig() {
        let a = sign("same-key", b"body-one");
        let b = sign("same-key", b"body-two");
        assert_ne!(a, b, "distinct bodies must produce distinct MACs");
    }

    /// Simulates a one-bit flip in the body — signature must change.
    #[test]
    fn sign_one_byte_diff_in_body_changes_sig() {
        let a = sign("k", b"aaaaaa");
        let b = sign("k", b"aaaaab");
        assert_ne!(a, b);
    }

    // ---- topics_match -------------------------------------------------------

    #[test]
    fn topics_match_empty_array_is_wildcard() {
        let topics = json!([]);
        assert!(topics_match(&topics, "area.updated"));
        assert!(topics_match(&topics, "anything"));
    }

    #[test]
    fn topics_match_exact_string_element() {
        let topics = json!(["area.updated", "area.deleted"]);
        assert!(topics_match(&topics, "area.updated"));
        assert!(topics_match(&topics, "area.deleted"));
        assert!(!topics_match(&topics, "area.created"));
    }

    #[test]
    fn topics_match_null_treated_as_wildcard() {
        // A non-array JSON value (null) → spec says treat as "all topics".
        let topics = serde_json::Value::Null;
        assert!(topics_match(&topics, "whatever"));
    }

    #[test]
    fn topics_match_non_array_object_treated_as_wildcard() {
        let topics = json!({"oops": "someone stored an object"});
        assert!(topics_match(&topics, "area.updated"));
    }

    #[test]
    fn topics_match_single_element_list() {
        let topics = json!(["area.route_updated"]);
        assert!(topics_match(&topics, "area.route_updated"));
        assert!(!topics_match(&topics, "area.updated"));
    }

    #[test]
    fn topics_match_is_exact_not_prefix() {
        // "area" must NOT match "area.updated" — no prefix / glob logic.
        let topics = json!(["area"]);
        assert!(!topics_match(&topics, "area.updated"));
    }

    // ---- header constants ---------------------------------------------------

    #[test]
    fn header_name_constants() {
        assert_eq!(SIGNATURE_HEADER, "X-Koji-Signature");
        assert_eq!(EVENT_ID_HEADER, "X-Koji-Event-Id");
    }

    // ---- project_matches ----------------------------------------------------

    #[test]
    fn project_matches_none_is_global() {
        assert!(project_matches(None, &json!({"projectIds": [7]})));
        assert!(project_matches(None, &json!({})));
    }

    #[test]
    fn project_matches_in_project_ids_array() {
        assert!(project_matches(Some(7), &json!({"projectIds": [3, 7]})));
        assert!(!project_matches(Some(8), &json!({"projectIds": [3, 7]})));
    }

    #[test]
    fn project_matches_scalar_project_id() {
        assert!(project_matches(Some(7), &json!({"projectId": 7})));
        assert!(!project_matches(Some(7), &json!({"projectId": 8})));
    }

    #[test]
    fn project_matches_no_project_keys_means_no_match_for_scoped_sub() {
        // A scoped subscription must NOT fire for events that carry no project info
        // (e.g. a future global topic) — scoping is opt-in per event.
        assert!(!project_matches(Some(7), &json!({"something": "else"})));
    }
}
