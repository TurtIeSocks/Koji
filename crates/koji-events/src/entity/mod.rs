//! sea-orm entities for the koji-events tables.
//!
//! Only `webhook_subscription` is read through a typed entity
//! (`active_subscriptions` + koji-service). Every `event_outbox` read/write —
//! publish, claim, backoff, dead-letter — uses raw `Statement` SQL in
//! `dispatcher.rs`; its schema source of truth is the migration.

pub mod webhook_subscription;
