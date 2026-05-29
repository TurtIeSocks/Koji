# Koji V2 — Phasing / Commit Roadmap

> **▶ RESUME HERE (2026-05-29, P5 done + runtime-verified).** Branch `claude/goofy-tu-5372ed`; `cargo build` (whole workspace; package is **`koji`**, NOT `koji-server` — `cargo build`/`-p koji`/`--bin koji`) green.
> **Done + committed (P0→P5, all green):** … P2-configs (`c138d0b`) · P3 (`fb25e8e`→`a42bf18`) · **P4 (`a4d7041`,`1a6f80d`)** · **migration driver fix (`0b6abcf`)** — restored `sqlx-mysql`+`runtime-actix-native-tls` on sea-orm-migration so the CLI connects · **P5a (`b4f9f83`)** — reconciled koji-dragonite vs the REAL `/v2/areas` contract (V2Response `ok/error` envelope, full `ApiArea`+per-mode blocks, geofence = single tri-state `Feature` per slot, `?page/per_page` meta pagination; 27 tests) · **P5b+P5c (`d69dbe8`)** — `DragoniteSubscriber` + dispatcher spawned + `POST /geofences/:id/publish` emits `area.geofence_updated` (linkage-gated 422) + geofence entity gained `dragonite_area_id`.
> **RUNTIME-VERIFIED against live dev DBs (dev_koji/dev_golbat/dev_drago @ 192.168.1.254) + a mock Dragonite:** migrations applied; v1+v2 calc (cluster/route/bootstrap) over real golbat data (1759 stops→clusters, stats match Dragonite contract); v2 jobs async+sync-bridge+dedup; v2 CRUD lifecycle; **P5 E2E:** publish→event_outbox(`delivered`)→dispatcher→`PATCH /v2/areas/1` (Bearer+UA+`{geofence:Feature}`). Tooling note: mysql 9.x CLI can't auth (native_password gone) — use the `/tmp/kojienv` pymysql venv or sqlx.
> **P6 ✅ DONE + runtime-verified (`5eb2501` api, `47af2c8` koji-db; spec `d103976`).** `ScannerType`/RDM/Hybrid + the controller connection removed; `KojiDb` 3→2 conns (`koji`,`scanner`); RDM entities (`db/instance`,`db/area`,`text.rs` RdmInstance, `normalize.rs`) + the v1 controller-write handlers (`push_to_prod` ×3, `private/instance.rs`, `/api/v1/project`) deleted. Unown-vs-RDM output naming collapsed to the **Unown arm** (RDM `…Smart…` types dropped). **Verified:** server boots with `CONTROLLER_DB_URL` unset (no panic); v1+v2 calc, v2 CRUD, and the P5 publish→PATCH E2E all still pass.
> **P7 ✅ SUBSTANTIALLY COMPLETE.** ✅ rename `api`→`koji-service` (`04a9af0`) · ✅ **koji-cli** (`08831b5`) · ✅ **koji-plugins** extraction + formalization (`ae57081`+`04fe12c`+`4df9a72`; manifest/JSON-protocol/registry/`KOJI_PLUGINS_DIR`, `create_cell_map`→koji-core, acyclic) · ✅ **OpenAPI 3.1** at `GET /api/v2/openapi.yaml` (`829229d`) · ✅ clippy deny-fix (`5923957`) + workspace `--fix` (`8679bb6`,`ea9efdd`) · ✅ **rustfmt --all** (`551a3ab`) · ✅ **React client `scanner_type` collapse** (`cf1f0ff`). `cargo test --workspace` + `cargo clippy` (deny-level 0) green; final runtime smoke on the latest binary green (healthz/openapi/meta/v1-calc/v2-calc/publish→PATCH).
> **Deferred follow-ups (documented, not blockers):** v1-calc re-point (intentionally NOT done — v1 deprecated + the v2 `CalculateHandler` covers only bootstrap+cluster+route, not v1's reroute/route-stats/area; re-pointing = handler expansion + 6-handler rewrite + regression risk for a dying surface) · `area.route_updated` producer (subscriber arm + mapping exist, unit-tested; needs a koji-mode→`AreaMode` decision) · calc `save_to_db`/`save_to_scanner` side-effects (deferred since P4) · residual manual clippy-pedantic warnings (large-variant boxing / manual trait impls — low-value churn).
> **▶ PR is READY pending the maintainer's go-ahead** (CLAUDE.md gates PR-opening behind a heads-up). Branch `claude/goofy-tu-5372ed`; PR body drafted in `refactor-workspace/PR-BODY.md`.
> **P5 follow-up (carry into P6/P7):** the `area.route_updated` arm (subscriber + payload + `area_route_patch`) is wired + unit-tested but has **no producer** — a route publish endpoint / calc persist=push needs the koji-route-mode→`AreaMode` mapping decision. Calc save_to_db/save_to_scanner side-effects still deferred.
> **P2 scope note:** config-struct **adoption** done (`c138d0b`) — clustering/routing/bootstrap take the koji-core configs; `calculate.rs` builds them at the call sites (flat `ArgsUnwrapped` fields kept; their removal is a trivial later cleanup). `koji-plugins` extraction **moved to P7** (the plugin system is reworked there — manifest/JSON protocol/registry — so the move + formalization land together, avoiding move-only churn + the `create_cell_map` cross-crate cycle now). Calc-path behavioral parity to be confirmed by the maintainer's `benchmark_mode` A/B (algorithm internals untouched; only param plumbing changed).
> **Context:** architecture + job-queue specs in `docs/superpowers/specs/`; per-phase plans in `docs/superpowers/plans/`. Pre-existing debt for the P7 sweep: `greedy.rs:798,817` clippy `unused_io_amount` (deny-level) + repo-wide rustfmt (project isn't fmt-enforced).

One long-lived branch, many commits. **Invariant: every commit compiles + passes existing tests.** Hard ordering rule: events + Dragonite client land **before** RDM/controller-write removal (never delete the only write path early).

**Conventions threaded through all phases** (architecture spec §2):
- **Deps → latest + workspace-managed.** When a phase touches a crate, bump its deps to latest (breaking OK, fix in-phase) and move them to root `[workspace.dependencies]` (`dep = { workspace = true }`). P0 scaffolds the `[workspace.dependencies]` table.
- **Macros first-class.** Reach for `koji-macros` to DRY repeated patterns (typed CRUD handlers, entity↔domain mappings, v1-shim adapters) once a pattern recurs 3+ times.

## Phase 0 — Workspace skeleton (mechanical, zero behavior change) ✅ DONE
- `refactor(workspace): relocate cargo workspace to repo root (crates/ + bins/)`
- `chore(build): update Dockerfile + ignores + vscode for root workspace layout`
> Done: build + tests green, zero behavior change. Deviation: no `koji-*` package renames — relocate only; renames fold into the phases that rewrite each crate.

## Phase 1 — Split the `model` nightmare (separation of concerns)
Delivered as focused, independently-green sub-plans (each reviewed before execution):
- **P1a** ✅ DONE — `koji-core` + `#[derive(StrEnum)]` + `enum_bridge!`; `FenceType`/`Category` + relocated strategy enums; conversion traits decoupled from sea-orm `Type`.
- **P1b** ✅ DONE (`0e555b3`) — moved geometry + conversion layer into `koji-core`; migrated all consumers to `koji_core` directly (no facade); `PointStruct` pure (db `LatLonRow` carve-out); `text.rs` split (RDM stays). Deviation: `GeoFormats` stays in `model::api` until P1d (its `Bound` variant references `args::BoundsArg`).
- **P1-conv** ✅ DONE (`a388df2`) — conversion-layer modernization: `FeatureCtx` bundles the param-carrying `(name, fence_type)` args; `wrapper_conversions!` macro collapses the identical wrappers across the single-feature types (`SingleVec` keeps its emptiness-guarded `to_collection` via the `@wrappers` arm); `From<PointArray> for PointStruct` (the one orphan-rule-allowed `From` win). Geometry types stay aliases → no algorithm hot-path churn. Orphan rule keeps the `Vec`-alias conversions on `To*` traits.
- **P1c** ✅ DONE (`bedc56e`→`76f1c64`) — extracted `koji-db` (Koji entities + db infra + enum_bridge + RDM types + `KojiDb`/bootstrap + `ModelError`) and `koji-scanner` (golbat read-only; returns `sea_orm::DbErr`, no custom error; owns `Total`). Broke the model↔koji-db cycle by moving `GeoFormats` + query/admin arg structs + pure utils + domain-enum mappers into `koji-core`; `TextHelpers`+RDM normalization → koji-db. `model` shrunk to `api::args` (→ koji-core only); acyclic graph. Deviation: `entity=DTO` decoupling (explicit response DTOs) deferred to P4/koji-service; koji-db mirrors model's old internal layout to minimize entity churn. Spec/plan: `docs/superpowers/{specs,plans}/2026-05-29-koji-v2-p1c-*`.
- **P1d** ✅ DONE (`c7618e7`, `fddc9fb`) — defined the 8 config structs (`S2Config` + Clustering/Routing/Bootstrap/Area/DataFilter/Output/Dev) in `koji-core::config` + moved `ReturnTypeArg` to koji-core; dropped the 6 deprecated `Args` fields (devices/fast/generations/only_unique/route_chunk_size/routing_time). Scope: structs **defined**, not yet **adopted** — `ArgsUnwrapped`→configs→algorithm-signatures adoption + flat-field removal fold into P2 (with `benchmark_mode` A/B as the behavioral net, since the unit tests don't cover the calc-handler path). `model` stays thin (→ koji-core only). Spec/plan: `docs/superpowers/{specs,plans}/2026-05-29-koji-v2-p1d-*`.
> Riskiest phase. No transitional facades — consumers migrate as types move.

## Phase 2 — Algorithms take structs
- ✅ DONE (`c138d0b`) `refactor(algorithms): cluster/route/bootstrap accept config structs (was 15/7/10 primitives)` — `clustering::main(&ClusteringConfig)`, `routing::main(radius, &RoutingConfig)`, `bootstrap::main(&BootstrapConfig, &RoutingConfig)` + bootstrap `sort(&RoutingConfig)`; `calculate.rs` builds configs at call sites. `bypass_adaptive_partition` stays a separate transient param. Flat `ArgsUnwrapped` fields kept (removal = trivial later cleanup).
- ↪ MOVED TO P7 `refactor(plugins): extract koji-plugins` — deferred so the move lands with the plugin formalization (P7), avoiding the `create_cell_map`/`stringify_points` cross-crate cycle as move-only churn now.
> Perf parity: maintainer runs `benchmark_mode` + `bypass_adaptive_partition` A/B (not runnable in the dev sandbox — needs a real DB). Algorithm internals unchanged, so parity risk is confined to the param plumbing (cross-checked 1:1).

## Phase 3 — New infra crates (additive, not yet wired) ✅ DONE
- ✅ (`fb25e8e`,`3d8eb6f`) `koji-jobs` — job table migration + queue API + worker pool + `JobHandler` + claim/lease/dedup/await (job-queue spec §4/§5/§6/§10). 11 DB-free unit tests.
- ✅ (`6f86696`,`e7a114d`) `koji-events` — event_outbox + webhook_subscription migrations + outbox dispatcher (claim/backoff/dead-letter) + `Subscriber`/`WebhookSubscriber` (HMAC). Design: P3b spec. 9 unit tests (HMAC vs RFC-4231 vector).
- ✅ (`ec1e9d2`,`a42bf18`) `koji-dragonite` + `area_fence`/`geofence.dragonite_area_id` migration — reqwest client plumbing + route mapping + tri-state patch. Wire types were PROVISIONAL; **now RECONCILED vs the real `/v2/areas` contract in P5a (`b4f9f83`)** — all `TODO(dragonite-reconcile)` cleared (V2Response envelope, full `ApiArea`, per-slot `Feature` geofence). 27 unit tests.
> All additive + unwired; build green. **RUNTIME-UNVERIFIED** (no DB / no live Dragonite in the sandbox): the queue claim/await/worker loop, the event dispatch/backoff loop, all HTTP. Maintainer integration-tests against MySQL + Dragonite (specs §12). Migrations run once; raw MySQL DDL.
> ⛔ **P4 IS A STOP-AND-CONFIRM GATE.** The `/api/v1` shim must be **byte-compatible with LIVE Dragonite** (`koji/main.go` calc requests) — and that contract isn't in this repo. Do not blind-build the v1 shim. The v2 surface + koji-service skeleton + sync-bridge wiring are buildable, but the v1-shim byte-compat needs the maintainer's Dragonite request/response samples + replay tests (architecture §6, job-queue spec §12). See memory `koji-v2-autonomous-delegate`.

## Phase 4 — API v2 ✅ DONE (core + CRUD) · v1-repoint + rename → P7
- ✅ (`a4d7041`) AppState (db + JobQueue + workers + EventDispatcher + optional DragoniteClient) + JSend envelope + `CalculateHandler` (calc-as-job) + v2 `/jobs` (submit/poll/cancel) + `/calc/*` sync-bridge (enqueue_or_attach HIGH + await_result 290s→504) + `/meta/algorithms` + `/healthz`.
- ✅ (`1a6f80d`) v2 typed CRUD resources — geofences/routes/projects/properties/tile-servers (list/create/get/update/delete, JSend, `?format=` on geometry resources, `crud_resource!` macro for the plain 3); `POST /geofences/:id/publish` stubbed 202 (event emission = P5).
- Design: `docs/superpowers/specs/2026-05-29-koji-v2-p4-api-v2-design.md`. Maintainer steer: **clean v2, v1 byte-compat dropped** (Args changed → not v1.1). 54 unit tests green.
- ↪ Deferred: **v1-calc re-point through the sync bridge** + **rename `api`→`koji-service`** → fold into P7. `/api/v1` currently byte-unchanged (best-effort legacy).
> ⚠️ **Whole HTTP+queue path is RUNTIME-UNVERIFIED** (no DB in sandbox). Maintainer integration-tests v2 calc/jobs/CRUD against MySQL before relying on it.

## Phase 5 — Replace controller writes with events → Dragonite ✅ DONE (core + verified)
- ✅ (`b4f9f83`) `feat(dragonite)!: reconcile wire types vs real /v2/areas` — V2Response envelope (`ok/error`+`meta`), full `ApiArea`/per-mode blocks, geofence = tri-state `Feature` per slot, `Tri` Deserialize, `?page/per_page` pagination, per-`AreaMode` PATCH builders. 27 tests.
- ✅ (`d69dbe8`) `feat(events): DragoniteSubscriber + dispatcher spawned + geofence publish emits events` — `POST /api/v2/geofences/:id/publish` → `area.geofence_updated` (base fence) via `EventDispatcher::publish`; linkage-gated (unlinked → 422); geofence entity gains `dragonite_area_id`. Subscriber maps payload → `area_geofence_patch`/`area_route_patch` → `PATCH /v2/areas/{id}`. E2E-verified vs mock Dragonite (outbox→`delivered`).
- ↪ **Follow-up (→P6/P7):** `area.route_updated` producer (route publish endpoint / calc persist=push) — needs koji-mode→`AreaMode` mapping. Per-mode fence push via `area_fence` table (no entity yet) gated on #558.

## Phase 6 — RDM removal ✅ DONE (write path replaced by P5)
- ✅ (`5eb2501`) `refactor(api)!: drop controller writes + ScannerType branches` — `load_feature` koji-only (no controller fallback); deleted v1 `push_to_prod` (geofence/route/project) + `save_scanner` + `private/instance.rs` + `utils/{request,error}.rs`; collapsed Unown output-naming to the surviving arm; dropped `CalcPayload.scanner_is_unown` + `ConfigResponse.scanner_type`.
- ✅ (`47af2c8`) `refactor(db)!: KojiDb 3→2 conns; drop ScannerType + RDM/controller entities` — `get_database_struct` keeps only KOJI/SCANNER urls (no controller conn, no scanner-type probe, no `DATABASE_URL`/`UNOWN_DB*` fallbacks); deleted `db/{instance,area}.rs`, `text.rs`, `utils/normalize.rs`; removed `RdmInstance*`/`InstanceParsing`.
- Design: `docs/superpowers/specs/2026-05-29-koji-v2-p6-rdm-removal-design.md` (`d103976`). Runtime-verified: boots with `CONTROLLER_DB_URL` unset; calc/CRUD/v1/P5-publish all green. External Dragonite/golbat tables untouched (Koji stopped connecting; issued no DROP).
- ↪ **Client (React) follow-up → P7:** ~10 `client/src` files still read `res.scanner_type` / branch on `'unown'`; collapse to the single path.

## Phase 7 — CLI, plugin formalization, polish (PARTIAL)
- ✅ (`08831b5`) `feat(cli): koji-cli (worker | enqueue | get)` over the job queue — `worker` spawns the pool + drains on ctrl_c; `enqueue --kind --payload [@file] [--wait] [--priority]`; `get --id`. (`run` inline-calc deferred — needs the calc-input resolution exposed from koji-service.)
- ✅ (`5923957`) clippy deny-level (`unused_io_amount`) cleared · ✅ (`04a9af0`) rename `api`→`koji-service` · ✅ (`ea9efdd`) clippy `--fix` on koji-service.
- ✅ `/meta/algorithms` + `/healthz` already shipped in P4.
- ⏳ `feat(plugins): extract koji-plugins + manifest (plugin.toml) + JSON protocol + registry` — also move `create_cell_map`→koji-core + `stringify_points`→koji-plugins to break the algorithms↔plugins cycle. **Design-heavy; not yet started.**
- ⏳ OpenAPI doc (recommend `utoipa`) · clippy-pedantic manual sweep · repo-wide `rustfmt` (isolated final commit) · React client `scanner_type` collapse.
> Final (when the above land): full suite + clippy + fmt; **open PR (maintainer heads-up first).**

## Verification cadence
Per CLAUDE.md: lint + typecheck + tests in **parallel** at each phase boundary; full suite only at boundaries (not per-commit).

## Risk register
| Risk | Mitigation |
|---|---|
| Phase 0 huge mechanical diff | self-contained commit; rely on cargo + green tests |
| Phase 1 coupling-break churn | incremental, temporary adapter, compiler-driven |
| v1 shim breaks live Dragonite | replay-Dragonite-requests integration tests in Phase 4 |
| Algorithm perf regression from struct refactor | benchmark_mode A/B before/after Phase 2 |
| #558 unmerged | per-mode push gated behind capability/config; not a hard dep |

## Assumptions (delegate checkpoint)
1. Every commit compiles + passes existing tests — no broken intermediate states.
2. Ordering fixed: events+dragonite (P3–5) precede RDM removal (P6).
3. v1 shim validated by replaying Dragonite's calc request shapes; byte-compatible legacy envelope.
4. No client edits in any phase here; client migration is a separate later effort (touch-points in map.md).
5. or-tools / VRP vendoring untouched; ORM stays sea-orm.
6. Koji adds its own tables; does NOT drop external Dragonite/golbat tables (stops writing, doesn't DROP).
7. Single branch; PR opened at end (draft throughout optional).
8. Migrations additive within the branch; the only `!`-breaking commit is P6 (Koji-internal env/scanner-type surface).
