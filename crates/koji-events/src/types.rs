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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    // ---- EventId ------------------------------------------------------------

    #[test]
    fn event_id_new_produces_26_char_ulid() {
        let id = EventId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 26, "ULID must be 26 Crockford base32 chars");
    }

    #[test]
    fn event_id_display_and_as_string_agree() {
        let id = EventId::new();
        assert_eq!(id.to_string(), id.as_string());
    }

    #[test]
    fn event_id_from_str_round_trips() {
        let id = EventId::new();
        let s = id.to_string();
        let parsed = EventId::from_str(&s).expect("valid ULID should parse");
        assert_eq!(parsed, id);
    }

    #[test]
    fn event_id_from_str_rejects_empty() {
        assert!(EventId::from_str("").is_err());
    }

    #[test]
    fn event_id_from_str_rejects_random_string() {
        assert!(EventId::from_str("not-a-ulid").is_err());
    }

    #[test]
    fn event_id_from_str_rejects_wrong_length() {
        // 25 chars — one short
        assert!(EventId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FA").is_err());
    }

    #[test]
    fn event_id_two_new_calls_differ() {
        // Not a strict time guarantee — just checks the generator isn't
        // constant. In practice ULIDs differ because of the random 80-bit
        // suffix even when called in the same millisecond.
        let a = EventId::new();
        let b = EventId::new();
        // Two ULIDs generated back-to-back may theoretically be equal if the
        // monotonic counter overflows, but that requires 2^80 calls per ms —
        // safe to assert inequality here.
        assert_ne!(a, b);
    }

    #[test]
    fn event_id_serde_round_trip() {
        let id = EventId::new();
        let json_str = serde_json::to_string(&id).expect("serialize");
        // Wire form: a JSON string (quoted 26-char ULID).
        assert!(json_str.starts_with('"'));
        assert!(json_str.ends_with('"'));
        let inner = &json_str[1..json_str.len() - 1];
        assert_eq!(inner.len(), 26);

        let back: EventId = serde_json::from_str(&json_str).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn event_id_serde_rejects_invalid_ulid_string() {
        let bad = r#""not-a-ulid""#;
        let result: Result<EventId, _> = serde_json::from_str(bad);
        assert!(result.is_err(), "should reject non-ULID JSON string");
    }

    #[test]
    fn event_id_default_is_non_nil() {
        // Default calls new() — should produce a valid, non-nil ULID.
        let id = EventId::default();
        assert_ne!(id.as_string(), "00000000000000000000000000");
    }

    #[test]
    fn event_id_copy_semantics() {
        let a = EventId::new();
        let b = a; // copy, not move
        assert_eq!(a, b);
    }

    // ---- Event --------------------------------------------------------------

    #[test]
    fn event_serde_round_trip_with_object_payload() {
        let ev = Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "area.route_updated".to_string(),
            payload: json!({"area_id": 42, "name": "test"}),
        };
        let s = serde_json::to_string(&ev).expect("serialize");
        let back: Event = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, ev);
    }

    #[test]
    fn event_serde_round_trip_with_null_payload() {
        let ev = Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "noop".to_string(),
            payload: serde_json::Value::Null,
        };
        let s = serde_json::to_string(&ev).expect("serialize");
        let back: Event = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back.payload, serde_json::Value::Null);
    }

    #[test]
    fn event_serde_round_trip_with_array_payload() {
        let ev = Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "bulk".to_string(),
            payload: json!([1, 2, 3]),
        };
        let s = serde_json::to_string(&ev).expect("serialize");
        let back: Event = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(back, ev);
    }

    #[test]
    fn event_clone_is_equal() {
        let ev = Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!({}),
        };
        assert_eq!(ev.clone(), ev);
    }

    #[test]
    fn event_equality_is_field_wise() {
        let base = Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: "t".to_string(),
            payload: json!({"x": 1}),
        };
        let diff_id = Event { id: "01ARZ3NDEKTSV4RRFFQ69G5FAW".to_string(), ..base.clone() };
        let diff_topic = Event { topic: "other".to_string(), ..base.clone() };
        let diff_payload = Event { payload: json!({"x": 2}), ..base.clone() };
        assert_ne!(base, diff_id);
        assert_ne!(base, diff_topic);
        assert_ne!(base, diff_payload);
    }

    #[test]
    fn event_deserialize_missing_field_fails() {
        // `payload` field is required.
        let bad = r#"{"id":"01ARZ3NDEKTSV4RRFFQ69G5FAV","topic":"t"}"#;
        let result: Result<Event, _> = serde_json::from_str(bad);
        assert!(result.is_err(), "missing payload must fail deserialization");
    }
}
