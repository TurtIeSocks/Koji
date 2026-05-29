//! # koji-jobs
//!
//! The **generic, persistent job-queue runtime** for Koji V2 (design:
//! `docs/superpowers/specs/2026-05-28-koji-job-queue-design.md`).
//!
//! This crate is intentionally domain-agnostic: it owns the `job` table entity,
//! multi-process-safe claim/lease/heartbeat, the worker pool, the [`JobHandler`]
//! trait, the [`JobQueue`] producer/consumer API, coarse [`ProgressHandle`] /
//! [`CancelToken`], and the in-process waiter registry that bridges the
//! synchronous request path. It has **no** knowledge of clustering/routing — the
//! concrete `KojiJob` payload and `CalculateHandler` live in `koji-service`
//! (spec §3).
//!
//! ## Shape
//! - [`JobQueue`] — `enqueue_or_attach` (dedup + grace, §6), `await_result`
//!   (sync bridge, §7), `get`, `cancel`, `claim` (§5), `notify_local_waiters`.
//! - [`JobHandler`] + [`HandlerRegistry`] — pluggable execution (§10).
//! - [`JobCtx`] = `{ cancel, progress }` handed to a handler's `run`.
//! - [`spawn_workers`](JobQueue::spawn_workers) → [`WorkerSet`] — the worker pool (§5).
//!
//! ## What is NOT verified here
//! There is no database in this crate's tests. The claim/lease/heartbeat,
//! `enqueue_or_attach` coalescing, the `await_result` `select!`, and the worker
//! loop are exercised only against a live MySQL/MariaDB in `koji-service`
//! integration tests. The unit tests here cover only DB-free logic
//! ([`dedup_key`] stability and [`JobId`] round-trip).

pub mod entity;
pub mod error;
pub mod handler;
pub mod queue;
pub mod types;
pub mod worker;

// ---- Public surface (spec §10) -------------------------------------------

pub use entity::JobStatus;
pub use error::{AwaitError, EnqueueError, JobError};
pub use handler::{HandlerRegistry, JobHandler};
pub use queue::{ClaimedJob, JobQueue, dedup_key};
pub use types::{CancelToken, JobCtx, JobId, JobOutcome, JobRecord, ProgressHandle};
pub use worker::WorkerSet;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn job_id_ulid_round_trips_through_string() {
        let id = JobId::new();
        let s = id.to_string();
        // Canonical ULID encoding is always 26 chars (Crockford base32).
        assert_eq!(s.len(), 26, "ULID string must be 26 chars, got {s:?}");

        let parsed = JobId::from_str(&s).expect("round-trip parse should succeed");
        assert_eq!(parsed, id, "FromStr(Display(id)) must equal id");
        assert_eq!(parsed.as_string(), s);
    }

    #[test]
    fn job_id_rejects_malformed_strings() {
        assert!(JobId::from_str("not-a-ulid").is_err());
        assert!(JobId::from_str("").is_err());
    }

    #[test]
    fn dedup_key_is_stable_for_identical_input() {
        let fields = json!({ "radius": 70, "min_points": 3, "area": "geofence:42" });
        let a = dedup_key("calc.route", &fields);
        let b = dedup_key("calc.route", &fields);
        assert_eq!(a, b, "same kind + same fields must hash identically");
        // sha256 hex is 64 chars → fits `dedup_key CHAR(64)`.
        assert_eq!(a.len(), 64, "dedup_key must be 64 hex chars");
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn dedup_key_is_field_order_independent() {
        // Same logical config, keys written in two different orders.
        let order_a = json!({ "radius": 70, "min_points": 3, "cluster_mode": "balanced" });
        let order_b = json!({ "cluster_mode": "balanced", "min_points": 3, "radius": 70 });
        assert_eq!(
            dedup_key("calc.route", &order_a),
            dedup_key("calc.route", &order_b),
            "field ordering must not affect the dedup key"
        );
    }

    #[test]
    fn dedup_key_is_field_order_independent_when_nested() {
        // Nested objects must also be normalized recursively.
        let a = json!({
            "config": { "radius": 70, "min_points": 3 },
            "filter": { "last_seen": 0, "data_type": "pokestop" }
        });
        let b = json!({
            "filter": { "data_type": "pokestop", "last_seen": 0 },
            "config": { "min_points": 3, "radius": 70 }
        });
        assert_eq!(dedup_key("calc.route", &a), dedup_key("calc.route", &b));
    }

    #[test]
    fn dedup_key_differs_on_value_change() {
        let a = json!({ "radius": 70 });
        let b = json!({ "radius": 80 });
        assert_ne!(
            dedup_key("calc.route", &a),
            dedup_key("calc.route", &b),
            "different values must produce different keys"
        );
    }

    #[test]
    fn dedup_key_differs_on_kind_change() {
        let fields = json!({ "radius": 70 });
        assert_ne!(
            dedup_key("calc.route", &fields),
            dedup_key("calc.bootstrap", &fields),
            "kind must be part of the hashed identity"
        );
    }

    #[test]
    fn dedup_key_array_order_is_significant() {
        // Arrays are order-significant (a route's point order matters), so a
        // reordered array must NOT collide.
        let a = json!({ "points": [1, 2, 3] });
        let b = json!({ "points": [3, 2, 1] });
        assert_ne!(
            dedup_key("calc.route", &a),
            dedup_key("calc.route", &b),
            "array element order must affect the key"
        );
    }

    #[test]
    fn job_status_terminal_classification() {
        assert!(JobStatus::Succeeded.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Canceled.is_terminal());
        assert!(!JobStatus::Queued.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
    }

    #[test]
    fn job_error_codes_are_stable() {
        assert_eq!(JobError::validation("x").code(), "validation_error");
        assert_eq!(JobError::internal("x").code(), "internal_error");
        assert_eq!(JobError::custom("my_code", "msg").code(), "my_code");
    }

    #[test]
    fn cancel_token_flips() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
        // Cloned handles observe the same flag.
        let clone = token.clone();
        assert!(clone.is_cancelled());
    }
}
