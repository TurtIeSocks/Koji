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

    /// Register an already-`Arc`'d handler (useful when a handler is shared
    /// elsewhere).
    pub fn register_arc(mut self, handler: Arc<dyn JobHandler>) -> Self {
        self.handlers.insert(handler.kind(), handler);
        self
    }

    /// Look up the handler for a `job.kind`, if any.
    pub fn get(&self, kind: &str) -> Option<Arc<dyn JobHandler>> {
        self.handlers.get(kind).cloned()
    }

    /// Whether a handler is registered for `kind`.
    pub fn contains(&self, kind: &str) -> bool {
        self.handlers.contains_key(kind)
    }
}

impl std::fmt::Debug for HandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HandlerRegistry")
            .field("kinds", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}
