//! Error types for the event outbox + dispatcher (events design spec).
//!
//! - [`DeliverError`] is the *subscriber-side* error: returned by
//!   [`crate::Subscriber::deliver`]. Any subscriber failure makes the dispatcher
//!   back off + retry the whole event.
//! - [`PublishError`] wraps failures of [`crate::EventDispatcher::publish`]
//!   (append-to-outbox).
//! - [`DispatchError`] covers failures of the claim/backoff loop's DB
//!   bookkeeping.

use thiserror::Error;

/// Subscriber-side delivery failure. Returned by
/// [`crate::Subscriber::deliver`]; recorded into `event_outbox.last_error` and
/// triggers a backoff retry of the event.
#[derive(Debug, Error)]
pub enum DeliverError {
    /// The HTTP request itself failed (connect/timeout/DNS/TLS) — no status.
    #[error("http transport error: {0}")]
    Http(String),

    /// The receiver responded with a non-2xx status.
    #[error("non-success status {status}")]
    Status {
        /// The HTTP status code returned.
        status: u16,
    },

    /// Any other delivery failure (e.g. a DB read for subscriptions, a
    /// subscriber-specific error).
    #[error("delivery error: {0}")]
    Other(String),
}

/// Failure of [`crate::EventDispatcher::publish`].
#[derive(Debug, Error)]
pub enum PublishError {
    /// The payload could not be serialized to JSON.
    #[error("failed to serialize event payload: {0}")]
    Serialize(#[from] serde_json::Error),

    /// An underlying database error occurred while inserting the outbox row.
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}

/// Failure of the dispatcher's claim / backoff bookkeeping (the loop logs and
/// retries these; never fails an event by itself).
#[derive(Debug, Error)]
pub enum DispatchError {
    /// An underlying database error occurred while claiming / updating a row.
    #[error("database error: {0}")]
    Db(#[from] sea_orm::DbErr),
}
