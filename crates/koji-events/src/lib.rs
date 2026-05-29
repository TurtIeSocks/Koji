//! # koji-events
//!
//! Durable domain-event delivery for Koji V2 (design:
//! `docs/superpowers/specs/2026-05-29-koji-v2-p3b-events-design.md`).
//!
//! Producers append events to an **outbox** table
//! ([`EventDispatcher::publish`]); a **dispatcher** worker claims due rows
//! (`FOR UPDATE SKIP LOCKED` + a 60s lease / 20s heartbeat, mirroring
//! `koji-jobs`), delivers each to every interested [`Subscriber`], retries with
//! exponential backoff, and dead-letters after `max_attempts`. The generic
//! [`WebhookSubscriber`] reads a `webhook_subscription` registry and POSTs the
//! payload with an HMAC-SHA256 signature.
//!
//! This crate is **additive and unwired** — nothing imports it yet. The concrete
//! `DragoniteSubscriber` and the actual event emission wire in later phases.
//!
//! ## Shape
//! - [`EventDispatcher`] — `publish` (outbox append) + `spawn`
//!   (claim→deliver→backoff loop → [`DispatcherHandle`]).
//! - [`Subscriber`] — pluggable delivery sink (`deliver` + `interested_in`).
//! - [`WebhookSubscriber`] — HTTP webhook delivery from `webhook_subscription`.
//! - [`Event`] / [`EventId`] — the value passed to subscribers + the outbox id.
//!
//! ## What is NOT verified here
//! There is no database (and no live HTTP endpoint) in this crate's tests. The
//! claim/lease/heartbeat, the backoff *persistence*, the dispatch loop, and the
//! webhook HTTP round-trip are exercised only against a live MySQL/MariaDB + a
//! real receiver during maintainer integration testing. The unit tests here
//! cover only DB-free logic (HMAC signature stability and the backoff schedule).

pub mod dispatcher;
pub mod entity;
pub mod error;
pub mod subscriber;
pub mod types;
pub mod webhook;

// ---- Public surface ------------------------------------------------------

pub use dispatcher::{DispatcherHandle, EventDispatcher};
pub use entity::EventStatus;
pub use error::{DeliverError, DispatchError, PublishError};
pub use subscriber::Subscriber;
pub use types::{Event, EventId};
pub use webhook::WebhookSubscriber;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::webhook::sign;
    use std::str::FromStr;

    #[test]
    fn event_id_ulid_round_trips_through_string() {
        let id = EventId::new();
        let s = id.to_string();
        // Canonical ULID encoding is always 26 chars (Crockford base32).
        assert_eq!(s.len(), 26, "ULID string must be 26 chars, got {s:?}");

        let parsed = EventId::from_str(&s).expect("round-trip parse should succeed");
        assert_eq!(parsed, id, "FromStr(Display(id)) must equal id");
        assert_eq!(parsed.as_string(), s);
    }

    #[test]
    fn event_id_rejects_malformed_strings() {
        assert!(EventId::from_str("not-a-ulid").is_err());
        assert!(EventId::from_str("").is_err());
    }

    #[test]
    fn event_status_terminal_classification() {
        assert!(EventStatus::Delivered.is_terminal());
        assert!(EventStatus::Dead.is_terminal());
        assert!(!EventStatus::Pending.is_terminal());
        assert!(!EventStatus::Delivering.is_terminal());
    }

    // ---- HMAC-SHA256 signature stability --------------------------------
    //
    // Known key + known body → known hex. The expected digest is the canonical
    // RFC 4231-style HMAC-SHA256 of "The quick brown fox jumps over the lazy
    // dog" under the key "key" — a widely published test vector.
    #[test]
    fn hmac_signature_matches_known_vector() {
        let sig = sign("key", b"The quick brown fox jumps over the lazy dog");
        assert_eq!(
            sig,
            "sha256=f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        );
    }

    #[test]
    fn hmac_signature_is_stable_and_prefixed() {
        let a = sign(
            "topsecret",
            br#"{"area":"geofence:42","topic":"area.route_updated"}"#,
        );
        let b = sign(
            "topsecret",
            br#"{"area":"geofence:42","topic":"area.route_updated"}"#,
        );
        assert_eq!(a, b, "same key + body must sign identically");
        assert!(
            a.starts_with("sha256="),
            "signature must carry the algo prefix"
        );
        // 64 hex chars after the `sha256=` prefix (32-byte SHA-256 digest).
        let hex = a.strip_prefix("sha256=").unwrap();
        assert_eq!(hex.len(), 64);
        assert!(hex.bytes().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn hmac_signature_changes_with_key_and_body() {
        let base = sign("key", b"body");
        assert_ne!(
            base,
            sign("other-key", b"body"),
            "different key → different sig"
        );
        assert_ne!(
            base,
            sign("key", b"other-body"),
            "different body → different sig"
        );
    }
}
