# Koji V2 — workspace modernization (P0–P7)

Refactors the fused `model` crate + 748-line `Args` super-struct + inline synchronous algorithm execution + direct controller-DB writes + legacy RDM support into a clean Cargo workspace with a durable job queue, an event/outbox→Dragonite pipeline, and a `/api/v2` JSend surface. One long-lived branch; **every commit compiles + passes tests.**

## Crate topology (acyclic; `koji-core` is the sink)
```
koji-core ← koji-db, koji-scanner, koji-plugins, koji-jobs, koji-events, koji-dragonite, algorithms
algorithms ← koji-service ;  koji-{jobs,events,dragonite,db,scanner,plugins} ← koji-service
koji-service ← bins/koji-server (pkg `koji`) ;  koji-{db,jobs,service} ← bins/koji-cli
```

## What landed, by phase
- **P0** workspace skeleton (`crates/` + `bins/`).
- **P1** split `model` → `koji-core` (geometry/enums/config/conversions) + `koji-db` (entities/queries) + `koji-scanner` (golbat read); broke `Args` into 8 config structs.
- **P2** clustering/routing/bootstrap take config structs (was 15/7/10 primitives).
- **P3** new infra crates: `koji-jobs` (DB-backed queue: `FOR UPDATE SKIP LOCKED`, lease/heartbeat, dedup, sync-bridge), `koji-events` (outbox + dispatcher + `Subscriber`/`WebhookSubscriber`), `koji-dragonite` (typed `/v2/areas` client).
- **P4** `/api/v2`: `AppState`, JSend, calc-as-a-job (`CalculateHandler`), `/jobs` + `/calc/*` sync-bridge + typed CRUD resources + `/meta/algorithms` + `/healthz`. `/api/v1` kept as a best-effort shim.
- **P5** events→Dragonite: **koji-dragonite reconciled against the REAL `/v2/areas` contract** (V2Response `ok/error` envelope, full `ApiArea` + per-mode blocks, per-slot tri-state `Feature` geofence, `meta` pagination); `DragoniteSubscriber` + dispatcher spawned; `POST /api/v2/geofences/:id/publish` → `area.geofence_updated` (linkage-gated).
- **P6** RDM removal: dropped `ScannerType`/RDM/Hybrid + the controller DB connection (`KojiDb` 3→2 conns); deleted the controller-write/RDM-import paths. Controller writes are now events→Dragonite.
- **P7** `api`→`koji-service` rename · `koji-cli` (`worker|enqueue|get`) · `koji-plugins` extracted + formalized (TOML manifest + JSON stdio protocol + registry + `KOJI_PLUGINS_DIR`) · clippy deny-level cleared + `--fix` sweep · OpenAPI 3.1 doc at `GET /api/v2/openapi.yaml` · React client `scanner_type` collapse.

## Runtime verification (live dev DBs + a mock Dragonite)
Migrations applied to `dev_koji`; v1 + v2 calc (cluster/route/bootstrap) over real golbat data; v2 jobs async + sync-bridge + content-hash dedup; v2 CRUD lifecycle; the full **P5 publish→outbox(`delivered`)→dispatcher→`PATCH /v2/areas/{id}`** chain (Bearer + `{geofence: Feature}`); server boots with **`CONTROLLER_DB_URL` unset** (2-conn `KojiDb`, no panic). `cargo test --workspace` + `cargo clippy` (deny-level) green.

## Breaking changes
- `/api/v1` is no longer byte-compatible with the old contract (the `Args` changes + RDM removal). Dragonite migrates to `/api/v2`.
- `ScannerType`/RDM/Hybrid + `CONTROLLER_DB_URL`/`UNOWN_DB_URL`/`UNOWN_DB`/`DATABASE_URL` env fallbacks removed; the controller DB connection is replaced by the Dragonite HTTP client.
- v1 `push_to_prod` (geofence/route/project) + the `/internal/routes` RDM-import endpoints removed (superseded by v2 events→Dragonite publish).

## v1 calc is now queue-backed (P7)
Every `/api/v1/calc/*` endpoint except `/area` resolves its async inputs, runs through the **same job queue + sync bridge** as v2 (`CalculateHandler`), and re-wraps the result in the legacy `{message,status,status_code,data,stats}` envelope — removing the last inline-synchronous algorithm execution path. The `CalculateHandler` was extended to cover all v1 modes (added `reroute` route-only + `route-stats` stats-only); `/area` (pure geometry sum) stays inline. This surfaced + fixed a **pre-existing queue race**: the per-job heartbeat used the lossy `Notify::notify_waiters()` to stop, which a near-instant handler (reroute/route-stats, ~1ms) could outrun → the heartbeat signal was lost, wedging the worker and stranding the job in `running`. Switched to `notify_one()` (stores a permit); this also hardens fast v2 calc jobs.

## Intentionally deferred (follow-ups, not blockers)
- **`area.route_updated` producer** — the subscriber arm + payload + `area_route_patch` exist + are unit-tested, but no producer is wired (needs a koji-route-mode→`AreaMode` mapping decision).
- **clippy-pedantic manual sweep** — residual ~manual warnings (large-variant boxing, manual trait impls); low-value style churn.
- **Calc `save_to_db`/`save_to_scanner` side-effects** in v2 (deferred since P4; v1 retains its `save_to_db`).
