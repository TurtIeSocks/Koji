# Jobs & the queue — how Koji executes async work

Answers to "all jobs sequential? all parallel? some concurrency limit?" (beta
feedback 2026-07-15). Everything below is code-verified; citations point at the
implementation.

## TL;DR

Jobs run **sequentially by default, and that is deliberate.** One worker claims
one job at a time; the CPU-bound algorithms inside a job (clustering, tsp-mt
routing, bootstrap) are already rayon-parallel, so a single job saturates every
core. Running two calc jobs concurrently would oversubscribe the same core pool
and make both slower.

## Mechanics

- The queue is a real MySQL `job` table — not in-memory. Jobs survive a restart
  while still `queued`. (`crates/migration/src/m20260529_000001_create_job_table.rs`)
- Workers claim with `SELECT … FOR UPDATE SKIP LOCKED`, so multiple koji
  processes can share one DB safely. (`crates/koji-jobs/src/queue.rs:448-521`)
- Worker count = env `KOJI_WORKER_CONCURRENCY`, default **1**, floored at 1.
  (`crates/koji-service/src/lib.rs:255-261`, `crates/koji-jobs/src/worker.rs:118-119`)
- Priority affects claim **order** only (calc enqueues at priority 100), never
  parallelism. There is no per-job-kind concurrency policy.
- Lifecycle: `queued → running → succeeded | failed | canceled`.
  (`crates/koji-jobs/src/entity.rs:54-82`)

## API surface

- `POST /api/v2/jobs` → always async: `202 Accepted` + `Location` + `job_id`.
- `GET /api/v2/jobs/{id}?wait=N` → long-polls up to 290s and **always returns
  the job resource with 200** (running or terminal) — never a 504.
- `GET /api/v2/jobs` lists; `DELETE /api/v2/jobs/{id}` requests cooperative cancel.

## Known limitation (accepted, not a bug)

Every job inserts with `max_attempts = 1` (column default). A job that was
`running` when the process died is marked `failed` on the next claim after its
60s lease lapses — it is **not retried**. Defensible for expensive calc jobs;
revisit only if it bites.

## Tuning

Leave `KOJI_WORKER_CONCURRENCY=1` unless a workload appears that is mostly
IO-bound (rayon idle). Raising it never speeds up a single calc job.
