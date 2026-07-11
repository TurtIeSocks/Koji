//! The persistent, DB-backed queue runtime (design spec §5–§7, §10).
//!
//! [`JobQueue`] is the producer/consumer API over the `job` table. It is generic
//! — it knows nothing about what a job *does* (that's a [`crate::JobHandler`]).
//!
//! Concurrency model:
//! - **Claiming** ([`JobQueue::claim`]) is multi-process-safe via `FOR UPDATE
//!   SKIP LOCKED` inside a transaction (spec §5). Multiple worker processes
//!   against the same DB never double-claim.
//! - **Coalescing** ([`JobQueue::enqueue_or_attach`]) collapses duplicate
//!   in-flight work and serves a 5-minute grace cache (spec §6), inside a txn
//!   that takes `FOR UPDATE` over the dedup set.
//! - **Waking** is two-tier: an in-process [`tokio::sync::Notify`] pokes the
//!   local worker on enqueue (no poll latency), and in-process oneshot waiters
//!   ([`JobQueue::waiters`]) give the sync bridge an instant wake when the local
//!   worker finishes. Cross-process waiters fall back to DB polling (spec §7).
//!
//! The hot SQL is hand-written (`Statement`) rather than built with the query
//! builder: the claim/dedup queries need MySQL-specific clauses the builder
//! makes awkward, and matching the spec's SQL verbatim keeps intent legible.

use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait, Value,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::{Notify, oneshot};

use crate::entity::{self, JobStatus};
use crate::error::{AwaitError, EnqueueError};
use crate::types::{JobEventSink, JobId, JobOutcome, JobRecord};

/// Lease duration applied on claim/renew: a claimed job's `lease_expires` is set
/// `NOW() + LEASE_SECS` (spec §9: 60s lease).
pub(crate) const LEASE_SECS: i64 = 60;

/// Grace window for the dedup cache: a `succeeded` row younger than this serves
/// a duplicate request from its cached result (spec §6/§9: 5 minutes).
const GRACE_SECS: i64 = 300;

/// DB-poll cadence for `await_result` when no local oneshot fires — catches
/// completions done by another process's worker (spec §7: 500ms).
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// A row handed to a worker by [`JobQueue::claim`]: just the fields the worker
/// needs to run + persist. (The full row is the entity [`entity::Model`].)
#[derive(Debug, Clone)]
pub struct ClaimedJob {
    /// Numeric PK (`job.id`) — used for targeted UPDATEs and progress writes.
    pub id: u64,
    /// API-facing id (`public_id`) — used to wake waiters.
    pub public_id: JobId,
    /// Handler kind to dispatch to.
    pub kind: String,
    /// The job payload to pass to the handler.
    pub payload: serde_json::Value,
    /// Attempt count *after* this claim (post-increment), for logging.
    pub attempts: i32,
}

/// The queue handle: producer (`enqueue_or_attach`), sync-bridge
/// (`await_result`), reader (`get`), control (`cancel`), and worker primitives
/// (`claim`, `notify_local_waiters`).
///
/// Cheap to share: wrap in `Arc` and hand clones to workers / HTTP handlers.
#[derive(Clone)]
pub struct JobQueue {
    /// Koji DB connection (MySQL 8+ / MariaDB 10.6+ for `SKIP LOCKED`).
    pub db: DatabaseConnection,
    /// Poked on local enqueue so the in-process worker skips poll latency
    /// (spec §8). Shared with the worker loop.
    pub notify: Arc<Notify>,
    /// In-process waiter registry: `public_id` → oneshot senders awaiting that
    /// job's outcome. Never persisted; cross-process waiters poll instead
    /// (spec §4/§7).
    pub waiters: Arc<DashMap<String, Vec<oneshot::Sender<JobOutcome>>>>,
    /// Running-job cancel tokens: `public_id` → the token handed to that job's
    /// handler ctx. The worker registers on claim and removes on completion;
    /// `cancel` flips the token so same-process handlers see the signal
    /// immediately (cross-process workers pick up `phase='canceling'` via their
    /// heartbeat).
    pub running_tokens: Arc<DashMap<String, crate::types::CancelToken>>,
    /// This process/worker-pool identity, written to `locked_by` on claim.
    pub worker_id: String,
    /// Optional realtime event sink. When set, the worker emits status + progress
    /// events through it (transport-agnostic: the hub in koji-service implements
    /// it; koji-jobs has no direct dependency on the hub).
    pub event_sink: Option<Arc<dyn JobEventSink>>,
}

impl JobQueue {
    /// Build a queue over `db`. `worker_id` identifies this process in
    /// `locked_by` (e.g. `hostname-pid`).
    pub fn new(db: DatabaseConnection, worker_id: impl Into<String>) -> Self {
        JobQueue {
            db,
            notify: Arc::new(Notify::new()),
            waiters: Arc::new(DashMap::new()),
            running_tokens: Arc::new(DashMap::new()),
            worker_id: worker_id.into(),
            event_sink: None,
        }
    }

    /// Attach a realtime event sink. Returns `self` for builder chaining.
    ///
    /// The sink receives `on_job_status` on claim (`running`) and on terminal
    /// outcomes (`succeeded`/`failed`/`canceled`), and `on_job_progress` on each
    /// `ProgressHandle::set` call. All calls are fire-and-forget — a sink error
    /// never fails a job.
    pub fn with_event_sink(mut self, sink: Arc<dyn JobEventSink>) -> Self {
        self.event_sink = Some(sink);
        self
    }

    // ---------------------------------------------------------------------
    // Producer: enqueue_or_attach (spec §6)
    // ---------------------------------------------------------------------

    /// Enqueue a job, coalescing onto an in-flight/cached one when a `dedup_key`
    /// is supplied (spec §6).
    ///
    /// Within a single transaction that takes `FOR UPDATE` over the dedup set:
    /// 1. a `succeeded` row with the same key finished within the 5-min grace →
    ///    return its id (caller's [`Self::await_result`] short-circuits on its
    ///    cached result, no new job);
    /// 2. a `queued`/`running` row with the same key → return its id (coalesce;
    ///    the caller attaches a waiter);
    /// 3. otherwise → `INSERT` a fresh `queued` job and return its new id.
    ///
    /// With `dedup_key = None`, always inserts (no coalescing). Pokes the local
    /// [`Notify`] after a successful insert so the in-process worker wakes
    /// immediately.
    pub async fn enqueue_or_attach(
        &self,
        kind: &str,
        dedup_key: Option<&str>,
        payload: &impl Serialize,
        priority: i16,
    ) -> Result<JobId, EnqueueError> {
        let payload_json = serde_json::to_value(payload)?;

        // Coalescing requires a txn so the dedup lookup + insert are atomic
        // against concurrent enqueues of the same key.
        let txn = self.db.begin().await?;

        if let Some(key) = dedup_key {
            // (1) Grace-cached success? Lock the dedup set so a concurrent
            // enqueue can't slip a second insert past us between SELECT and
            // INSERT. `NOW() - INTERVAL GRACE_SECS SECOND` bounds the cache.
            let cached = txn
                .query_one(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "SELECT `public_id` FROM `job` \
                     WHERE `dedup_key` = ? AND `status` = 'succeeded' \
                       AND `finished_at` > (NOW() - INTERVAL ? SECOND) \
                     ORDER BY `finished_at` DESC LIMIT 1 FOR UPDATE",
                    [Value::from(key.to_owned()), Value::from(GRACE_SECS)],
                ))
                .await?;
            if let Some(row) = cached {
                let public_id: String = row.try_get("", "public_id")?;
                txn.commit().await?;
                return parse_public_id(&public_id);
            }

            // (2) In-flight (queued or running) with the same key? Coalesce.
            let inflight = txn
                .query_one(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "SELECT `public_id` FROM `job` \
                     WHERE `dedup_key` = ? AND `status` IN ('queued','running') \
                     ORDER BY `id` ASC LIMIT 1 FOR UPDATE",
                    [Value::from(key.to_owned())],
                ))
                .await?;
            if let Some(row) = inflight {
                let public_id: String = row.try_get("", "public_id")?;
                txn.commit().await?;
                return parse_public_id(&public_id);
            }
        }

        // (3) Insert a fresh job.
        let id = JobId::new();
        txn.execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO `job` \
               (`public_id`, `kind`, `dedup_key`, `status`, `priority`, `payload`, `created_at`) \
             VALUES (?, ?, ?, 'queued', ?, ?, NOW())",
            [
                Value::from(id.as_string()),
                Value::from(kind.to_owned()),
                Value::from(dedup_key.map(str::to_owned)),
                Value::from(priority),
                Value::from(payload_json),
            ],
        ))
        .await?;
        txn.commit().await?;

        // Wake the local worker so a freshly-enqueued job isn't stuck behind the
        // poll interval (spec §8).
        self.notify.notify_one();
        Ok(id)
    }

    // ---------------------------------------------------------------------
    // Sync bridge: await_result (spec §7)
    // ---------------------------------------------------------------------

    /// Wait up to `timeout` for `id` to reach a terminal state, returning its
    /// [`JobOutcome`] (spec §7).
    ///
    /// Short-circuits immediately if the job is already terminal — this is how a
    /// dedup cached-hit (a grace-window `succeeded` row) is served with no extra
    /// code path (spec §10).
    ///
    /// Otherwise it registers an in-process oneshot in [`Self::waiters`] and
    /// `select!`s over three arms:
    /// - the oneshot (local worker finished → instant wake),
    /// - a 500ms DB poll (another process's worker finished it),
    /// - the overall timeout (→ [`AwaitError::Timeout`]; the job keeps running
    ///   and feeds the grace window — never marked failed, spec §11).
    pub async fn await_result(
        &self,
        id: JobId,
        timeout: Duration,
    ) -> Result<JobOutcome, AwaitError> {
        let key = id.as_string();

        // Fast path: already terminal (includes the dedup cached-hit case).
        if let Some(outcome) = self.load_terminal_outcome(&key).await? {
            return Ok(outcome);
        }

        // Register an in-process waiter so a local-worker completion wakes us
        // without waiting for the next poll tick.
        let (tx, rx) = oneshot::channel::<JobOutcome>();
        self.waiters.entry(key.clone()).or_default().push(tx);

        // Re-check after registering: the job may have completed in the race
        // between the fast-path check and the waiter registration, in which case
        // no notify will ever arrive on our oneshot.
        if let Some(outcome) = self.load_terminal_outcome(&key).await? {
            return Ok(outcome);
        }

        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);
        tokio::pin!(rx);

        loop {
            tokio::select! {
                // Local worker finished and notified our oneshot.
                res = &mut rx => {
                    return match res {
                        Ok(outcome) => Ok(outcome),
                        // Sender dropped without sending (shouldn't normally
                        // happen) — fall back to a DB read.
                        Err(_) => self
                            .load_terminal_outcome(&key)
                            .await?
                            .ok_or(AwaitError::NotFound),
                    };
                }
                // Poll the DB: catches completions performed by another process.
                _ = tokio::time::sleep(POLL_INTERVAL) => {
                    if let Some(outcome) = self.load_terminal_outcome(&key).await? {
                        return Ok(outcome);
                    }
                }
                // Overall deadline: 504 for the sync bridge. Job keeps running.
                _ = &mut deadline => {
                    return Err(AwaitError::Timeout);
                }
            }
        }
    }

    /// Load a job by `public_id` and, if it is terminal, project it to a
    /// [`JobOutcome`]. `Ok(None)` means the job exists but is not yet terminal;
    /// `Err(NotFound)` means no such job.
    async fn load_terminal_outcome(
        &self,
        public_id: &str,
    ) -> Result<Option<JobOutcome>, AwaitError> {
        let model = self.find_by_public_id(public_id).await?;
        match model {
            Some(m) => Ok(outcome_from_model(&m)),
            None => Err(AwaitError::NotFound),
        }
    }

    // ---------------------------------------------------------------------
    // Reader / control
    // ---------------------------------------------------------------------

    /// Fetch the observable state of a job (spec §10).
    pub async fn get(&self, id: JobId) -> Result<JobRecord, AwaitError> {
        let model = self
            .find_by_public_id(&id.as_string())
            .await?
            .ok_or(AwaitError::NotFound)?;
        Ok(JobRecord {
            id,
            kind: model.kind,
            status: model.status,
            progress: model.progress,
            phase: model.phase,
            result: model.result,
            error: model.error,
        })
    }

    /// List jobs newest-first (`id DESC`), optionally filtered by `status`,
    /// page-limited by the 1-based `page` + `per_page` (offset is derived
    /// internally as `(page - 1) * per_page`). Returns `(rows, total)` where
    /// `total` is the unpaginated count (for the `meta` block). Spec §10
    /// (observability).
    pub async fn list(
        &self,
        page: i64,
        per_page: i64,
        status: Option<JobStatus>,
    ) -> Result<(Vec<JobRecord>, i64), DbErr> {
        use sea_orm::{
            ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
        };

        let per_page = per_page.max(1);
        let offset = (page.max(1) - 1) * per_page;

        let mut find = entity::Entity::find();
        if let Some(s) = status {
            find = find.filter(entity::Column::Status.eq(s));
        }

        let total = find.clone().count(&self.db).await? as i64;

        let models = find
            .order_by_desc(entity::Column::Id)
            .limit(per_page as u64)
            .offset(offset as u64)
            .all(&self.db)
            .await?;

        let rows = models
            .into_iter()
            .map(|m| JobRecord {
                id: m.public_id.parse().unwrap_or_else(|_| JobId::new()),
                kind: m.kind,
                status: m.status,
                progress: m.progress,
                phase: m.phase,
                result: m.result,
                error: m.error,
            })
            .collect();

        Ok((rows, total))
    }

    /// Request cancellation of a job (spec §9: honored at the next phase
    /// boundary).
    ///
    /// A still-`queued` job is transitioned straight to `canceled` (it will
    /// never be claimed) with the event sink + local waiters notified, since no
    /// worker will ever do it. A `running` job is flagged by writing
    /// `phase='canceling'` AND flipping its registered
    /// [`crate::types::CancelToken`] (same-process signal; a worker in another
    /// process picks the phase flag up via its heartbeat). Terminal jobs are
    /// left untouched.
    pub async fn cancel(&self, id: JobId) -> Result<(), AwaitError> {
        let public_id = id.as_string();
        let res = self
            .db
            .execute(Statement::from_sql_and_values(
                DbBackend::MySql,
                "UPDATE `job` SET `status` = 'canceled', `finished_at` = NOW() \
                 WHERE `public_id` = ? AND `status` = 'queued'",
                [Value::from(public_id.clone())],
            ))
            .await?;

        if res.rows_affected() > 0 {
            // Terminal transition performed outside the worker: honor the
            // JobEventSink contract + wake local awaiters ourselves.
            if let Some(s) = &self.event_sink {
                s.on_job_status(&public_id, "canceled", 0.0, None);
            }
            self.notify_local_waiters(&public_id, JobOutcome::Canceled);
            return Ok(());
        }

        // Not queued: either running (flag it for cooperative cancel) or
        // already terminal (no-op). Setting the phase marker is observable
        // via `get` and signals intent; the worker honors its CancelToken.
        self.db
            .execute(Statement::from_sql_and_values(
                DbBackend::MySql,
                "UPDATE `job` SET `phase` = 'canceling' \
                 WHERE `public_id` = ? AND `status` = 'running'",
                [Value::from(public_id.clone())],
            ))
            .await?;
        // Flip the in-process token so a handler running in THIS process sees
        // the signal at its next `ctx.cancel.is_cancelled()` check.
        if let Some(tok) = self.running_tokens.get(&public_id) {
            tok.cancel();
        }
        Ok(())
    }

    // ---------------------------------------------------------------------
    // Worker primitives
    // ---------------------------------------------------------------------

    /// Atomically claim the next runnable job (spec §5), or `None` if the queue
    /// is empty.
    ///
    /// In one transaction:
    /// 1. `SELECT … WHERE status='queued' OR (running AND lease_expires < NOW())`
    ///    `AND attempts < max_attempts ORDER BY priority DESC, id ASC LIMIT 1`
    ///    `FOR UPDATE SKIP LOCKED` — `SKIP LOCKED` lets concurrent workers grab
    ///    *different* rows without blocking each other.
    /// 2. `UPDATE … status='running', locked_by=?, lease_expires=NOW()+60s,`
    ///    `started_at=COALESCE(started_at, NOW()), attempts=attempts+1`.
    ///
    /// Reclaim-exhaustion rule (spec §5): a reclaimable `running` row whose
    /// `attempts + 1 > max_attempts` is marked `failed` here instead of re-run,
    /// and the claim continues to the next candidate. (A dead worker's calc job
    /// with `max_attempts=1` thus fails on reclaim rather than silently
    /// recomputing expensive work.)
    pub async fn claim(&self) -> Result<Option<ClaimedJob>, DbErr> {
        // Loop so that when we skip an exhausted reclaim (marking it failed) we
        // immediately try for the next candidate within fresh transactions.
        loop {
            let txn = self.db.begin().await?;

            let candidate = txn
                .query_one(Statement::from_string(
                    DbBackend::MySql,
                    "SELECT `id`, `attempts`, `max_attempts` FROM `job` \
                     WHERE (`status` = 'queued' OR (`status` = 'running' AND `lease_expires` < NOW())) \
                       AND `attempts` <= `max_attempts` \
                     ORDER BY `priority` DESC, `id` ASC \
                     LIMIT 1 \
                     FOR UPDATE SKIP LOCKED",
                ))
                .await?;

            let Some(row) = candidate else {
                // Nothing claimable.
                txn.commit().await?;
                return Ok(None);
            };

            let id: u64 = row.try_get("", "id")?;
            let attempts: i32 = row.try_get("", "attempts")?;
            let max_attempts: i32 = row.try_get("", "max_attempts")?;

            // Reclaim-exhaustion: this claim would push attempts past the cap →
            // fail it instead of running, then look for another job.
            if attempts + 1 > max_attempts {
                txn.execute(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "UPDATE `job` SET `status` = 'failed', \
                       `error` = COALESCE(`error`, 'max attempts exceeded on reclaim'), \
                       `finished_at` = NOW() \
                     WHERE `id` = ?",
                    [Value::from(id)],
                ))
                .await?;
                txn.commit().await?;

                // Notify any local waiters that this job failed.
                if let Some(public_id) = self.public_id_of(id).await? {
                    self.notify_local_waiters(
                        &public_id,
                        JobOutcome::Failed {
                            error: "max attempts exceeded on reclaim".to_owned(),
                            code: "internal_error".to_owned(),
                        },
                    );
                }
                continue;
            }

            // Mark it running and take the lease.
            txn.execute(Statement::from_sql_and_values(
                DbBackend::MySql,
                "UPDATE `job` SET `status` = 'running', `locked_by` = ?, \
                   `lease_expires` = NOW() + INTERVAL ? SECOND, \
                   `started_at` = COALESCE(`started_at`, NOW()), \
                   `attempts` = `attempts` + 1 \
                 WHERE `id` = ?",
                [
                    Value::from(self.worker_id.clone()),
                    Value::from(LEASE_SECS),
                    Value::from(id),
                ],
            ))
            .await?;

            // Read back the payload + public_id for the claimed row.
            let claimed = txn
                .query_one(Statement::from_sql_and_values(
                    DbBackend::MySql,
                    "SELECT `public_id`, `kind`, `payload`, `attempts` FROM `job` WHERE `id` = ?",
                    [Value::from(id)],
                ))
                .await?
                .ok_or_else(|| DbErr::Custom("claimed job vanished mid-transaction".to_owned()))?;

            let public_id_str: String = claimed.try_get("", "public_id")?;
            let kind: String = claimed.try_get("", "kind")?;
            let payload: serde_json::Value = claimed.try_get("", "payload")?;
            let new_attempts: i32 = claimed.try_get("", "attempts")?;

            txn.commit().await?;

            let public_id = parse_public_id(&public_id_str)
                .map_err(|_| DbErr::Custom(format!("invalid public_id in row: {public_id_str}")))?;

            return Ok(Some(ClaimedJob {
                id,
                public_id,
                kind,
                payload,
                attempts: new_attempts,
            }));
        }
    }

    /// Deliver `outcome` to every in-process waiter registered for `public_id`
    /// and drop the entry (spec §5/§7).
    ///
    /// Cheap no-op when there are no local waiters (the common case for
    /// CLI/async jobs with no sync client attached). A closed receiver (caller
    /// already timed out / went away) is silently skipped.
    pub fn notify_local_waiters(&self, public_id: &str, outcome: JobOutcome) {
        if let Some((_, senders)) = self.waiters.remove(public_id) {
            for tx in senders {
                // Ignore send errors: the receiver may have timed out already.
                let _ = tx.send(outcome.clone());
            }
        }
    }

    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    /// Load a full job row by `public_id` via the entity.
    async fn find_by_public_id(&self, public_id: &str) -> Result<Option<entity::Model>, DbErr> {
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        entity::Entity::find()
            .filter(entity::Column::PublicId.eq(public_id))
            .one(&self.db)
            .await
    }

    /// Resolve a numeric `job.id` to its `public_id`.
    async fn public_id_of(&self, id: u64) -> Result<Option<String>, DbErr> {
        let row = self
            .db
            .query_one(Statement::from_sql_and_values(
                DbBackend::MySql,
                "SELECT `public_id` FROM `job` WHERE `id` = ?",
                [Value::from(id)],
            ))
            .await?;
        match row {
            Some(r) => Ok(Some(r.try_get("", "public_id")?)),
            None => Ok(None),
        }
    }
}

/// Project a terminal job model to its [`JobOutcome`]; `None` if not terminal.
fn outcome_from_model(m: &entity::Model) -> Option<JobOutcome> {
    match m.status {
        JobStatus::Succeeded => Some(JobOutcome::Succeeded(
            m.result.clone().unwrap_or(serde_json::Value::Null),
        )),
        JobStatus::Failed => Some(JobOutcome::Failed {
            error: m.error.clone().unwrap_or_else(|| "job failed".to_owned()),
            // The persisted error string is the source of truth; a generic code
            // is used when the row predates code capture. (Concrete handlers can
            // encode a code into the message; the sync bridge owns final mapping.)
            code: "internal_error".to_owned(),
        }),
        JobStatus::Canceled => Some(JobOutcome::Canceled),
        JobStatus::Queued | JobStatus::Running => None,
    }
}

/// Parse a stored `public_id` string into a [`JobId`], mapping a decode failure
/// to an [`EnqueueError::Db`] custom error (a malformed `public_id` is a DB
/// integrity problem, not a serialize error).
fn parse_public_id(s: &str) -> Result<JobId, EnqueueError> {
    s.parse::<JobId>()
        .map_err(|_| EnqueueError::Db(DbErr::Custom(format!("invalid public_id stored: {s}"))))
}

/// Compute the content-addressed `dedup_key` for a job (spec §6):
/// `sha256(kind ‖ canonical_json(fields))`, hex-encoded (64 chars → `CHAR(64)`).
///
/// `fields` are normalized to a stable, **field-order-independent** form by
/// serializing through a `BTreeMap` (sorted keys) before hashing, so two callers
/// that pass logically-equal configs in different field orders produce the same
/// key. This is the generic primitive; `koji-service` composes the concrete
/// `kind + config + area/data identity` on top of it.
pub fn dedup_key(kind: &str, fields: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical_json(fields).as_bytes());
    let digest = hasher.finalize();
    // Hex-encode to 64 chars to fit `dedup_key CHAR(64)`.
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Serialize `value` to JSON with object keys recursively sorted, so that
/// semantically-identical values with different key orderings serialize
/// identically. Numbers/strings/arrays keep their order (arrays are order-significant).
fn canonical_json(value: &serde_json::Value) -> String {
    fn canonicalize(value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(map) => {
                // BTreeMap sorts keys; recurse into values.
                let sorted: std::collections::BTreeMap<String, serde_json::Value> = map
                    .iter()
                    .map(|(k, v)| (k.clone(), canonicalize(v)))
                    .collect();
                serde_json::to_value(sorted).unwrap_or(serde_json::Value::Null)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(canonicalize).collect())
            }
            other => other.clone(),
        }
    }
    // Serializing a BTreeMap-backed Value yields sorted-key output deterministically.
    serde_json::to_string(&canonicalize(value)).unwrap_or_default()
}
