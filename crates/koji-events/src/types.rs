//! Public value types for the event outbox (events design spec, "Crate
//! surface").
//!
//! - [`EventId`] — the opaque, sortable, API-facing identifier (`public_id`,
//!   ULID-backed). Mirrors `koji_jobs::JobId`.
//! - [`Event`] — what a [`crate::Subscriber`] receives: the event id, topic, and
//!   raw JSON payload.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use ulid::Ulid;

/// Opaque, sortable, API-facing event identifier — the `public_id` column.
///
/// Backed by a ULID (Crockford base32, 26 chars): lexicographically sortable by
/// creation time and safe to expose to clients (unlike the auto-increment
/// `BIGINT` PK). `Display`/`FromStr` round-trip through the canonical ULID
/// string, and serde (de)serializes it as that same 26-char string — so the
/// JSON wire form is the opaque id clients see, never the raw u128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(pub Ulid);

impl Serialize for EventId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for EventId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ulid::from_string(&s)
            .map(EventId)
            .map_err(|e| D::Error::custom(format!("invalid ULID: {e}")))
    }
}

impl EventId {
    /// Generate a fresh, time-ordered id.
    pub fn new() -> Self {
        EventId(Ulid::new())
    }

    /// The canonical 26-char ULID string (exactly what is stored in
    /// `public_id`).
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for EventId {
    type Err = ulid::DecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(EventId(Ulid::from_string(s)?))
    }
}

/// A domain event handed to a [`crate::Subscriber`].
///
/// `id` is the outbox row's `public_id` (the ULID string, used as the
/// idempotency key on webhook delivery via `X-Koji-Event-Id`). `topic` selects
/// interested subscribers; `payload` is the raw JSON appended at `publish`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// The event's `public_id` (ULID string).
    pub id: String,
    /// The event topic (e.g. `area.route_updated`).
    pub topic: String,
    /// The event payload, as appended to the outbox.
    pub payload: serde_json::Value,
}
