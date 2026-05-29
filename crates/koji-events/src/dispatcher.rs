//! The outbox producer + dispatcher runtime (events design spec, "Dispatcher
//! semantics").
//!
//! [`EventDispatcher::publish`] appends a `pending` row to `event_outbox` —
//! callable with no loop running (a pure outbox append, so producers emit even
//! when delivery is async/later).
//!
//! [`EventDispatcher::spawn`] runs the claim→deliver→backoff loop, mirroring the
//! koji-jobs worker (`crates/koji-jobs/src/worker.rs`):
//!
//! ```text
//! loop {
//!   row = claim_due()              // else wait / poll, continue
//!   spawn heartbeat(row)           // renew lease every 20s
//!   ok = deliver to every interested subscriber
//!   stop heartbeat
//!   if ok  -> status='delivered', delivered_at=NOW()
//!   else   -> attempts+1, last_error, next_attempt_at=NOW()+backoff,
//!             status='pending' (or 'dead' if attempts>=max_attempts)
//! }
//! ```
//!
//! Claiming is multi-process-safe via `FOR UPDATE SKIP LOCKED` in a transaction,
//! exactly like the job queue. The hot claim/backoff SQL is hand-written
//! (`Statement`) because it needs MySQL-specific clauses the query builder makes
//! awkward.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Value,
};
use serde::Serialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

use crate::error::PublishError;
use crate::subscriber::Subscriber;
use crate::types::{Event, EventId};

/// Lease duration applied on claim/renew: `lease_expires = NOW() + LEASE_SECS`
/// (spec: 60s lease, reusing the koji-jobs cadence).
const LEASE_SECS: i64 = 60;

/// Heartbeat cadence: renew the lease every 20s against the 60s lease.
const HEARTBEAT_SECS: u64 = 20;

/// Backoff cap: the exponential schedule `2^attempts` seconds never exceeds this
/// (spec: capped at 3600s = 1h).
const BACKOFF_CAP_SECS: i64 = 3600;

/// How long the loop waits before re-polling when no row is due. Bounds the
/// latency for rows that become due (their `next_attempt_at` passing) or are
/// published by another process (whose append can't poke our local `Notify`).
const IDLE_POLL: Duration = Duration::from_millis(500);

/// Backoff after a transient DB/claim error so a flapping DB doesn't spin the
/// loop hot.
const ERROR_BACKOFF: Duration = Duration::from_secs(1);

/// Compute the backoff delay (seconds) for an event that has just recorded its
/// `attempts`-th failure: `min(2^attempts, BACKOFF_CAP_SECS)`.
///
/// `attempts` is the post-increment value (≥ 1). Saturates so a large
/// `attempts` can never overflow the shift — it just clamps at the cap.
fn backoff_secs(attempts: i32) -> i64 {
    if attempts <= 0 {
        return 1;
    }
    // 2^attempts via shift, guarding the shift width (anything ≥ 63 overflows
    // i64 and is well past the cap anyway).
    let delay = if attempts >= 63 {
        BACKOFF_CAP_SECS
    } else {
        1i64.checked_shl(attempts as u32)
            .unwrap_or(BACKOFF_CAP_SECS)
    };
    delay.min(BACKOFF_CAP_SECS)
}

/// A row claimed by the dispatcher loop: the fields needed to deliver + persist.
#[derive(Debug, Clone)]
struct ClaimedEvent {
    /// Numeric PK (`event_outbox.id`) — used for targeted UPDATEs + heartbeat.
    id: u64,
    /// The [`Event`] handed to subscribers (public_id, topic, payload).
    event: Event,
    /// `attempts` *before* this delivery — used to size the next backoff.
    attempts: i32,
    /// The row's `max_attempts` — dead-letter threshold.
    max_attempts: i32,
}

/// The outbox producer + dispatcher.
///
/// Cheap to share: wrap in `Arc` and [`EventDispatcher::spawn`] the loop, or call
/// [`EventDispatcher::publish`] (an associated fn — no instance needed) from any
/// producer.
pub struct EventDispatcher {
    /// Koji DB connection (MySQL 8+ / MariaDB 10.6+ for `SKIP LOCKED`).
    db: DatabaseConnection,
    /// The subscribers an event is delivered to (every one whose
    /// `interested_in(topic)` is true).
    subscribers: Vec<Arc<dyn Subscriber>>,
    /// This process identity, written to `locked_by` on claim (e.g.
    /// `hostname-pid`).
    worker_id: String,
}

impl EventDispatcher {
    /// Build a dispatcher over `db` with `subscribers`. `worker_id` identifies
    /// this process in `locked_by`.
    pub fn new(
        db: DatabaseConnection,
        subscribers: Vec<Arc<dyn Subscriber>>,
        worker_id: impl Into<String>,
    ) -> Self {
        EventDispatcher {
            db,
            subscribers,
            worker_id: worker_id.into(),
        }
    }

    // ---------------------------------------------------------------------
    // Producer: publish (pure outbox append)
    // ---------------------------------------------------------------------

    /// Append a `pending` event to the outbox and return its [`EventId`].
    ///
    /// An associated function (takes `db`, not `self`) so producers can emit
    /// events without the dispatcher loop running — delivery is picked up
    /// whenever a loop next claims due rows. `next_attempt_at` is `NOW()`, so the
    /// row is immediately due.
    pub async fn publish(
        db: &DatabaseConnection,
        topic: &str,
        payload: &impl Serialize,
    ) -> Result<EventId, PublishError> {
        let payload_json = serde_json::to_value(payload)?;
        let id = EventId::new();
        db.execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO `event_outbox` \
               (`public_id`, `topic`, `payload`, `status`, `next_attempt_at`, `created_at`) \
             VALUES (?, ?, ?, 'pending', NOW(), NOW())",
            [
                Value::from(id.as_string()),
                Value::from(topic.to_owned()),
                Value::from(payload_json),
            ],
        ))
        .await?;
        Ok(id)
    }

    // ---------------------------------------------------------------------
    // Dispatcher loop
    // ---------------------------------------------------------------------

    /// Spawn the claim→deliver→backoff loop, returning a [`DispatcherHandle`]
    /// for graceful shutdown.
    ///
    /// The loop runs until [`DispatcherHandle::shutdown`] is called (or the
    /// handle is [`DispatcherHandle::abort`]ed). It is safe to run multiple
    /// dispatchers (this process or others) against the same DB: `SKIP LOCKED`
    /// guarantees no row is double-claimed.
    pub fn spawn(self: Arc<Self>) -> DispatcherHandle {
        let shutdown = Arc::new(Shutdown::default());
        let loop_shutdown = Arc::clone(&shutdown);
        let handle = tokio::spawn(async move {
            self.dispatch_loop(loop_shutdown).await;
        });
        DispatcherHandle {
            handle: Some(handle),
            shutdown,
        }
    }

    /// The dispatch loop body (spec). Runs until `shutdown` is signaled.
    async fn dispatch_loop(self: Arc<Self>, shutdown: Arc<Shutdown>) {
        log::info!("[koji-events] dispatcher started (id={})", self.worker_id);

        loop {
            if shutdown.is_requested() {
                break;
            }

            let claimed = match self.claim_due().await {
                Ok(Some(row)) => row,
                Ok(None) => {
                    // Nothing due: wait for a poll tick or a shutdown signal.
                    tokio::select! {
                        _ = tokio::time::sleep(IDLE_POLL) => {}
                        _ = shutdown.wake.notified() => break,
                    }
                    continue;
                }
                Err(e) => {
                    log::error!("[koji-events] claim error: {e}; backing off");
                    tokio::time::sleep(ERROR_BACKOFF).await;
                    continue;
                }
            };

            self.deliver_claimed(claimed).await;
        }

        log::info!("[koji-events] dispatcher stopped (id={})", self.worker_id);
    }

    /// Atomically claim the next due outbox row (spec), or `None` if none is due.
    ///
    /// In one transaction:
    /// 1. `SELECT … WHERE (status='pending' OR (status='delivering' AND
    ///    lease_expires < NOW())) AND next_attempt_at <= NOW() AND attempts <
    ///    max_attempts ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED`.
    /// 2. `UPDATE … status='delivering', locked_by=?, lease_expires=NOW()+60s`.
    async fn claim_due(&self) -> Result<Option<ClaimedEvent>, DbErr> {
        let txn = self.db.begin().await?;

        let candidate = txn
            .query_one(Statement::from_string(
                DbBackend::MySql,
                "SELECT `id`, `public_id`, `topic`, `payload`, `attempts`, `max_attempts` \
                 FROM `event_outbox` \
                 WHERE (`status` = 'pending' OR (`status` = 'delivering' AND `lease_expires` < NOW())) \
                   AND `next_attempt_at` <= NOW() \
                   AND `attempts` < `max_attempts` \
                 ORDER BY `id` ASC \
                 LIMIT 1 \
                 FOR UPDATE SKIP LOCKED",
            ))
            .await?;

        let Some(row) = candidate else {
            txn.commit().await?;
            return Ok(None);
        };

        let id: u64 = row.try_get("", "id")?;
        let public_id: String = row.try_get("", "public_id")?;
        let topic: String = row.try_get("", "topic")?;
        let payload: serde_json::Value = row.try_get("", "payload")?;
        let attempts: i32 = row.try_get("", "attempts")?;
        let max_attempts: i32 = row.try_get("", "max_attempts")?;

        // Take the lease + mark delivering.
        txn.execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `event_outbox` SET `status` = 'delivering', `locked_by` = ?, \
               `lease_expires` = NOW() + INTERVAL ? SECOND \
             WHERE `id` = ?",
            [
                Value::from(self.worker_id.clone()),
                Value::from(LEASE_SECS),
                Value::from(id),
            ],
        ))
        .await?;

        txn.commit().await?;

        Ok(Some(ClaimedEvent {
            id,
            event: Event {
                id: public_id,
                topic,
                payload,
            },
            attempts,
            max_attempts,
        }))
    }

    /// Deliver one claimed event to every interested subscriber, with a
    /// heartbeat running, then persist the terminal/backoff state.
    async fn deliver_claimed(self: &Arc<Self>, claimed: ClaimedEvent) {
        let event = &claimed.event;
        log::debug!(
            "[koji-events] claimed event id={} public_id={} topic={} attempt={}",
            claimed.id,
            event.id,
            event.topic,
            claimed.attempts
        );

        // Heartbeat: renew the lease every 20s so another dispatcher doesn't
        // reclaim this row mid-delivery. Stopped via its own Notify.
        let stop_heartbeat = Arc::new(Notify::new());
        let heartbeat = spawn_heartbeat(self.db.clone(), claimed.id, Arc::clone(&stop_heartbeat));

        // Deliver to every interested subscriber; collect the first failure.
        let result = self.deliver_to_subscribers(event).await;

        // Stop the heartbeat before the terminal write. `notify_one` (NOT
        // `notify_waiters`): a fast delivery can finish before the freshly-spawned
        // heartbeat task reaches its `stop.notified()` await; `notify_waiters`
        // would lose that signal and `heartbeat.await` would hang the dispatcher.
        // `notify_one` stores a permit so the next `notified()` completes at once.
        stop_heartbeat.notify_one();
        if let Err(e) = heartbeat.await {
            log::warn!(
                "[koji-events] heartbeat join error for event id={}: {e}",
                claimed.id
            );
        }

        match result {
            Ok(()) => self.mark_delivered(claimed.id).await,
            Err(err) => self.mark_failed(&claimed, &err.to_string()).await,
        }
    }

    /// Deliver to each subscriber whose `interested_in(topic)` is true. Returns
    /// the first [`crate::DeliverError`] (so the whole event retries), or `Ok`
    /// when every interested subscriber accepts.
    async fn deliver_to_subscribers(
        &self,
        event: &Event,
    ) -> Result<(), crate::error::DeliverError> {
        for sub in &self.subscribers {
            if !sub.interested_in(&event.topic) {
                continue;
            }
            if let Err(e) = sub.deliver(event).await {
                log::warn!(
                    "[koji-events] subscriber '{}' failed for event {}: {e}",
                    sub.name(),
                    event.id
                );
                return Err(e);
            }
        }
        Ok(())
    }

    /// Mark a row `delivered` with `delivered_at = NOW()`. Best-effort: a
    /// persist failure is logged (the lease lapses and the row is reclaimed; a
    /// successful re-delivery is idempotent on the receiver via
    /// `X-Koji-Event-Id`).
    async fn mark_delivered(&self, id: u64) {
        let stmt = Statement::from_sql_and_values(
            DbBackend::MySql,
            "UPDATE `event_outbox` SET `status` = 'delivered', `delivered_at` = NOW(), \
               `last_error` = NULL \
             WHERE `id` = ?",
            [Value::from(id)],
        );
        if let Err(e) = self.db.execute(stmt).await {
            log::error!("[koji-events] failed to mark event id={id} delivered: {e}");
        }
    }

    /// Record a failed delivery: `attempts + 1`, `last_error`, and either back
    /// off (`status='pending'`, `next_attempt_at = NOW() + backoff`) or
    /// dead-letter (`status='dead'`) once `attempts + 1 >= max_attempts`.
    async fn mark_failed(&self, claimed: &ClaimedEvent, last_error: &str) {
        let new_attempts = claimed.attempts + 1;
        let stmt = if new_attempts >= claimed.max_attempts {
            // Exhausted → dead-letter. (We still bump attempts so the stored
            // count reflects the final try.)
            Statement::from_sql_and_values(
                DbBackend::MySql,
                "UPDATE `event_outbox` SET `status` = 'dead', `attempts` = ?, \
                   `last_error` = ?, `locked_by` = NULL, `lease_expires` = NULL \
                 WHERE `id` = ?",
                [
                    Value::from(new_attempts),
                    Value::from(last_error.to_owned()),
                    Value::from(claimed.id),
                ],
            )
        } else {
            let delay = backoff_secs(new_attempts);
            Statement::from_sql_and_values(
                DbBackend::MySql,
                "UPDATE `event_outbox` SET `status` = 'pending', `attempts` = ?, \
                   `last_error` = ?, `next_attempt_at` = NOW() + INTERVAL ? SECOND, \
                   `locked_by` = NULL, `lease_expires` = NULL \
                 WHERE `id` = ?",
                [
                    Value::from(new_attempts),
                    Value::from(last_error.to_owned()),
                    Value::from(delay),
                    Value::from(claimed.id),
                ],
            )
        };
        if let Err(e) = self.db.execute(stmt).await {
            log::error!(
                "[koji-events] failed to record failure for event id={}: {e}",
                claimed.id
            );
        }
    }
}

/// Spawn the heartbeat task for a delivering row: every [`HEARTBEAT_SECS`],
/// extend `lease_expires` to `NOW() + LEASE_SECS`. Exits when `stop` fires.
fn spawn_heartbeat(db: DatabaseConnection, event_id: u64, stop: Arc<Notify>) -> JoinHandle<()> {
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
                        "UPDATE `event_outbox` SET `lease_expires` = NOW() + INTERVAL ? SECOND \
                         WHERE `id` = ? AND `status` = 'delivering'",
                        [Value::from(LEASE_SECS), Value::from(event_id)],
                    );
                    if let Err(e) = db.execute(stmt).await {
                        // Not fatal on its own; if renews keep failing the lease
                        // lapses and another dispatcher reclaims.
                        log::warn!(
                            "[koji-events] heartbeat renew failed for event id={event_id}: {e}"
                        );
                    }
                }
            }
        }
    })
}

/// Graceful-shutdown signal shared with the dispatch loop.
///
/// Two parts, like the koji-jobs worker: an [`AtomicBool`] carries the durable
/// "shutdown requested" flag (cheaply polled between claims), and a [`Notify`]
/// wakes the loop if it is parked waiting for work.
#[derive(Default)]
struct Shutdown {
    flag: AtomicBool,
    wake: Notify,
}

impl Shutdown {
    fn trigger(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    fn is_requested(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Owns the spawned dispatch task + its shutdown signal.
///
/// Drop does **not** auto-stop the loop (the `JoinHandle` would just detach);
/// call [`DispatcherHandle::shutdown`] for a graceful stop, or
/// [`DispatcherHandle::abort`] to cancel immediately.
pub struct DispatcherHandle {
    handle: Option<JoinHandle<()>>,
    shutdown: Arc<Shutdown>,
}

impl DispatcherHandle {
    /// Signal the loop to stop after its current delivery and wait for it to
    /// finish.
    pub async fn shutdown(mut self) {
        self.shutdown.trigger();
        if let Some(handle) = self.handle.take()
            && let Err(e) = handle.await
        {
            log::error!("[koji-events] dispatcher join error during shutdown: {e}");
        }
    }

    /// Abort the loop immediately without waiting for an in-flight delivery.
    pub fn abort(mut self) {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::backoff_secs;

    #[test]
    fn backoff_follows_powers_of_two_until_the_cap() {
        // attempts → 2^attempts seconds, capped at 3600.
        assert_eq!(backoff_secs(1), 2);
        assert_eq!(backoff_secs(2), 4);
        assert_eq!(backoff_secs(3), 8);
        assert_eq!(backoff_secs(4), 16);
        assert_eq!(backoff_secs(5), 32);
        assert_eq!(backoff_secs(6), 64);
        assert_eq!(backoff_secs(7), 128);
        assert_eq!(backoff_secs(8), 256);
        assert_eq!(backoff_secs(9), 512);
        assert_eq!(backoff_secs(10), 1024);
        assert_eq!(backoff_secs(11), 2048);
    }

    #[test]
    fn backoff_is_capped_at_one_hour() {
        // 2^12 = 4096 > 3600 → clamps to the 3600s cap, and stays there.
        assert_eq!(backoff_secs(12), 3600);
        assert_eq!(backoff_secs(13), 3600);
        assert_eq!(backoff_secs(20), 3600);
        assert_eq!(backoff_secs(62), 3600);
        // Guard the shift-width edge: a huge attempts count must not panic /
        // overflow — it just clamps.
        assert_eq!(backoff_secs(63), 3600);
        assert_eq!(backoff_secs(64), 3600);
        assert_eq!(backoff_secs(i32::MAX), 3600);
    }

    #[test]
    fn backoff_floors_at_one_second_for_nonpositive_attempts() {
        assert_eq!(backoff_secs(0), 1);
        assert_eq!(backoff_secs(-1), 1);
    }
}
