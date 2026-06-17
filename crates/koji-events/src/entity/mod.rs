//! sea-orm entities for the koji-events tables (`event_outbox`,
//! `webhook_subscription`).
//!
//! These back the typed reads + the source-of-truth column/enum names; the hot
//! claim/backoff updates use raw `Statement` SQL (see `dispatcher.rs`).

pub mod event_outbox;
pub mod webhook_subscription;
