# Koji V2 — Phasing / Commit Roadmap

> **▶ RESUME HERE (paused 2026-05-29).** Branch `claude/goofy-tu-5372ed`; tree clean, `cargo build --workspace` + tests green (16+3 pass, 4+1 ignored).
> **Done + committed (P0→P4, ~30 commits, all green):** … P2-configs (`c138d0b`) · P3 (`fb25e8e`→`a42bf18`: koji-jobs/events/dragonite + migrations) · **P4 (`a4d7041`,`1a6f80d`)** — v2 API: AppState (db+JobQueue+workers+EventDispatcher+optional DragoniteClient) + JSend + `CalculateHandler` (calc-as-job) + `/api/v2` jobs/calc-sync-bridge/CRUD-resources/meta + `/healthz`. 54 unit tests.
> **NEXT — MAINTAINER-GATED.** **P5** (events→Dragonite): wire the deferred calc-persist + `POST /publish` to `events.publish`, add `DragoniteSubscriber`, spawn the dispatcher. Needs (a) the koji-dragonite wire types **reconciled** vs real `/v2/areas`, and (b) the koji-jobs/koji-events runtime **verified against a real DB** (all flagged RUNTIME-UNVERIFIED). **P6** (RDM removal) is **destructive + hard-ordering-gated**: per architecture §10, do NOT remove the controller-write path until P5's event→Dragonite replacement is landed AND verified. **P7**: koji-cli + koji-plugins extraction (the deferred P2 move) + thiserror/clippy/fmt sweep + v1-calc re-point + rename `api`→`koji-service` + OpenAPI + open PR.
> Autonomous run reached the verification frontier here: P1–P4 are structural/additive (build + 54 tests green); P5 needs the external Dragonite contract + DB runtime, P6 is destructive. Resume P5 once the maintainer reconciles Dragonite + integration-tests the P3/P4 stack.
> **P4-core deferred (→P5):** calc persistence side-effects (save_to_db/scanner), event emission, DragoniteClient usage, v1-calc re-point through the bridge. **Runtime-unverified:** the whole HTTP+queue path (no DB in sandbox).
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
- ✅ (`ec1e9d2`,`a42bf18`) `koji-dragonite` + `area_fence`/`geofence.dragonite_area_id` migration — JSend + tri-state `V2GeofencePatch` + reqwest client plumbing + known route mapping. **Wire types PROVISIONAL** (no Dragonite OpenAPI here — every speculative field/endpoint marked `TODO(dragonite-reconcile)`; maintainer must reconcile vs Dragonite's real `/v2/areas` before P5). 15 unit tests.
> All additive + unwired; build green. **RUNTIME-UNVERIFIED** (no DB / no live Dragonite in the sandbox): the queue claim/await/worker loop, the event dispatch/backoff loop, all HTTP. Maintainer integration-tests against MySQL + Dragonite (specs §12). Migrations run once; raw MySQL DDL.
> ⛔ **P4 IS A STOP-AND-CONFIRM GATE.** The `/api/v1` shim must be **byte-compatible with LIVE Dragonite** (`koji/main.go` calc requests) — and that contract isn't in this repo. Do not blind-build the v1 shim. The v2 surface + koji-service skeleton + sync-bridge wiring are buildable, but the v1-shim byte-compat needs the maintainer's Dragonite request/response samples + replay tests (architecture §6, job-queue spec §12). See memory `koji-v2-autonomous-delegate`.

## Phase 4 — API v2 ✅ DONE (core + CRUD) · v1-repoint + rename → P7
- ✅ (`a4d7041`) AppState (db + JobQueue + workers + EventDispatcher + optional DragoniteClient) + JSend envelope + `CalculateHandler` (calc-as-job) + v2 `/jobs` (submit/poll/cancel) + `/calc/*` sync-bridge (enqueue_or_attach HIGH + await_result 290s→504) + `/meta/algorithms` + `/healthz`.
- ✅ (`1a6f80d`) v2 typed CRUD resources — geofences/routes/projects/properties/tile-servers (list/create/get/update/delete, JSend, `?format=` on geometry resources, `crud_resource!` macro for the plain 3); `POST /geofences/:id/publish` stubbed 202 (event emission = P5).
- Design: `docs/superpowers/specs/2026-05-29-koji-v2-p4-api-v2-design.md`. Maintainer steer: **clean v2, v1 byte-compat dropped** (Args changed → not v1.1). 54 unit tests green.
- ↪ Deferred: **v1-calc re-point through the sync bridge** + **rename `api`→`koji-service`** → fold into P7. `/api/v1` currently byte-unchanged (best-effort legacy).
> ⚠️ **Whole HTTP+queue path is RUNTIME-UNVERIFIED** (no DB in sandbox). Maintainer integration-tests v2 calc/jobs/CRUD against MySQL before relying on it.

## Phase 5 — Replace controller writes with events → Dragonite
- `feat(events): DragoniteSubscriber; calc-persist + publish emit events instead of direct writes`
- `feat(dragonite): per-mode fence push via area_fence (gated on #558 capability)`

## Phase 6 — RDM removal (now safe — write path replaced)
- `feat(scanner)!: drop ScannerType/RDM/Hybrid, instance.rs, controller-write paths; KojiDb 3→2 conns`
> Only destructive-to-Koji-code phase; external Dragonite/golbat tables untouched.

## Phase 7 — CLI, plugin formalization, polish
- `feat(cli): koji-cli (run | enqueue --wait | worker)`
- `feat(plugins): extract koji-plugins (moved from P2) + manifest (plugin.toml) + JSON protocol + external dir + registry` — also move `create_cell_map`→koji-core (or koji-plugins) + `stringify_points`→koji-plugins to break the algorithms↔plugins cycle.
- `feat(api): /meta/algorithms + /healthz; OpenAPI doc`
- `chore(quality): thiserror error types, async-pattern + clippy-pedantic sweep, fill test gaps`
> Final: full suite + clippy + fmt; open PR.

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
