> **⚠️ PARTIALLY SUPERSEDED (2026-07-15):** §7 (sync bridge → 504) and §9
> (per-kind retries / configurable `max_attempts`) were never built as
> described. The implemented behavior — always-async 202, `?wait=` long-poll
> that returns 200 (never 504), `max_attempts` fixed at 1 — is specced in
> `2026-06-15-v2-api-redesign.md` §4.1 and documented in `docs/jobs-and-queue.md`.

# Koji Job Queue + Sync-Bridge — Design Spec

**Date:** 2026-05-28
**Status:** Approved (design); pending implementation plan
**Scope:** The persistent algorithm job queue and the synchronous-request bridge that lets the live Dragonite `/api/v1/calc/*` contract ride an async queue. Part of the broader Koji V2 refactor (`refactor-workspace/`); this spec is the deep design for one hotspot.

## 1. Context & problem

Today the actix handler runs clustering/routing **synchronously and inline** (`api/.../calculate.rs`), blocking a web worker for the whole (potentially long) computation. Algorithms are sync + rayon. Dragonite POSTs `/api/v1/calc/{bootstrap,route/pokestop,route/spawnpoint,route/fort}` and **blocks waiting for the route data + a `stats` block**.

V2 introduces a **persistent, DB-backed job queue** so that: long work is decoupled from request lifetime, jobs survive restarts and are observable, the new `koji-cli` shares one execution pipeline, and new clients can submit-and-poll — **without breaking Dragonite's synchronous contract**.

## 2. Goals / non-goals

**Goals**
- Durable queue in the Koji DB (no new infra/broker).
- Preserve the synchronous Dragonite contract (request returns the result + stats).
- Async submit+poll for new v2 clients.
- Shared queue usable by the web server and `koji-cli` (producer, consumer, standalone worker).
- Multi-process-safe claiming; crash recovery.
- Avoid redundant recompute of identical concurrent/retried requests.

**Non-goals**
- Job preemption (rayon work can't be interrupted mid-pass).
- Fine-grained progress (algorithm internals stay untouched — coarse phase markers only).
- Horizontal autoscaling / external brokers.
- Long-lived result caching across data changes.

## 3. Crate placement
- **`koji-jobs`** (generic): `job` entity, claim/lease/heartbeat, worker pool, `JobHandler` trait, `JobQueue` API, `ProgressHandle`, `CancelToken`, in-process waiter registry. No knowledge of clustering.
- **`koji-service`**: concrete `KojiJob` payload, `CalculateHandler` (pulls data via `koji-golbat`, runs `koji-algorithms`, emits persistence events), worker bootstrap in `AppState`, the sync-bridge HTTP glue.
- **`koji-migration`**: the `job` table DDL.

## 4. Schema

```sql
CREATE TABLE job (
  id            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  public_id     CHAR(26)     NOT NULL UNIQUE,    -- ULID, opaque + sortable, API-facing
  kind          VARCHAR(64)  NOT NULL,
  dedup_key     CHAR(64)     NULL,               -- sha256(kind + canonical_json(config) + area/data identity)
  status        ENUM('queued','running','succeeded','failed','canceled') NOT NULL DEFAULT 'queued',
  priority      SMALLINT     NOT NULL DEFAULT 0, -- sync calc = HIGH (e.g. 100), async/CLI = 0
  payload       JSON         NOT NULL,
  result        JSON         NULL,
  error         TEXT         NULL,
  progress      FLOAT        NOT NULL DEFAULT 0,
  phase         VARCHAR(32)  NULL,               -- 'clustering' | 'routing' | ...
  attempts      INT          NOT NULL DEFAULT 0,
  max_attempts  INT          NOT NULL DEFAULT 1,
  locked_by     VARCHAR(64)  NULL,               -- worker id
  lease_expires DATETIME     NULL,
  created_at    DATETIME     NOT NULL,
  started_at    DATETIME     NULL,
  finished_at   DATETIME     NULL,
  INDEX idx_claim (status, priority, id),
  INDEX idx_lease (status, lease_expires),
  INDEX idx_dedup (dedup_key, status, finished_at)
);
```

Waiters for the sync path are held **in-process**: `DashMap<public_id, Vec<oneshot::Sender<JobOutcome>>>`. They are never persisted; cross-process waiters fall back to DB polling.

**Requirement:** Koji DB is MySQL 8+ / MariaDB 10.6+ (`FOR UPDATE SKIP LOCKED`). On older engines, fall back to an optimistic `UPDATE … WHERE status='queued' AND id=?` guarded claim.

## 5. Lifecycle

### Claim (multi-worker safe)
```sql
-- in a txn:
SELECT id FROM job
  WHERE (status='queued' OR (status='running' AND lease_expires < NOW()))
    AND attempts < max_attempts
  ORDER BY priority DESC, id ASC
  LIMIT 1
  FOR UPDATE SKIP LOCKED;
UPDATE job SET status='running', locked_by=?, lease_expires=NOW()+INTERVAL 60 SECOND,
               started_at=COALESCE(started_at, NOW()), attempts=attempts+1
  WHERE id=?;
```
A reclaimable `running` job whose `attempts` would exceed `max_attempts` is transitioned to `failed` instead of re-run.

### Execute
```
worker loop:
  job = claim()                       # else wait on Notify or poll_interval, continue
  spawn heartbeat(job)                # renew lease_expires every 20s
  outcome = spawn_blocking(|| handler.run(job.payload, &ctx))   # CPU/rayon off the runtime
  persist(status = succeeded|failed, result|error, finished_at = NOW())
  stop heartbeat
  notify_local_waiters(job.public_id, outcome)
```

### Crash recovery
Dead worker → lease expires → another worker reclaims via the normal claim query. Calc `max_attempts=1` ⇒ a crashed calc job is marked `failed` on reclaim (no silent recompute of expensive work). The sync client receives a failure and may re-request (a fresh job).

## 6. Dedup + grace (coalesce, no stale cache)

`dedup_key = sha256(kind ‖ canonical_json(normalized_config) ‖ area/data identity)`, where area identity is the geofence id when available, else a hash of the resolved geometry + `DataFilter`.

`enqueue_or_attach(dedup_key, payload, priority)` within a txn (`FOR UPDATE` over the dedup_key set):
1. `succeeded` row, same key, `finished_at > NOW() - 5min` → return it with its cached `result` (no new job).
2. `queued` or `running` row, same key → return its `public_id` (coalesce; caller attaches a waiter).
3. otherwise → `INSERT` a new `queued` job.

The 5-minute grace exists specifically to serve a timed-out sync client's retry from the just-computed result, and to collapse rapid duplicate triggers — **not** to cache across Dragonite's intentional data-refresh recalcs. The janitor purges `succeeded` after 24h, `failed`/`canceled` after 7d.

## 7. Sync-bridge (preserves the Dragonite contract)

```
HTTP calc request (v1 shim or v2 ?wait/Prefer)
  → build config → dedup_key
  → enqueue_or_attach(priority = HIGH)
      • cached-hit  → 200 { data, stats }  (return now)
      • queued/running/new → register in-proc oneshot, then:
          select! {
            oneshot.recv()       // local worker finished → instant wake
            db_poll(500ms)       // a koji-cli worker finished it → caught by poll
            timeout(290s)        // → 504; job KEEPS running and populates the grace window
          }
  → 200 { data, stats }   |   4xx/5xx mapped from job.error
```
Async (v2 default): `POST /jobs` (or `POST /calc/*` sugar) → `202 { job_id }`; client polls `GET /jobs/:id` → status/progress, then result + `stats`. Same queue + same dedup as sync.

`stats` (`{cluster, route, total}`) is part of `result`, so both paths surface it; Dragonite's stats logging is preserved.

## 8. Workers / concurrency
- Default **single** CPU-bound worker (`KOJI_WORKER_CONCURRENCY`, default 1). One rayon job at a time uses all cores without oversubscription.
- Sync calc (`priority` HIGH) is claimed before async/CLI (`priority` 0); no preemption, so a sync request may wait for one in-progress job but is always next.
- Local `enqueue` pokes a `tokio::Notify` so the in-process worker skips poll latency.
- Horizontal scale = additional `koji-cli worker` processes against the same DB (safe via `SKIP LOCKED`).

## 9. Decided defaults
| Concern | Decision |
|---|---|
| Cancel-on-disconnect (sync) | No — job continues (idempotent; feeds grace; may have other waiters). |
| Async cancel | Explicit `DELETE /jobs/:id` → sets a flag; honored at the next phase boundary. |
| Backpressure | `queued` depth > `KOJI_MAX_QUEUE_DEPTH` (default 100) → `429 + Retry-After` for async submits; sync calc bypasses the soft limit up to a hard cap. |
| Lease / heartbeat | 60s lease, 20s heartbeat. |
| Sync timeout | 290s → 504; job continues. |
| Retries | calc `max_attempts=1`; configurable per kind. |
| Progress | coarse phase markers (`queued→clustering→routing→done`). |

## 10. Public interfaces (`koji-jobs`)
```rust
pub trait JobHandler: Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    fn run(&self, payload: serde_json::Value, ctx: &JobCtx) -> Result<serde_json::Value, JobError>;
}
pub struct JobCtx { pub cancel: CancelToken, pub progress: ProgressHandle }

impl JobQueue {
    pub async fn enqueue_or_attach(&self, kind: &str, dedup_key: Option<&str>,
                                   payload: &impl Serialize, priority: i16) -> Result<JobId>;
    // await_result short-circuits: if `id` is already terminal (succeeded/failed/canceled)
    // it returns immediately — this is how a dedup cached-hit is served with no extra path.
    pub async fn await_result(&self, id: JobId, timeout: Duration) -> Result<JobOutcome, AwaitError>;
    pub async fn get(&self, id: JobId) -> Result<JobRecord>;
    pub async fn cancel(&self, id: JobId) -> Result<()>;
    pub fn spawn_workers(self: Arc<Self>, n: usize, registry: HandlerRegistry) -> WorkerSet;
}
```

## 11. Error handling
- `JobError` (handler-side, `thiserror`) → serialized to `job.error` + a v2 error code (`validation_error`, `internal_error`, …).
- Sync bridge maps job failure → JSend error envelope with the right HTTP status.
- DB/claim errors are retried with backoff inside the worker loop; they never fail a job by themselves.
- `await` timeout → 504; never marks the job failed.

## 12. Testing strategy
- **Unit:** dedup_key stability (same config → same key; normalized field order); `enqueue_or_attach` three-way branch; claim ordering by priority; lease-expiry reclaim; `max_attempts` exhaustion → failed.
- **Concurrency:** N simultaneous `enqueue_or_attach` with one dedup_key ⇒ exactly one job inserted, all callers attached; two workers never double-claim (SKIP LOCKED).
- **Sync-bridge:** local-worker fast wake; cross-process completion via poll; timeout → 504 + job completes + grace serves the retry.
- **Crash:** kill a worker mid-run ⇒ lease reclaim marks calc job failed (not re-run).
- **Contract:** replay Dragonite's exact `koji/main.go` calc requests through the v1 shim ⇒ byte-compatible `{data, stats}` responses.

## 13. Out of scope / future
- Preemption, fair-share scheduling, per-tenant quotas.
- Result cache beyond the grace window.
- Distributed coordination beyond DB-row claiming.
- Progress streaming (SSE/WebSocket) — possible later via the event system.
