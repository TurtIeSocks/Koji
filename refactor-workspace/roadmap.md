# Koji V2 — Phasing / Commit Roadmap

One long-lived branch, many commits. **Invariant: every commit compiles + passes existing tests.** Hard ordering rule: events + Dragonite client land **before** RDM/controller-write removal (never delete the only write path early).

**Conventions threaded through all phases** (architecture spec §2):
- **Deps → latest + workspace-managed.** When a phase touches a crate, bump its deps to latest (breaking OK, fix in-phase) and move them to root `[workspace.dependencies]` (`dep = { workspace = true }`). P0 scaffolds the `[workspace.dependencies]` table.
- **Macros first-class.** Reach for `koji-macros` to DRY repeated patterns (typed CRUD handlers, entity↔domain mappings, v1-shim adapters) once a pattern recurs 3+ times.

## Phase 0 — Workspace skeleton (mechanical, zero behavior change)
- `chore(workspace): add root Cargo.toml, move server/* → crates/ + bins/, rename koji-*`
- `chore(ci): update Dockerfile + .github paths for new layout`
> Verify: full build + test green. Pure move — diff is large, risk low.

## Phase 1 — Split the `model` nightmare (separation of concerns)
- `refactor(core): extract koji-core (geometry, conversions, GeoFormats, BBox)`
- `feat(core): domain enums (FenceType/DataType/ReturnType/…) + From/Into mapping in db`
- `refactor(db): extract koji-db (Koji's own entities); stop exposing entities as DTOs`
- `refactor(scanner): extract koji-scanner (golbat read-only)`
- `refactor(core): break Args → config structs (Clustering/Routing/Bootstrap/S2/Output/AreaInput); drop 6 deprecated fields`
> Riskiest phase — lean on the compiler; keep a temporary Args→config adapter so API still builds.

## Phase 2 — Algorithms take structs
- `refactor(algorithms): cluster/route/bootstrap accept config structs (was 15 primitives)`
- `refactor(plugins): extract koji-plugins (move only; protocol unchanged for now)`
> Verify perf parity via existing `benchmark_mode` + `bypass_adaptive_partition` A/B toggle.

## Phase 3 — New infra crates (additive, not yet wired)
- `feat(jobs): koji-jobs — job table migration, queue API, worker pool, JobHandler`
- `feat(events): koji-events — outbox + webhook_subscription migrations, dispatcher, WebhookSubscriber`
- `feat(dragonite): koji-dragonite — typed /v2/areas client + JSend types`
- `feat(migration): area_fence + area.dragonite_area_id linkage`

## Phase 4 — API v2 + v1 shim
- `feat(service): managed AppState + koji-service skeleton + JSend envelope`
- `feat(api): v2 typed resources (geofences/routes/projects/properties/tile-servers) + scanner-data + geo + meta`
- `feat(api): v2 jobs endpoints (POST /jobs + /calc/* sugar) async submit+poll`
- `feat(api): sync calc bridge (enqueue+await) for the Dragonite contract`
- `feat(api): v1 shim — map legacy routes onto v2 + legacy envelope`
> Verify: integration tests replaying Dragonite's exact `koji/main.go` calc requests against the v1 shim. **Dragonite is live — this must be byte-compatible.**

## Phase 5 — Replace controller writes with events → Dragonite
- `feat(events): DragoniteSubscriber; calc-persist + publish emit events instead of direct writes`
- `feat(dragonite): per-mode fence push via area_fence (gated on #558 capability)`

## Phase 6 — RDM removal (now safe — write path replaced)
- `feat(scanner)!: drop ScannerType/RDM/Hybrid, instance.rs, controller-write paths; KojiDb 3→2 conns`
> Only destructive-to-Koji-code phase; external Dragonite/golbat tables untouched.

## Phase 7 — CLI, plugin formalization, polish
- `feat(cli): koji-cli (run | enqueue --wait | worker)`
- `feat(plugins): manifest (plugin.toml) + JSON protocol + external dir + registry`
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
