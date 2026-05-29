//! The [`Subscriber`] trait: a pluggable delivery sink for domain events
//! (events design spec, "Crate surface").
//!
//! The dispatcher delivers each claimed event to every subscriber whose
//! [`Subscriber::interested_in`] returns `true` for the event topic. Delivery is
//! all-or-nothing per event: if any interested subscriber returns
//! [`DeliverError`], the dispatcher backs the event off and retries the whole
//! set (receiver-side idempotency via `X-Koji-Event-Id` makes the re-delivery to
//! already-succeeded subscribers safe).

use async_trait::async_trait;

use crate::error::DeliverError;
use crate::types::Event;

/// A delivery sink for domain events.
///
/// Implementors are shared as `Arc<dyn Subscriber>` across the dispatcher loop,
/// hence the `Send + Sync + 'static` bound.
#[async_trait]
pub trait Subscriber: Send + Sync + 'static {
    /// A short, stable name for logging (e.g. `"webhook"`).
    fn name(&self) -> &'static str;

    /// Deliver `event`. `Ok(())` means accepted; any `Err` makes the dispatcher
    /// retry the whole event with backoff.
    async fn deliver(&self, event: &Event) -> Result<(), DeliverError>;

    /// Whether this subscriber wants `topic`. Defaults to all topics; a
    /// subscriber that fans out to a per-topic registry (e.g.
    /// [`crate::WebhookSubscriber`]) typically also defaults to `true` and
    /// filters internally.
    fn interested_in(&self, _topic: &str) -> bool {
        true
    }
}
