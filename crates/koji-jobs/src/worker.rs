//! The worker pool (design spec §5, §8).
//!
//! [`JobQueue::spawn_workers`] launches `n` worker tasks against one queue and
//! returns a [`WorkerSet`] that owns their `JoinHandle`s and a shutdown signal.
//!
//! Each worker runs the loop from spec §5:
//! ```text
//! loop {
//!   job = claim()                  // else wait on Notify or poll, continue
//!   spawn heartbeat(job)           // renew lease every 20s
//!   outcome = spawn_blocking(|| handler.run(payload, &ctx))  // CPU/rayon off the runtime
//!   persist(succeeded|failed, result|error, finished_at = NOW())
//!   stop heartbeat
//!   notify_local_waiters(public_id, outcome)
//! }
//! ```
//!
//! Default concurrency is 1 (spec §8): a single CPU-bound worker means one rayon
//! job at a time uses all cores without oversubscription. More workers exist for
//! multi-process scale-out, which is safe via `SKIP LOCKED` in [`JobQueue::claim`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sea_orm::{ConnectionTrait, DbBackend, Statement, Value};
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::handler::HandlerRegistry;
use crate::queue::{ClaimedJob, JobQueue, LEASE_SECS};
use crate::types::{CancelToken, JobCtx, JobOutcome, ProgressHandle};

/// Heartbeat cadence: renew the lease every 20s against the 60s lease (spec §9).
const HEARTBEAT_SECS: u64 = 20;

/// How long a worker waits for a wake (`Notify`) before falling back to a poll
/// when the queue came up empty. Bounds the latency for jobs enqueued by another
/// process (whose enqueue can't poke our local `Notify`).
const IDLE_POLL: Duration = Duration::from_millis(500);

/// Backoff after a transient DB/claim error in the worker loop, so a flapping DB
/// doesn't spin the loop hot (spec §11: claim errors are retried, never fail a
/// job by themselves).
const ERROR_BACKOFF: Duration = Duration::from_secs(1);

/// Graceful-shutdown signal shared with every worker.
///
/// Two parts because a worker needs both to *cheaply poll* "has shutdown been
/// requested?" between claims (the [`AtomicBool`]) and to be *woken* if it is
/// parked waiting for work (the [`Notify`]). A lone `Notify` can only signal a
/// future notify (no level state), so an `AtomicBool` carries the durable flag.
#[derive(Default)]
struct Shutdown {
    flag: AtomicBool,
    wake: Notify,
}

impl Shutdown {
    /// Mark shutdown requested and wake every parked worker.
    fn trigger(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    /// Cheap, non-blocking check of the durable flag.
    fn is_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Owns the spawned worker tasks + a shutdown signal.
///
/// Drop does **not** auto-stop the workers (their `JoinHandle`s would just
/// detach); call [`WorkerSet::shutdown`] for a graceful stop.
pub struct WorkerSet {
    handles: Vec<JoinHandle<()>>,
    shutdown: Arc<Shutdown>,
}

impl WorkerSet {
    /// Signal all workers to stop after their current job and wait for them to
    /// finish.
    ///
    /// Workers check the shutdown signal at the top of each loop iteration and
    /// between claim waits, so an idle pool stops promptly and a busy worker
    /// stops once its in-flight job completes (rayon work is not interruptible —
    /// spec §2).
    pub async fn shutdown(self) {
        // Set the flag and wake every worker parked on the shutdown/notify select.
        self.shutdown.trigger();
        for handle in self.handles {
            // A panicked worker surfaces here; we log and continue shutting down.
            if let Err(e) = handle.await {
                log::error!("[koji-jobs] worker task join error during shutdown: {e}");
            }
        }
    }

    /// Number of worker tasks in this set.
    pub fn len(&self) -> usize {
        self.handles.len()
    }

    /// Whether the set has no workers.
    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }
}

impl JobQueue {
    /// Spawn `n` worker tasks consuming this queue with `registry`, returning a
    /// [`WorkerSet`] (spec §10).
    ///
    /// `n` is clamped to at least 1. All workers share the same `Arc<JobQueue>`
    /// and a cloned [`HandlerRegistry`]; they coordinate purely through the DB
    /// (claim) and the in-process `Notify`.
    pub fn spawn_workers(self: Arc<Self>, n: usize, registry: HandlerRegistry) -> WorkerSet {
        let n = n.max(1);
        let shutdown = Arc::new(Shutdown::default());
        let mut handles = Vec::with_capacity(n);

        for worker_idx in 0..n {
            let queue = Arc::clone(&self);
            let registry = registry.clone();
            let shutdown = Arc::clone(&shutdown);
            handles.push(tokio::spawn(async move {
                worker_loop(worker_idx, queue, registry, shutdown).await;
            }));
        }

        WorkerSet { handles, shutdown }
    }
}

/// The per-worker loop (spec §5). Runs until `shutdown` is signaled.
async fn worker_loop(
    worker_idx: usize,
    queue: Arc<JobQueue>,
    registry: HandlerRegistry,
    shutdown: Arc<Shutdown>,
) {
    log::info!(
        "[koji-jobs] worker {worker_idx} started (id={})",
        queue.worker_id
    );

    loop {
        // Stop promptly if asked, before attempting another claim. Covers the
        // case where shutdown fired while we were running a job.
        if shutdown.is_requested() {
            break;
        }

        // Hold the pause gate as a reader across this claim+run. The synchronous
        // (queue-bypass) calc path takes the write guard, which — because the
        // RwLock is write-preferring — blocks this acquire until any in-flight
        // job finishes, so the pool is paused while the inline compute runs and
        // the two rayon workloads never oversubscribe cores. Released before
        // parking (below) so an idle worker never holds the gate against a
        // waiting sync writer.
        let pause_guard = Arc::clone(&queue.pause_gate).read_owned().await;

        // Try to claim a job. Claim errors are transient — back off and retry,
        // never fail a job because of them (spec §11).
        let claimed = match queue.claim().await {
            Ok(Some(job)) => job,
            Ok(None) => {
                // Empty queue: drop the gate so a sync writer can proceed, then
                // wait for a local enqueue poke or a poll tick, or a shutdown
                // signal — whichever comes first.
                drop(pause_guard);
                tokio::select! {
                    _ = queue.notify.notified() => {}
                    _ = tokio::time::sleep(IDLE_POLL) => {}
                    _ = shutdown.wake.notified() => break,
                }
                continue;
            }
            Err(e) => {
                drop(pause_guard);
                log::error!("[koji-jobs] worker {worker_idx} claim error: {e}; backing off");
                tokio::time::sleep(ERROR_BACKOFF).await;
                continue;
            }
        };

        run_claimed_job(worker_idx, &queue, &registry, claimed).await;
        // Explicit drop: release the pause gate before the next iteration's
        // acquire so a waiting sync writer isn't starved by a busy worker.
        drop(pause_guard);
    }

    log::info!("[koji-jobs] worker {worker_idx} stopped");
}

/// Execute a single claimed job end-to-end: heartbeat → run → persist → notify.
async fn run_claimed_job(
    worker_idx: usize,
    queue: &Arc<JobQueue>,
    registry: &HandlerRegistry,
    mut claimed: ClaimedJob,
) {
    let public_id = claimed.public_id.as_string();
    log::debug!(
        "[koji-jobs] worker {worker_idx} claimed job id={} public_id={} kind={} attempt={}",
        claimed.id,
        public_id,
        claimed.kind,
        claimed.attempts
    );

    // Emit "running" status event now that the job is claimed.
    if let Some(s) = &queue.event_sink {
        s.on_job_status(&public_id, "running", 0.0, None);
    }

    // No handler for this kind: fail the job rather than spin (a misconfigured
    // registry shouldn't wedge the queue).
    let Some(handler) = registry.get(&claimed.kind) else {
        log::error!(
            "[koji-jobs] worker {worker_idx}: no handler for kind '{}' (job {})",
            claimed.kind,
            public_id
        );
        let outcome = JobOutcome::Failed {
            error: format!("no handler registered for kind '{}'", claimed.kind),
            code: "internal_error".to_owned(),
        };
        persist_outcome(queue, claimed.id, &outcome).await;
        // Emit terminal status before waking waiters.
        if let Some(s) = &queue.event_sink {
            s.on_job_status(&public_id, "failed", 0.0, None);
        }
        queue.notify_local_waiters(&public_id, outcome);
        return;
    };

    // Per-job cancellation flag + progress sink. The token is registered on the
    // queue so `cancel()` can flip it for same-process signals; the heartbeat
    // additionally polls `phase='canceling'` for cross-process cancels.
    let cancel = CancelToken::new();
    queue
        .running_tokens
        .insert(public_id.clone(), cancel.clone());
    let progress = ProgressHandle::new(
        queue.db.clone(),
        claimed.id,
        public_id.clone(),
        queue.event_sink.clone(),
    );
    let ctx = JobCtx::new(cancel.clone(), progress);

    // Start the heartbeat: renew the lease every 20s so another worker doesn't
    // reclaim this job mid-run (spec §5). Stopped via its own Notify once the
    // handler returns.
    let stop_heartbeat = Arc::new(Notify::new());
    let heartbeat = spawn_heartbeat(
        Arc::clone(queue),
        claimed.id,
        Arc::clone(&stop_heartbeat),
        cancel.clone(),
    );

    // Run the (synchronous, CPU-bound) handler off the async runtime so it never
    // blocks other tasks / the heartbeat (spec §5).
    // Move the payload out (calc payloads can embed large data_points JSON);
    // only claimed.id is read after this point.
    let payload = std::mem::take(&mut claimed.payload);
    let handler = Arc::clone(&handler);
    let run_result = tokio::task::spawn_blocking(move || handler.run(payload, &ctx)).await;

    // Handler finished (or its blocking task panicked) → stop the heartbeat.
    // `notify_one` (NOT `notify_waiters`): a fast handler can return before the
    // freshly-spawned heartbeat task has been polled to its `stop.notified()`
    // await point. `notify_waiters` only wakes *currently-registered* waiters, so
    // that signal would be lost and `heartbeat.await` below would hang forever
    // (wedging the worker + leaving the job stuck `running`). `notify_one` stores
    // a permit when no waiter is registered yet, so the heartbeat's next
    // `notified()` completes immediately.
    stop_heartbeat.notify_one();
    if let Err(e) = heartbeat.await {
        log::warn!("[koji-jobs] worker {worker_idx} heartbeat join error: {e}");
    }
    queue.running_tokens.remove(&public_id);

    // Map the run result to an outcome. A panic in the blocking task becomes an
    // internal failure (the job is not left dangling in `running`).
    let outcome = match run_result {
        Ok(Ok(result)) => JobOutcome::Succeeded(result),
        // A handler that bailed at a cancel check reports the dedicated
        // "canceled" code — a canceled run must persist as canceled, not be
        // overwritten to failed.
        Ok(Err(job_err)) if job_err.code() == "canceled" => JobOutcome::Canceled,
        Ok(Err(job_err)) => JobOutcome::Failed {
            error: job_err.to_string(),
            code: job_err.code().to_owned(),
        },
        Err(join_err) => {
            log::error!(
                "[koji-jobs] worker {worker_idx}: handler panicked for job {public_id}: {join_err}"
            );
            JobOutcome::Failed {
                error: "handler panicked".to_owned(),
                code: "internal_error".to_owned(),
            }
        }
    };

    // Persist terminal state + result/error + finished_at, then wake waiters.
    persist_outcome(queue, claimed.id, &outcome).await;

    // Emit terminal status event (best-effort, fire-and-forget).
    if let Some(s) = &queue.event_sink {
        let (status, progress) = match &outcome {
            JobOutcome::Succeeded(_) => ("succeeded", 1.0_f32),
            JobOutcome::Failed { .. } => ("failed", 0.0_f32),
            JobOutcome::Canceled => ("canceled", 0.0_f32),
        };
        s.on_job_status(&public_id, status, progress, None);
    }

    queue.notify_local_waiters(&public_id, outcome);
}

/// Spawn the heartbeat task for a running job: every [`HEARTBEAT_SECS`], extend
/// `lease_expires` to `NOW() + LEASE_SECS`. Exits when `stop` is signaled.
fn spawn_heartbeat(
    queue: Arc<JobQueue>,
    job_id: u64,
    stop: Arc<Notify>,
    cancel: CancelToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_SECS));
        // Skip the immediate first tick: the claim already set a fresh lease.
        interval.tick().await;
        loop {
            tokio::select! {
                _ = stop.notified() => break,
                _ = interval.tick() => {
                    let stmt = Statement::from_sql_and_values(
                        DbBackend::MySql,
                        "UPDATE `job` SET `lease_expires` = NOW() + INTERVAL ? SECOND \
                         WHERE `id` = ? AND `status` = 'running'",
                        [Value::from(LEASE_SECS), Value::from(job_id)],
                    );
                    if let Err(e) = queue.db.execute(stmt).await {
                        // A failed renew isn't fatal on its own; if it keeps
                        // failing the lease lapses and another worker reclaims.
                        log::warn!("[koji-jobs] heartbeat renew failed for job id={job_id}: {e}");
                    }
                    // Cross-process cancel pickup: a `cancel()` in another process
                    // can only write `phase='canceling'` — relay it into this
                    // job's in-process token on each renew.
                    if !cancel.is_cancelled() {
                        let phase_q = Statement::from_sql_and_values(
                            DbBackend::MySql,
                            "SELECT `phase` FROM `job` WHERE `id` = ?",
                            [Value::from(job_id)],
                        );
                        match queue.db.query_one(phase_q).await {
                            Ok(Some(row)) => {
                                if row.try_get::<Option<String>>("", "phase").ok().flatten().as_deref()
                                    == Some("canceling")
                                {
                                    cancel.cancel();
                                }
                            }
                            Ok(None) => {}
                            Err(e) => log::warn!(
                                "[koji-jobs] heartbeat phase check failed for job id={job_id}: {e}"
                            ),
                        }
                    }
                }
            }
        }
    })
}

/// Write the terminal state for a job: `succeeded` (+ result) / `failed`
/// (+ error) / `canceled`, with `finished_at = NOW()`.
///
/// Best-effort: a persist failure is logged. (The lease will lapse and the job
/// is eligible for reclaim, which — for `max_attempts=1` calc jobs — fails it,
/// consistent with the crash-recovery contract in spec §5.)
async fn persist_outcome(queue: &JobQueue, job_id: u64, outcome: &JobOutcome) {
    let stmt = match outcome {
        JobOutcome::Succeeded(result) => Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `job` SET `status` = 'succeeded', `result` = ?, `progress` = 1.0, \
               `finished_at` = NOW() WHERE `id` = ?",
            [Value::from(result.clone()), Value::from(job_id)],
        ),
        JobOutcome::Failed { error, .. } => Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `job` SET `status` = 'failed', `error` = ?, `finished_at` = NOW() \
             WHERE `id` = ?",
            [Value::from(error.clone()), Value::from(job_id)],
        ),
        JobOutcome::Canceled => Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `job` SET `status` = 'canceled', `finished_at` = NOW() WHERE `id` = ?",
            [Value::from(job_id)],
        ),
    };
    if let Err(e) = queue.db.execute(stmt).await {
        log::error!("[koji-jobs] failed to persist outcome for job id={job_id}: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression guard for the heartbeat-stop race fixed in e7fb172.
    ///
    /// In [`run_claimed_job`] the heartbeat is stopped with `stop.notify_one()`
    /// (NOT `notify_waiters()`). A near-instant handler can return and call the
    /// stop *before* the freshly-`tokio::spawn`ed heartbeat task has been polled
    /// to its `stop.notified()` await point. The fix relies on `Notify`'s permit
    /// semantics: `notify_one` on a `Notify` with no registered waiter stores a
    /// single permit, so the *next* `notified()` completes immediately.
    /// `notify_waiters` stores nothing — it only wakes waiters already parked —
    /// so the late waiter would miss the signal and `heartbeat.await` would hang
    /// forever, wedging the worker and leaving the job stuck `running`.
    ///
    /// This pins that semantic directly (no DB — koji-jobs unit tests are
    /// DB-free; see the crate docs). We notify BEFORE the waiter registers, then
    /// assert the waiter still wakes. The wait is wrapped in a short
    /// `tokio::time::timeout` so a regression to `notify_waiters` fails fast
    /// instead of hanging the suite.
    #[tokio::test]
    async fn notify_one_wakes_a_waiter_that_registers_after_the_signal() {
        let stop = Arc::new(Notify::new());

        // Signal the stop BEFORE anything is awaiting it — this is the race: the
        // handler finished before the heartbeat task reached `stop.notified()`.
        // `notify_one` parks a permit for the next waiter.
        stop.notify_one();

        // Now spawn the "heartbeat" task, which registers its `notified()` only
        // after the signal already fired. The stored permit must wake it.
        let stop_in_task = Arc::clone(&stop);
        let heartbeat = tokio::spawn(async move {
            stop_in_task.notified().await;
        });

        // With `notify_one` this resolves immediately; with `notify_waiters` the
        // permit is never stored and this times out (the regression).
        let joined = tokio::time::timeout(Duration::from_secs(5), heartbeat).await;

        assert!(
            joined.is_ok(),
            "heartbeat waiter did not wake: notify_one's stored permit was lost \
             (a regression to notify_waiters would hang here)"
        );
        joined
            .expect("waiter must wake within the timeout")
            .expect("heartbeat task must not panic");
    }

    /// Sibling sanity check documenting *why* the fix was needed: a waiter that
    /// registers after `notify_waiters()` (the pre-fix call) is NOT woken,
    /// because `notify_waiters` wakes only already-registered waiters and stores
    /// no permit. We assert the missed wake by observing that a bounded wait
    /// times out. This is the failure mode the production `notify_one` avoids.
    #[tokio::test]
    async fn notify_waiters_does_not_wake_a_late_waiter_documenting_the_bug() {
        let stop = Arc::new(Notify::new());

        // Pre-fix behavior: wake waiters, but none are registered yet.
        stop.notify_waiters();

        let stop_in_task = Arc::clone(&stop);
        let heartbeat = tokio::spawn(async move {
            stop_in_task.notified().await;
        });

        // The late waiter never sees the signal, so a bounded wait must elapse.
        // Kept short — this is the deliberately-hanging path.
        let joined = tokio::time::timeout(Duration::from_millis(200), heartbeat).await;

        assert!(
            joined.is_err(),
            "notify_waiters unexpectedly woke a late waiter; if Notify gains \
             permit semantics for notify_waiters, the production stop signal \
             could switch back to it — but until then notify_one is required"
        );
    }
}
