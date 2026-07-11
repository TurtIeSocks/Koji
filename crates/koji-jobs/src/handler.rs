//! The handler abstraction (design spec §10).
//!
//! [`JobHandler`] is the *only* thing a concrete job type implements. It is
//! deliberately generic: this crate has no knowledge of clustering/routing — the
//! concrete `CalculateHandler` lives in `koji-service` (spec §3). `run` is
//! **synchronous** because the real work is CPU-bound (rayon); the worker calls
//! it inside `tokio::task::spawn_blocking` so it never stalls the async runtime
//! (spec §5).

use std::collections::HashMap;
use std::sync::Arc;

use crate::error::JobError;
use crate::types::JobCtx;

/// A unit of work the queue knows how to execute.
///
/// Implementors are registered in a [`HandlerRegistry`] keyed by [`JobHandler::kind`],
/// which must match the `job.kind` column of the rows they should handle.
///
/// `Send + Sync + 'static`: a handler is shared across worker tasks (`Arc<dyn
/// JobHandler>`) and moved into `spawn_blocking`.
pub trait JobHandler: Send + Sync + 'static {
    /// The stable kind string this handler claims (matches `job.kind`).
    fn kind(&self) -> &'static str;

    /// Run the job to completion. **Synchronous** — invoked on a blocking thread.
    ///
    /// Receives the deserialized `job.payload` and a [`JobCtx`] (cancellation +
    /// progress). Returns the result JSON on success or a [`JobError`] on
    /// failure; the worker persists either outcome.
    fn run(&self, payload: serde_json::Value, ctx: &JobCtx) -> Result<serde_json::Value, JobError>;
}

/// Maps a `job.kind` to the [`JobHandler`] that executes it.
///
/// Built once at startup and handed to [`crate::JobQueue::spawn_workers`]; shared
/// (cloned) across workers. Lookups are cheap (`HashMap` + `Arc` clone).
#[derive(Clone, Default)]
pub struct HandlerRegistry {
    handlers: HashMap<&'static str, Arc<dyn JobHandler>>,
}

impl HandlerRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        HandlerRegistry {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler under its [`JobHandler::kind`].
    ///
    /// Returns `self` for fluent chaining. A later registration with the same
    /// kind overwrites the earlier one.
    pub fn register<H: JobHandler>(mut self, handler: H) -> Self {
        self.handlers.insert(handler.kind(), Arc::new(handler));
        self
    }

    /// Look up the handler for a `job.kind`, if any.
    pub fn get(&self, kind: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(kind).cloned()
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("kinds", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::JobError;
    use serde_json::json;

    // ── Test handlers ─────────────────────────────────────────────────────

    /// Echoes the payload back as the result.
    struct EchoHandler;
    impl JobHandler for EchoHandler {
        fn kind(&self) -> &'static str {
            "echo"
        }
        fn run(
            &self,
            payload: serde_json::Value,
            _ctx: &JobCtx,
        ) -> Result<serde_json::Value, JobError> {
            Ok(payload)
        }
    }

    /// Always returns a validation error.
    struct FailingHandler;
    impl JobHandler for FailingHandler {
        fn kind(&self) -> &'static str {
            "fail"
        }
        fn run(
            &self,
            _payload: serde_json::Value,
            _ctx: &JobCtx,
        ) -> Result<serde_json::Value, JobError> {
            Err(JobError::validation("bad input"))
        }
    }

    /// Returns a fixed result (does not echo).
    struct CalcHandler;
    impl JobHandler for CalcHandler {
        fn kind(&self) -> &'static str {
            "calc.route"
        }
        fn run(
            &self,
            _payload: serde_json::Value,
            _ctx: &JobCtx,
        ) -> Result<serde_json::Value, JobError> {
            Ok(json!({"routes": 3}))
        }
    }

    // ── Test helpers ──────────────────────────────────────────────────────

    /// Build a minimal JobCtx backed by a real CancelToken.
    ///
    /// sea-orm's `DatabaseConnection::default()` is the `Disconnected` variant —
    /// perfectly constructible without a live pool. (`set()` is never called by
    /// these tests, so the disconnected handle is never used.)
    fn make_ctx() -> JobCtx {
        use crate::types::{CancelToken, ProgressHandle};

        let progress = ProgressHandle::new(
            sea_orm::DatabaseConnection::default(),
            0,
            "test".to_string(),
            None,
        );
        JobCtx {
            cancel: CancelToken::new(),
            progress,
        }
    }

    // ── HandlerRegistry::new / default ────────────────────────────────────

    #[test]
    fn new_registry_is_empty() {
        let reg = HandlerRegistry::new();
        assert!(reg.get("echo").is_none());
    }

    #[test]
    fn default_registry_is_empty() {
        let reg = HandlerRegistry::default();
        assert!(reg.get("anything").is_none());
    }

    // ── HandlerRegistry::register + get ────────────────────────────────────

    #[test]
    fn register_single_handler_then_get() {
        let ctx = make_ctx();
        let reg = HandlerRegistry::new().register(EchoHandler);
        let h = reg.get("echo").expect("handler must be present");
        let result = h.run(json!({"x": 1}), &ctx);
        assert_eq!(result.unwrap(), json!({"x": 1}));
    }

    #[test]
    fn get_unknown_kind_returns_none() {
        let reg = HandlerRegistry::new().register(EchoHandler);
        assert!(reg.get("unknown").is_none());
    }

    #[test]
    fn register_multiple_handlers_all_found() {
        let reg = HandlerRegistry::new()
            .register(EchoHandler)
            .register(FailingHandler)
            .register(CalcHandler);
        assert!(reg.get("echo").is_some());
        assert!(reg.get("fail").is_some());
        assert!(reg.get("calc.route").is_some());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn register_overwrites_prior_for_same_kind() {
        struct FirstHandler;
        impl JobHandler for FirstHandler {
            fn kind(&self) -> &'static str {
                "dupe"
            }
            fn run(
                &self,
                _p: serde_json::Value,
                _ctx: &JobCtx,
            ) -> Result<serde_json::Value, JobError> {
                Ok(json!("first"))
            }
        }
        struct SecondHandler;
        impl JobHandler for SecondHandler {
            fn kind(&self) -> &'static str {
                "dupe"
            }
            fn run(
                &self,
                _p: serde_json::Value,
                _ctx: &JobCtx,
            ) -> Result<serde_json::Value, JobError> {
                Ok(json!("second"))
            }
        }

        let ctx = make_ctx();
        let reg = HandlerRegistry::new()
            .register(FirstHandler)
            .register(SecondHandler);
        let h = reg.get("dupe").unwrap();
        // Second registration wins.
        assert_eq!(h.run(json!(null), &ctx).unwrap(), json!("second"));
    }

    // ── HandlerRegistry::get returns independent Arc clones ───────────────

    #[test]
    fn two_gets_return_independent_arc_clones() {
        let ctx = make_ctx();
        let reg = HandlerRegistry::new().register(EchoHandler);
        let h1 = reg.get("echo").unwrap();
        let h2 = reg.get("echo").unwrap();
        assert_eq!(h1.run(json!(1), &ctx).unwrap(), json!(1));
        assert_eq!(h2.run(json!(2), &ctx).unwrap(), json!(2));
    }

    // ── HandlerRegistry Debug ──────────────────────────────────────────────

    #[test]
    fn debug_output_includes_registered_kind() {
        let reg = HandlerRegistry::new().register(EchoHandler);
        let dbg = format!("{reg:?}");
        assert!(dbg.contains("HandlerRegistry"), "must name the type");
        assert!(dbg.contains("echo"), "must include registered kind");
    }

    #[test]
    fn debug_empty_registry_has_no_kind_names() {
        let reg = HandlerRegistry::new();
        let dbg = format!("{reg:?}");
        assert!(dbg.contains("HandlerRegistry"));
        assert!(!dbg.contains("echo"));
    }

    // ── Handler dispatch: Ok vs Err paths ─────────────────────────────────

    #[test]
    fn echo_handler_run_returns_payload() {
        let ctx = make_ctx();
        let h = EchoHandler;
        let payload = json!({"radius": 70, "min_points": 3});
        let result = h.run(payload.clone(), &ctx);
        assert_eq!(result.unwrap(), payload);
    }

    #[test]
    fn failing_handler_run_returns_validation_error() {
        let ctx = make_ctx();
        let h = FailingHandler;
        let err = h.run(json!({}), &ctx).unwrap_err();
        assert_eq!(err.code(), "validation_error");
    }

    #[test]
    fn calc_handler_run_returns_fixed_result() {
        let ctx = make_ctx();
        let h = CalcHandler;
        let result = h.run(json!({"anything": true}), &ctx);
        assert_eq!(result.unwrap(), json!({"routes": 3}));
    }

    // ── HandlerRegistry Clone shares handlers ─────────────────────────────

    #[test]
    fn cloned_registry_has_same_handlers() {
        let ctx = make_ctx();
        let reg = HandlerRegistry::new().register(EchoHandler);
        let reg2 = reg.clone();
        assert!(reg2.get("echo").is_some());
        let h = reg2.get("echo").unwrap();
        assert_eq!(h.run(json!("hello"), &ctx).unwrap(), json!("hello"));
    }

    #[test]
    fn clone_does_not_share_mutations() {
        // Registering into a clone does not affect the original.
        let reg = HandlerRegistry::new().register(EchoHandler);
        let reg2 = reg.clone().register(FailingHandler);
        // Original has only "echo".
        assert!(reg.get("echo").is_some());
        assert!(reg.get("fail").is_none());
        // Clone has both.
        assert!(reg2.get("echo").is_some());
        assert!(reg2.get("fail").is_some());
    }

    // ── kind() accessor on trait object ───────────────────────────────────

    #[test]
    fn handler_kind_accessor_returns_correct_string() {
        let h: &dyn JobHandler = &EchoHandler;
        assert_eq!(h.kind(), "echo");
        let h2: &dyn JobHandler = &FailingHandler;
        assert_eq!(h2.kind(), "fail");
        let h3: &dyn JobHandler = &CalcHandler;
        assert_eq!(h3.kind(), "calc.route");
    }
}
