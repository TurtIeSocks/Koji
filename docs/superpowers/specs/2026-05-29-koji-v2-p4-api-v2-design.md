# Koji V2 — Phase 4: API v2 + v1 shim (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation. Maintainer steer: **clean v2, best practices — v1 byte-compat is moot** (the `Args` changes already break the old contract; this is v2, not v1.1). Assumptions = async review checkpoint.
**Parent:** architecture §6; job-queue spec §3/§7 (sync bridge). Wires the P3 infra (koji-jobs/events/dragonite).

## Crate approach (decided)
**Evolve the existing `api` crate in place** into the v2 service rather than building a parallel `koji-service` (avoids duplicating the actix bootstrap + reworking 15 handler files into a second crate). Add `AppState` + JSend + an `/api/v2` scope; keep `/api/v1` as the shim. **Rename `api` → `koji-service` as a final cosmetic step** (P4 tail or P7) — the structure is what matters.

## AppState (managed)
Replace the bare `web::Data<KojiDb>` with one managed `AppState`:
```rust
pub struct AppState {
    pub db: koji_db::KojiDb,
    pub jobs: Arc<koji_jobs::JobQueue>,
    pub events: Arc<koji_events::EventDispatcher>,
    pub dragonite: Option<koji_dragonite::DragoniteClient>, // None until configured
    pub nominatim: nominatim::Client,
}
```
Built once in `start()`; workers spawned (`jobs.spawn_workers(KOJI_WORKER_CONCURRENCY, registry)`). The `CalculateHandler` is registered in the `HandlerRegistry`.

## JSend envelope (v2)
```rust
#[serde(tag="status", rename_all="lowercase")]
enum JSend<T> { Success{data:T}, Fail{data:Value}, Error{message:String, code:Option<String>, data:Option<Value>} }
```
v2 handlers return `JSend` + a semantic HTTP status. (Mirrors `koji_dragonite::jsend` — consider sharing later.)

## Calc as a job (the v2 heart)
- Move the calc bodies (`api/public/v1/calculate.rs` cluster/route/bootstrap/reroute) into a `CalculateHandler: koji_jobs::JobHandler` whose `run(payload, ctx)` deserializes a `CalcPayload` (the resolved `ArgsUnwrapped`-equivalent: area FC + data_points + the koji-core config structs), runs `clustering/routing/bootstrap`, returns `{ data, stats }` JSON. Phase markers via `ctx.progress`.
- **v2:** `POST /api/v2/jobs` (kind+payload) → `202 {job_id}`; `GET /api/v2/jobs/:id` → status/progress then result; `POST /api/v2/calc/*` sugar builds the payload + enqueues. `DELETE /jobs/:id` cancels.
- **sync bridge** (job-queue §7): `POST /api/v2/calc/*?wait=1` (and the v1 calc shim) → `enqueue_or_attach(priority=HIGH)` then `await_result(290s)` → `200 {data,stats}` | `504` (job continues). dedup via content-hash.

## v2 typed resources (best-practice CRUD)
`GET|POST /api/v2/{geofences,routes,projects,properties,tile-servers}` · `GET|PATCH|DELETE …/:id` · `POST /geofences/:id/publish` (replaces the GET-that-mutates `/push/{id}`). `GET /api/v2/scanner-data/:type` · `POST /api/v2/geo/{s2,simplify,convert}` · `GET /api/v2/meta/algorithms` · `GET /healthz`. Handlers reuse the koji-db `Query::*` methods (already there) + wrap in JSend. `?format=` query replaces the path `/{return_type}` catch-all.
**Macro the CRUD** (`koji-macros`) once the geofences/routes/projects/properties/tile-servers handlers show the repeated shape (architecture §2 convention).

## v1 shim
Keep the existing `/api/v1/*` routes (already implemented in `api/public/v1`). Re-point calc through the sync-bridge (so v1 calc is queue-backed too) and keep the legacy `{message,status,status_code,data,stats}` envelope (the existing `Response` type). **No byte-chasing** — best-effort legacy shape; the maintainer adjusts Dragonite to the v2 contract over time.

## Sub-commit plan
1. **AppState + JSend + skeleton** — `AppState` struct, JSend types, wire `JobQueue`/`EventDispatcher`/optional `DragoniteClient` into `start()`, spawn workers, add `/healthz` + `/api/v2/meta/algorithms`. Green.
2. **CalculateHandler + sync bridge + v2 jobs endpoints** — the `JobHandler`, `CalcPayload`, `POST/GET /api/v2/jobs`, `/api/v2/calc/*` (+`?wait`). Re-point v1 calc through it. Green.
3. **v2 typed resources** (+ macro) + `?format=`. Green.
4. **Rename `api` → `koji-service`** + point `bins/koji-server`. Green. Full test.

## Assumptions (DELEGATE CHECKPOINT)
1. **Evolve `api` in place; rename to koji-service last.** (Lower churn than a parallel crate; same end-state.)
2. **v1 byte-compat dropped** per maintainer — v1 shim keeps the legacy envelope shape but isn't byte-verified against old Dragonite; Dragonite migrates to v2.
3. **Calc runs through koji-jobs** for both v1 (sync bridge) and v2 — single execution pipeline (job-queue spec goal). Default 1 CPU worker.
4. **`CalcPayload`** = the resolved inputs + koji-core config structs (serializable); the `model::api::args::Args.init()` resolution feeds it. (`model` still parses the v1/v2 request bodies until it dissolves.)
5. **Events emitted on calc-persist + publish** are wired in **P5** (not P4) — P4 just makes the queue + endpoints; `events.publish(...)` calls land with the DragoniteSubscriber in P5.
6. **Runtime-unverified** (no DB / no live calc in sandbox): the whole HTTP+queue path. Maintainer integration-tests. Algorithm internals unchanged.
