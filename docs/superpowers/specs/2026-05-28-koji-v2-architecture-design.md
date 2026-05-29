# Koji V2 — Architecture Design Spec

**Date:** 2026-05-28
**Status:** Approved (design); pending implementation plans
**Companion specs:** [job-queue + sync-bridge](2026-05-28-koji-job-queue-design.md) (hotspot deep-dive)
**Planning artifacts:** `refactor-workspace/{trace,goals,assessment,map,roadmap}.md`

This is the authoritative design for the Koji V2 refactor — a single long-lived branch, many ordered commits, no client changes in phase 1. It consolidates ten locked decisions and the per-area designs. Each subsystem will get its own deep-dive spec + implementation plan as it's built (the job-queue spec is the first).

## 1. Context

Koji computes clustering/routing for Pokémon-GO scanning and integrates with the UnownHash stack (Golbat data DB + Dragonite controller). Current pain: one fused `model` crate (db + API DTOs + utils), a 748-line `Args` super-struct, 61 inconsistent HTTP routes, synchronous inline algorithm execution, **direct writes to Dragonite's controller DB**, and legacy RDM support. Dragonite calls Koji at `/api/v1/calc/*` (live contract) and is itself being modernized (GeoJSON-native areas in #557, per-mode geofences in #558).

## 2. Goals → decisions

Sixteen goals (see `refactor-workspace/goals.md`). Ten forking decisions, locked:

| # | Decision |
|---|---|
| 1 | **Persistent DB-backed job queue** — jobs table → worker pool → `spawn_blocking`(rayon); pollable; CLI shares it. |
| 2 | **Clean `/api/v2` + `/api/v1` as a thin shim** over the v2 core. Client + Dragonite ride v1 in phase 1. |
| 3 | **Layered ~9-crate workspace at repo root**, crates under `crates/`, bins under `bins/`. |
| 4 | **Durable outbox + dispatcher (retry/backoff) + webhook registry**; the Dragonite pusher is the first built-in subscriber. |
| 5 | **Formalized subprocess plugins** — Rust trait + external dir + manifest + versioned JSON protocol. |
| 6 | Calc surface = **hybrid** (generic `POST /jobs` canonical + `POST /calc/*` sugar). |
| 7 | Response format = **JSend, matching Dragonite v2** (`status`/`data`/`meta`/`error{code,message,field}`). |
| 8 | CRUD = **typed resource handlers** (no generic `{resource}` dispatch). |
| 9 | Dragonite area identity = **stored `dragonite_area_id` linkage** (not name-resolution). |
| 10 | PR #558 fences = **separate per-mode fence entities** (`area_fence` linkage). |

Constraints: big-bang on one branch; no breaking changes to live consumers in phase 1; solo dev; sea-orm + or-tools/VRP retained.

## 3. Workspace & crate topology

Workspace manifest at repo root. `client/` (JS) + `or-tools/` (C++) are non-member siblings, untouched in phase 1.

```
crates/
  koji-core        pure domain — geometry, GeoJSON conversions, config structs, domain enums, errors. NO db/http/tokio.
  koji-macros      proc-macros (kept)
  koji-nominatim   geocoding client (→ core)
  koji-plugins     AlgorithmPlugin trait + SubprocessPlugin + registry (→ core)
  koji-algorithms  clustering/routing/bootstrap/s2/stats (→ core, plugins, macros)
  koji-scanner     read-only golbat data access (→ core)
  koji-db          Koji's own sea-orm entities + queries (→ core)
  koji-dragonite   typed Dragonite v2 API client (→ core)
  koji-jobs        persistent queue runtime + JobHandler trait (→ core, db)
  koji-events      outbox + dispatcher + Subscriber trait + webhook registry (→ core, db)
  koji-migration   sea-orm migrations (standalone)
  koji-service     actix app: v2 + v1 shim + AppState + concrete handlers/subscribers (→ all above)
bins/
  koji-server      → koji-service
  koji-cli         run | enqueue | worker (→ core, algorithms, jobs, db)
```
Acyclic; `koji-core` is the universal sink. Isolation: `koji-jobs` exposes a generic `JobHandler` (concrete clustering handlers live in `koji-service`); `koji-events` owns the `Subscriber` trait + generic webhook subscriber (the Dragonite pusher is wired in `koji-service`) — so neither infra crate depends on `algorithms` or `dragonite`.

Old→new mapping + dropped set: see `refactor-workspace/map.md`.

## 4. Domain model & the `Args` breakdown

`Args` (37 fields, 6 deprecated) → composable config structs in `koji-core`: `AreaInput`, `DataFilter`, `ClusteringConfig`, `RoutingConfig`, `BootstrapConfig`, `S2Config`, `OutputConfig`, `DevConfig`. Deprecated fields dropped (`devices`, `fast`, `generations`, `only_unique`, `route_chunk_size`, `routing_time`).

Algorithm entry points change from long primitive lists to these structs:
```rust
fn cluster(points: &[Point], cfg: &ClusteringConfig, ctx: &FeatureCollection, stats: &mut Stats) -> Vec<Point>;
fn route(points: &[Point], clusters: Vec<Point>, radius: f64, cfg: &RoutingConfig, stats: &mut Stats) -> Vec<Point>;
fn bootstrap(area: &FeatureCollection, cfg: &BootstrapConfig, routing: &RoutingConfig, stats: &mut Stats) -> Vec<Feature>;
```

**One execution core, two HTTP adapters:** both the v2 request DTO and the legacy `Args` (v1 shim) map into the same config structs — so the shim is a second `From` mapper, not duplicated algorithm wiring.

**Separation of concerns:** `koji-core` owns domain enums (`FenceType`, `DataType`, `Category`, `ReturnType`, `GeometryType`, strategy enums); `koji-db` keeps sea-orm's generated enums internal with `From`/`Into` at the boundary; entities stop doubling as API DTOs (explicit response DTOs in `koji-service`).

## 5. Job queue & sync-bridge

Full design in the [job-queue spec](2026-05-28-koji-job-queue-design.md). Summary: a `job` table claimed via `FOR UPDATE SKIP LOCKED` with lease/heartbeat; one CPU-bound worker by default, sync calc out-prioritizes batch; **enqueue+await** bridges the synchronous Dragonite contract (in-proc oneshot + DB-poll fallback, 290s timeout → 504 with the job continuing); **in-flight coalescing + 5-min grace** dedup by content-hash; `koji-cli` can run directly, enqueue, or act as a standalone worker.

## 6. API v2 + v1 shim

`/api/v2`, JSend envelope (matches Dragonite v2 exactly), semantic HTTP codes, `?format=` query (not path segments), typed resources:
```
POST /jobs · GET /jobs/:id · GET /jobs · DELETE /jobs/:id          # calc (+ POST /calc/* sugar)
GET|POST /geofences · GET|PATCH|DELETE /geofences/:id · POST /geofences/:id/publish
…same shape… /routes /projects /properties /tile-servers · GET|PUT /projects/:id/geofences
GET /scanner-data/:type · POST /geo/{s2/cells,s2/coverage,simplify,convert}
GET /meta/algorithms · GET /healthz · GET /config · POST /auth/login|logout · GET /geocode
```
Kills the four sins: GET-that-mutates → `POST /:id/publish` (or event), catch-all `/{return_type}` → `?format=`, raw-vs-envelope → one JSend format, in-body `status_code` → real HTTP codes.

**v1 shim:** a `v1` module in `koji-service` maps every legacy route onto v2 handlers and re-wraps responses in the legacy `{message,status,status_code,data,stats}` envelope. Validated by replaying Dragonite's exact `koji/main.go` calc requests — must stay byte-compatible. Managed `AppState` holds DB pools, `JobQueue`, the event dispatcher, registries, and the Dragonite client.

## 7. Dragonite integration + events + PR #558

**`koji-dragonite`** — typed client over Dragonite `/v2/areas/*` (`GET|POST|PATCH|DELETE`, paginated list), JSend types, `base_url + bearer + user_agent`. Respects the tri-state `V2GeofencePatch` (absent=no-op, null=clear, value=set) and sends **only fields it intends to change**. Maps Koji `SingleVec` route → Dragonite lat-lon route array; Koji `Feature` fence → `ApiGeofence` (both GeoJSON-native, zero-loss). Mutations target an area by stored **`dragonite_area_id`** (linkage column on Koji's area/geofence; pushes gated until linked).

**`koji-events`** — domain events (`area.route_updated`, `area.geofence_updated`) persisted to `event_outbox`; a dispatcher worker claims due rows (`SKIP LOCKED`), delivers to subscribers, exponential-backoff + dead-letter after `max_attempts`. Subscribers: generic `WebhookSubscriber` (HTTP POST + HMAC-`secret`, topic-filtered, from `webhook_subscription`) and `DragoniteSubscriber` (wired in `koji-service`). Single path: both a calc job's `persist=push` and an explicit `POST /geofences/:id/publish` emit events — no inline writes.

**PR #558 readiness** — per-mode fences modeled as separate entities:
```sql
area_fence(id, dragonite_area_id, mode ENUM('base','pokemon','quest','fort'), geofence_id → geofence.id, UNIQUE(dragonite_area_id,mode))
```
A geofence can fill multiple `(area,mode)` slots; per-mode rows override `base`. The Dragonite pusher gathers an area's slots → one PATCH with base + per-mode `geofence` patch fields (gated behind #558 capability/config until those columns land).

## 8. Plugin system

`koji-plugins`: one `AlgorithmPlugin` trait, `SubprocessPlugin` impl now (native impl possible later). Per-plugin `plugin.toml` manifest (`name`, `kind`, `version`, `entry`, explicit `interpreter`, declared+validated `[args]`). Plugins live in `KOJI_PLUGINS_DIR` (default `./plugins`, **out of the source tree**); a `PluginRegistry` scans + indexes at startup and feeds `/api/v2/meta/algorithms`. Versioned JSON stdin/stdout protocol with a real error channel (replaces the ad-hoc `lat,lng` lines); stderr captured to logs; executed via the job worker's `spawn_blocking`.

## 9. RDM removal

Pure deletion: `ScannerType` (RDM/Unown/Hybrid) gone entirely — single path, no auto-detect. Delete `instance.rs` + the `area.rs` controller-write logic (→ `koji-dragonite`); drop `RdmInstance*`, `InstanceParsing::Rdm`, deprecated env fallbacks (`UNOWN_DB_URL`/`UNOWN_DB`/`DATABASE_URL`); collapse the 14 `ScannerType` branches. `KojiDb` 3 connections → **2** (`koji`, `scanner`); the `controller` connection is replaced by the Dragonite HTTP client.

## 10. Phasing

Eight phases, ~25 commits, full detail in `refactor-workspace/roadmap.md`. Invariant: every commit compiles + passes tests. Hard rule: events + Dragonite client (P3–P5) land **before** RDM/controller-write removal (P6) — never delete the only write path early.

```
P0 workspace skeleton · P1 split model + break Args · P2 algorithms take structs · P3 jobs/events/dragonite crates ·
P4 API v2 + v1 shim · P5 events→Dragonite replaces writes · P6 RDM removal · P7 CLI + plugins + quality + PR
```

## 11. Client touch-points (deferred — phase 1 does not touch the client)

The v1 shim keeps all current client calls working. Future client migration (separate effort): `/api/v1`→`/api/v2`, adopt JSend, `GET …/push/{id}`→`POST …/:id/publish`, path `…/{return_type}`→`?format=`, sync calc→`POST /jobs` + poll, and new UI for job status/history, webhook-subscription CRUD, `dragonite_area_id` linkage, and per-mode (`area_fence`) fence assignment.

## 12. Open / deferred
- Per-subsystem deep-dive specs authored as each phase begins (job-queue done; events, v1-shim compat, and the model-split are the next candidates).
- Client migration is out of scope for this branch.
- `or-tools`/VRP solver and the ORM choice (sea-orm) are unchanged.
