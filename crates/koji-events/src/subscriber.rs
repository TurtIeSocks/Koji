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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DeliverError;

    /// Minimal subscriber that accepts every event and records delivery count.
    struct AcceptAll {
        count: std::sync::atomic::AtomicUsize,
    }

    impl AcceptAll {
        fn new() -> Self {
            AcceptAll { count: std::sync::atomic::AtomicUsize::new(0) }
        }
        fn delivered(&self) -> usize {
            self.count.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl Subscriber for AcceptAll {
        fn name(&self) -> &'static str {
            "accept-all"
        }
        async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
            self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        }
    }

    /// Subscriber that rejects every event.
    struct AlwaysFail;

    #[async_trait]
    impl Subscriber for AlwaysFail {
        fn name(&self) -> &'static str {
            "always-fail"
        }
        async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
            Err(DeliverError::Other("forced failure".into()))
        }
    }

    /// Subscriber that only wants a specific topic, overriding `interested_in`.
    struct TopicFilter {
        want: &'static str,
    }

    #[async_trait]
    impl Subscriber for TopicFilter {
        fn name(&self) -> &'static str {
            "topic-filter"
        }
        async fn deliver(&self, _event: &Event) -> Result<(), DeliverError> {
            Ok(())
        }
        fn interested_in(&self, topic: &str) -> bool {
            topic == self.want
        }
    }

    fn make_event(topic: &str) -> Event {
        Event {
            id: "01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(),
            topic: topic.to_string(),
            payload: serde_json::json!({}),
        }
    }

    // ---- default interested_in -------------------------------------------

    #[test]
    fn default_interested_in_returns_true_for_any_topic() {
        let s = AcceptAll::new();
        assert!(s.interested_in("area.updated"));
        assert!(s.interested_in(""));
        assert!(s.interested_in("completely.random.topic"));
    }

    // ---- overridden interested_in ----------------------------------------

    #[test]
    fn overridden_interested_in_matches_exact_topic() {
        let s = TopicFilter { want: "area.route_updated" };
        assert!(s.interested_in("area.route_updated"));
        assert!(!s.interested_in("area.updated"));
        assert!(!s.interested_in(""));
    }

    // ---- deliver Ok / Err -----------------------------------------------

    #[tokio::test]
    async fn accept_all_deliver_returns_ok() {
        let s = AcceptAll::new();
        let ev = make_event("area.updated");
        assert!(s.deliver(&ev).await.is_ok());
        assert_eq!(s.delivered(), 1);
    }

    #[tokio::test]
    async fn accept_all_deliver_accumulates_count() {
        let s = AcceptAll::new();
        let ev = make_event("t");
        s.deliver(&ev).await.unwrap();
        s.deliver(&ev).await.unwrap();
        s.deliver(&ev).await.unwrap();
        assert_eq!(s.delivered(), 3);
    }

    #[tokio::test]
    async fn always_fail_deliver_returns_err() {
        let s = AlwaysFail;
        let ev = make_event("t");
        let err = s.deliver(&ev).await.unwrap_err();
        assert!(matches!(err, DeliverError::Other(_)));
    }

    // ---- name ---------------------------------------------------------------

    #[test]
    fn subscriber_name_is_static_str() {
        let s = AcceptAll::new();
        assert_eq!(s.name(), "accept-all");
        let f = AlwaysFail;
        assert_eq!(f.name(), "always-fail");
    }

    // ---- Arc<dyn Subscriber> dispatch (Send + Sync + 'static) -------------

    #[tokio::test]
    async fn subscriber_usable_via_arc_dyn() {
        let s: std::sync::Arc<dyn Subscriber> = std::sync::Arc::new(AcceptAll::new());
        let ev = make_event("x");
        assert!(s.deliver(&ev).await.is_ok());
        assert!(s.interested_in("x"));
    }
}
