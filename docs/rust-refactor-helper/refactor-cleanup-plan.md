# Koji Rust Refactor / Cleanup Plan — 2026-07-10

Produced by a 20-agent audit workflow (16 crate shards + 4 cross-cut sweeps) with adversarial
verification, plus a workspace clippy run. Branch `claude/v2` @ `ee75201c`.

> **EXECUTION STATUS (2026-07-10, ~45 commits `831532a1..`):** Executed same-day.
> - **DONE**: all of P0 (0.1–0.19 — every ⏳ item was hand-verified and confirmed real — plus the
>   fastest-determinism fix), the full koji-db P1 purge (route/geofence v1 chains, json-cache
>   surface, AdminReq, to_geofence_project, stale allows), P1 other-crates (Stats::log,
>   cells_to_nearest_face_edges, event_outbox, dragonite envelope, golbat all/bound,
>   lock topics, nominatim write-only fields, genetic_post_processing), all of P2 (2.1–2.12),
>   P3 3.2 (Crucible::run + hoisting), P4 4.1–4.4 (greedy, sec, enqueue_inputs was NOT done — see
>   remaining), P5 jobs waiter-leak + field privacy + dragonite typed HttpStatus + s2 BFS rewrite.
> - **REMAINING** (deliberately deferred): P3 3.1 refine.rs module split + 3.4 stats.rs split
>   (pure code motion), 4.3 enqueue_inputs move-not-clone, 4.5–4.7 perf batch, P5 s2 KojiBbox
>   signatures + root re-export trims + JobStatus lowercase (needs web-client coordination),
>   P6 refine LNS tests + webhook wiremock test + lint policy, the ~85 ▫️ LOW batch,
>   nominatim lookup/reverse delete (product decision), koji-plugins collapse (closed: keep).

**Verification status legend**
- ✅ **CONFIRMED** — an independent adversarial verifier read the code and upheld the finding.
- ⏳ **UNVERIFIED** — found by an auditor; the verifier never ran (spend limit hit mid-run).
  Treat as high-probability but re-check the cited code before acting.
- ▫️ **LOW** — low-severity; unverified by design. Batch material.

**Resume the verification tail** (43 pending verdicts + the never-run `sweep-panics` audit):
`Workflow({scriptPath: ".../koji-refactor-audit-wf_d4bcf451-865.js", resumeFromRunId: "wf_d4bcf451-865"})`
— everything completed replays from cache; only the tail runs (Sonnet).

**Clippy ground truth**: default `cargo clippy --workspace --all-targets` is **clean** (only a
dep-tree future-incompat note for `num-bigint-dig`/`proc-macro-error2`). Pedantic pass: ~1300
warnings, dominated by doc nits; substantive classes are folded into P0/P4/P6 below.

---

## P0 — Correctness & panic paths (do first; all small)

Real bug-risk. Each is an S-effort surgical fix.

### Verified ✅

| # | Where | Problem | Fix |
|---|-------|---------|-----|
| 0.1 | `koji-service/src/lib.rs:480-483` | **Migration failure logged and swallowed** — server boots against un-migrated schema | Propagate error out of `start()`; abort startup |
| 0.2 | `koji-service/src/internal/realtime/mod.rs:132-180` | WS `Some(Ok(msg)) =` select pattern silently disables inbound branch on stream end/error → task + broadcast receiver leak per dead connection | Bind `msg = msg_stream.next()`, `break` on `None`/`Some(Err)` |
| 0.3 | `algorithms/src/clustering/crucible/solve.rs:62`, `crucible/mod.rs:375` | `partial_cmp().unwrap()` panics on NaN — same class refine.rs was already hardened against | `unwrap_or(Ordering::Equal)` / `total_cmp`, match refine.rs |
| 0.4 | `algorithms/src/bootstrap/radius.rs:113-132` | `Polygon::try_from().unwrap()` + `extremes().unwrap()` panic on API-reachable input (Point/LineString features) | Early-return empty Vec, mirror `BootstrapS2::build_polygons` |
| 0.5 | `algorithms/src/utils.rs:50-69` | `rotate_to_best` geohash `encode().unwrap()` ×2, unconditional in `routing::main` — panics on non-finite coords | Warn-and-fallback like `sort_geohash` |
| 0.6 | `koji-service/src/public/v2/geofences.rs:238` +7 more sites (routes.rs, golbat_data.rs) | Blanket `.map_err(\|_\| NotFound)` collapses DB outages into 404s | One `not_found_or_db()` helper mapping `ModelError::Database` → 500 |
| 0.7 | `koji-db/src/db/tile_server.rs:47-53`, `project.rs:105-111` | `paginate` swallows `fetch_page` errors into empty page (and tile_server logs as `[project]`) | Propagate with `?` like every other entity |
| 0.8 | `koji-service/src/lib.rs:561-572, 664-670` | Nominatim client rebuilt with 2× `unwrap()` inside worker-factory closure; PORT parse unwrap | Build once before `HttpServer::new`, clone in; friendly startup errors |

### Unverified ⏳ — re-check code first, then fix (all look severe)

| # | Where | Problem |
|---|-------|---------|
| 0.9 | `koji-db/src/utils/sort.rs:5-11` | `a[sort_by].as_array().unwrap()` in comparator; user-controlled `?sortBy=name.length` passes the `contains("length")` guard → request-thread panic |
| 0.10 | `koji-db/src/name_modifier.rs:67-71` | `apply()` slices by raw byte offsets from wire DTO — `trimstart > len`, usize underflow, mid-multibyte-char all panic (Polish names are the expected domain) |
| 0.11 | `koji-core/src/geometry/geometry.rs:7-42` | `ensure_first_last` uses `&&` not `\|\|` — axis-aligned open rings left unclosed; MySQL `ST_GeomFromGeoJSON` rejects them. Mirror copy in `koji_output.rs:379-388` |
| 0.12 | `koji-events/src/webhook.rs:63-69` | Default reqwest client has **no timeout**; a hanging webhook endpoint stalls the single dispatch loop forever (heartbeat keeps renewing the lease, so never reclaimed) |
| 0.13 | `koji-golbat/src/util.rs:21-51` | `sql_raw_bbox` emits invalid SQL (leading `OR`, or empty string → `AND ()`) when first feature is non-polygon / no polygons — 500s from user-suppliable areas |
| 0.14 | `koji-jobs/src/queue.rs:361-394` + `worker.rs` | **Cancellation of a running job is a no-op**: `cancel()` writes phase=canceling but nothing flips the per-job `CancelToken`; `JobOutcome::Canceled` arm unreachable; doc comment on `cancel()` is false |
| 0.15 | `koji-jobs/src/handler.rs:140-162` | Test helper builds `DatabaseConnection` via `alloc_zeroed + ptr::read` — instant UB. sea-orm 1.1 has `Default` (Disconnected); delete the unsafe block |
| 0.16 | `koji-core/src/geometry/koji_geojson.rs:35-40` | `from_value(props).unwrap_or_default()` nukes ALL feature properties when one typed field fails (e.g. string `id`) — S5c-class silent data loss |
| 0.17 | `koji-core/src/geometry/geometry.rs:66-84` | `simplify` unwraps `try_from` on user geojson → v2 geometry endpoint panic |
| 0.18 | `macros/src/lib.rs:184-197` | `StrEnum` never rejects non-lowercase `#[str("Alpha")]` → silent serialize/deserialize roundtrip break; duplicate literals → dead arms |
| 0.19 | `koji-plugins/src/protocol.rs:54-63` | latlng decode silently skips malformed tokens → plugin diagnostics on stdout become "success with zero points", routing replaces real clusters with it |

Also fold in (from `fastest.rs`, verified ✅ low): `cluster()` returns `HashMap::into_values()` —
output order randomized per process despite "deterministic" doc; sibling map already uses BTreeMap
(same commit!). S fix: BTreeMap + strengthen the test that currently sorts both sides.

---

## P1 — Dead code purge (biggest line-count win, ~1,500-2,500 lines)

The v1→v2 API migration stranded whole call chains. All verified by repo-wide caller greps.

### koji-db (verified ✅ — the motherlode, ~800+ lines)

| Target | Lines | Note |
|--------|-------|------|
| `route.rs`: `to_feature`, `by_geofence`, `by_geofence_feature`, `by_geofence_koji` + tests | ~270 | v1 endpoint gone; v2 uses `as_koji_collection`/`get_one_koji` |
| `route.rs`: `upsert_from_geometry` chain (+ helpers, + unwrap panic at :485) | ~170 | v1 "save from calculate" path gone |
| `route.rs`: `get_all`/`create`/`update` | ~80 | zero callers, `update` panics at :362; found independently by 3 agents |
| `geofence/writes.rs:150-212` + `mod.rs:122-220`: `upsert_from_geometry`/`upsert_koji_item`/`associate_parent`/`build_geofence_upsert_map` + tests | ~200 | v2 writes go through `upsert`/`upsert_json_return`/import |
| `geofence/mod.rs:228-359`: `Model::to_feature` + `FeatureRenderSpec` machinery + goldens | ~130+ | ⚠️ gated on P1-refute note below |
| Five entities' `get_json_cache` + `get_all_no_fences` | ~60 | v1 cache surface, test-only |
| `geofence_project.rs`: `update`/`update_by_id` (delete now); `create`/`get_all`/`delete` (delete or demote) | ~80 | junction rows fully managed by `upsert_related` |
| `project.rs:65-90` `get_one_json_with_related`, `property.rs:96-98` `get_all`, `tile_server.rs:65-67` `get_all`, `plugin_config.rs:85-91` `get_one_json` | ~40 | all uncalled |
| `query_args.rs:259-293`: `AdminReq` + `parse()` | ~35 | ⏳ services build `AdminReqParsed` directly |
| `utils/json.rs`: `to_geofence_project` (trait method + impl + 4 tests) | ~70 | ✅ |
| `db/mod.rs`: `Default for PaginateResults<()>`, `VecToJson` trait + impls | ~25 | ▫️ falls out of the above |

**⚠️ Refuted-scope note:** `geofence/project_view.rs` deletion (by_project/project_as_* chain) was
REFUTED by the verifier — `docs/user-stories/v2-parity-gap-list.md` still tracks the `/parent/`
endpoints it would serve. Decide product-side first: if that parity item is dropped, this plus
`Model::to_feature` (P1 row above) unlock together.

### Other crates

| Target | Status | Lines | Note |
|--------|--------|-------|------|
| `algorithms/src/stats.rs:175-266`: `Stats::log` box-drawing printer + `WIDTH` | ✅ | ~95 | zero callers |
| `algorithms/src/bootstrap/s2.rs:226-262`: `cells_to_nearest_face_edges` + its 5 tests | ✅ | ~75 | call site removed as buggy (bac5176a); kept alive only by own tests |
| `nominatim`: `lookup.rs`, `reverse.rs`, `Response`/`Address` tree, orphaned serde_utils | ⏳ | ~900 | **product decision**: knowingly kept for fork parity 2026-06-16; koji-service only calls `search()`. Dead types carry latent deser bugs (typo'd keys, `house_number: u64`) |
| `koji-events/src/entity/event_outbox.rs` | ⏳ | ~138 | all outbox I/O is raw SQL; module doc claim false |
| `koji-dragonite/src/envelope.rs:51-111`: V2Meta/pagination machinery + exec/exec_data merge | ⏳ | ~80 | only `patch_area` exists; meta always discarded |
| `koji-golbat`: `Query::all`/`bound` (spawnpoint, station, + 2 macro arms in fort_query) | ⏳ | ~80 | v2 collapsed bbox path onto area path |
| `nominatim/src/client.rs:10-16`: write-only `user_agent`/`email` fields | ⏳ | ~10 | email never even sent upstream |
| `algorithms/clustering/config.rs`: `genetic_post_processing` threaded end-to-end, read by nothing | ⏳ | ~15 | strip from wire surface or doc the plan |
| `koji-service` `internal/realtime/topics.rs`: `lock_topic`/`lock_record_topic` | ⏳ | ~10 | scaffolding for feature that never landed; found by 3 agents |
| `geofence/reads.rs:139-141` `get_all_json` | ⏳ | ~5 | |
| `koji-core`: `text_utils::get_mode_acronym`, `s2::from_array_to_cell_id`, `KojiBbox::{from_rect,to_rect}` | ⏳/▫️ | ~50 | RDM-era leftovers; from_rect may gain a caller via P4.6 |

### Stale `#[allow(dead_code)]` purge (✅ verified, found by 3 agents independently)

`utils/format.rs:10`, `utils/pagination.rs:14`, `utils/error.rs:13`, `utils/openapi.rs:12`,
`utils/api_response.rs:85,103` — all "Phase 0 ahead of consumers" rationales are stale; consumers
landed. Remove the allows + stale comments, let rustc surface anything genuinely dead underneath.

---

## P2 — Duplication collapse

| # | Status | What | Fix | Effort |
|---|--------|------|-----|--------|
| 2.1 | ✅ | Third copy of Welzl MEC: `fastest.rs:37-120` ≡ `crucible/geometry.rs:16-105` (crucible's is more robust) | Promote crucible's to clustering-shared; delete fastest's ~70-line copy | S |
| 2.2 | ✅ | `refine.rs` passes copy-paste 4 scaffolding blocks (rtree index, dirty snapshot, neighbor dedupe, move-center commit — the double-kill invariant lives in 2 places, one uncommented) | Extract `Refiner` helpers: `live_center_index()`, `dirty_snapshot()`, `nearest_live_neighbors()`, `move_center()` | M |
| 2.3 | ✅ | Paginate epilogue copy-pasted 6× + sort/LIKE prologue; route.rs hand-rolls exactly what `#[macros::crud_query]` generates | `PaginateResults::from_paginator()` helper; put `crud_query` on route's Query (~40 lines gone) | M |
| 2.4 | ✅ | geofence/route v2 handlers near-verbatim: PATCH merge body, create-tail (with silent id-0 bug), DELETE-404 | Shared `public/v2/crud` helpers: `merge_patch`, `created_response` (surface missing id as 500), `delete_or_404` | M |
| 2.5 | ✅ | `?format=`/`?rt=` negotiation struct ×3 with clone-forced impls | `get_return_type(&str)`, `Copy` on `ReturnTypeArg`, one shared `negotiate_return_type()` | S |
| 2.6 | ✅ | Internal row-list handlers hand-roll pagination math that `utils/pagination.rs` already provides | Use `Pagination::from_parts` + `Meta::build` in both | S |
| 2.7 | ✅ | ~350 of lib.rs's 673 lines are test-app builders, 5× session-middleware block, service lists drifted (test_db_app_with_internal missing 4 scopes) | Extract `test_support` module + `v2_api_services(cfg)` shared with prod `start()` | M |
| 2.8 | ⏳ | `koji-db/utils/json.rs` — eight `to_*` impls repeat the same if-let pyramid (to_webhook = 84-line nested-if god fn); found by 2 agents | `as_obj`/`req_str`/`parse_geometry` helpers, `?`-chained early returns; existing tests cover it | M |
| 2.9 | ⏳ | `koji-golbat/station.rs`: `area`/`stats` byte-identical raw SQL; spawnpoint.rs already models the fix | Private `query_area()` helper (2 agents found this) | S |
| 2.10 | ⏳ | `has_column` guard copy-pasted in 3 migrations, already drifted into 2 signatures | One `pub(crate) async fn has_column(manager, table, col)` | S |
| 2.11 | ▫️ | `to_koji_geometry` JSON-parse two-liner duplicated route↔geofence (doc admits "Mirrors...") | `parse_geometry_json()` in db/mod.rs | S |
| 2.12 | ▫️ | cli/server dotenv+env_logger startup byte-identical; `get_database_struct` connect block ×2 | tiny shared init helpers | S |

---

## P3 — Structure splits (pure code motion)

| # | Status | What | Effort |
|---|--------|------|--------|
| 3.1 | ✅ | `crucible/refine.rs` (2103 lines, ~15 concerns) → `refine/{grid,lns,swap,polish}.rs` + core in mod.rs. Verifier mapped exact seams (grid 40-250, swap 744-1009, lns 1155-1477+1689-1764, polish 1478-1688) | M |
| 3.2 | ✅ | `Crucible::run` ~170 lines AND redoes variant-independent work per variant (owned.clone+gather_halo+Frame::project ×5; internal_score rebuilds id_to_idx map it already has) → extract `build_jobs`/`solve_variant`/`recombine`, hoist invariants. **This one is also a perf fix** | M |
| 3.3 | ▫️ | `macros/src/lib.rs` (1101 lines, 5 independent macros) → one file per macro, thin entry points at root | M |
| 3.4 | ▫️ | `stats.rs` (1011 lines): scoring free-fns + tests → `stats/score.rs` (~halves file) | S |
| 3.5 | ▫️ | `route.rs` 935 lines — **don't split**; P1 deletions land it near 400. Split only if deletions rejected | — |

---

## P4 — Performance (measured against Ch.3 "performance mindset")

| # | Status | What | Fix | Effort |
|---|--------|------|-----|--------|
| 4.1 | ✅ | `greedy.rs:398` clones every surviving candidate's point-vec per size iteration (shallow `Vec<&Point>` but dominant allocation) | Materialize `all` only at insertion | M |
| 4.2 | ✅ | `sec/sec.rs:14-43` clones full point Vec 2× per attempt × up to 100 attempts per cluster | `&[Point]` params, one working buffer, reuse validity | S |
| 4.3 | ✅ | `ops.rs enqueue_inputs(&self)` clones whole FeatureCollections only because serialization happens after | Serialize first, then consuming `into_enqueue_inputs(self)` | S |
| 4.4 | ✅ | `import.rs` runs `find().all()` twice pulling every geometry blob to build a name set + name→id map | One `select_only(Id, Name)` query | S |
| 4.5 | ⏳ | `name_modifier.rs:178` compiles a Regex per row render | `LazyLock<Regex>` | S |
| 4.6 | ⏳ | `geometry_geojson_bbox` deep-clones geometry into throwaway Feature per `simplify`/`trim_precision` call; `remove_internal_props` deep-clones owned Feature | by-ref conversions / consuming self | S |
| 4.7 | ▫️ | Batch: `calc.rs:120` request clone, s2.rs ids clone, import.rs per-item geometry clones, `fastest.rs` absorb-loop O(group²) clone, `&String`→`&str` params (several), jobs worker payload clone, dedup_key per-byte Strings | one sweep PR | M |

---

## P5 — API surface & hygiene

- ⏳ `koji-jobs/queue.rs:70-87` — all five JobQueue fields pub (incl. waiters DashMap, Notify); nothing external touches them → make private. Plus (⏳, high-value): `await_result` leaks waiter senders on 3 of 4 exit paths; `cancel()`/claim-exhaustion skip event sink + waiters (violates JobEventSink contract).
- ⏳ `koji-core/s2`: `get_region_cells` vs `get_cells` take 4 positional coords in DIFFERENT orders — the exact silent-swap hazard KojiBbox was created to kill → take `KojiBbox`. Also 3× duplicated 4-vertex cell extraction; `ToGeo` trait has no external users.
- ⏳ `koji-core/s2:140-231` — `circle_coverage` spawns one OS thread per flood-fill cell (unbounded) around `Arc<Mutex<HashSet>>` that leaks into the public API → single-threaded BFS like `s2_grid`, return plain `Covered` (one caller).
- ⏳ `koji-dragonite`: stringly `http_{status}` error protocol reverse-engineered cross-crate via `strip_prefix` → structured `HttpStatus{status,body}` variant. Crate doc header documents 6 endpoints; client implements 1.
- ▫️ Root re-export trims: koji-golbat (macro-support exports → pub(crate)), koji-dragonite, koji-plugins, koji-service `pub mod requests` glob (~20 wire types leaked; consumers use 4 symbols).
- ▫️ `JobStatus` serde PascalCase on REST vs lowercase realtime — web client papers over it (`use-calc.ts:18`) → `rename_all = "lowercase"`, coordinate with client.
- ▫️ `koji-plugins`: registry's 3 parallel HashMaps → one entry map; `PluginKind` FromStr next to Display (kills service-side `parse_kind` drift); Display for `PluginProtocol` (kills Debug-lowercase hack).
- **koji-plugins collapse question: CLOSED as product decision, not cleanup** — auditor confirmed 0 in-tree plugins but the machinery is a shipped, documented feature with a known external plugin (tsp-mt); collapse would delete v2 API + admin UI + DB table + docs. Record and move on.

## P6 — Test gaps & policy (from best-practices ch.5 + pedantic data)

- ✅ `refine.rs` LNS/swap machinery has zero direct unit tests despite producing a real count-corruption bug historically → 3 focused Refiner tests (split-descent, competing-window commit rejection, count[] integrity).
- ⏳ `koji-events deliver_to` HTTP behavior (signature-of-exact-bytes, ping/event, header filter) only tested manually → wiremock round-trip test; the signing contract is the crate's core API.
- **Pedantic lint policy** (optional, M): 110 float `==` comparisons (geometry dedup is often intentional — needs judgment, not a blanket fix), ~157 lossy casts (mostly usize↔f64 in algorithms; triage the u64→i64 DB ones), 30 wildcard imports, 45 redundant closures. If wanted: add a curated `[workspace.lints.clippy]` set (NOT blanket pedantic; doc-nit classes are noise here). The float-midpoint "overflow" lints (17) are false alarms on lat/lon — ignore.
- Both test-only correctness items: 0.15 (UB in koji-jobs test helper) and fastest.rs determinism-test-hides-bug belong to the first P0 PR.

---

## Suggested execution order

1. **PR 1 — P0 verified panics/bugs** (0.1-0.8 + fastest determinism): ~10 S-fixes, each independently testable. Biggest correctness payoff per line.
2. **PR 2 — P0 unverified**: verify 0.9-0.19 first (resume the workflow tail or check by hand), fix what survives. 0.12 (webhook timeout) and 0.14 (cancel no-op) look most serious.
3. **PR 3 — koji-db dead-code purge** (P1): mechanical deletes, each guarded by `cargo check --workspace` + test run. Decide the project_view/parity question first.
4. **PR 4 — dup collapse in koji-db + service** (2.3-2.7): pairs naturally with PR 3's survivors.
5. **PR 5 — algorithms**: 2.1-2.2, 3.1-3.2, 4.1-4.2 (one crate, one reviewer context).
6. **PR 6+** — remaining P4/P5 batches, nominatim product decision, lint policy.

Ground rules for every PR: pure-motion or delete-only where possible; `cargo clippy --workspace
--all-targets` stays clean; full test suite at PR boundary only (per repo workflow rules).
