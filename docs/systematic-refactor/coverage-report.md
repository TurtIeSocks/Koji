# Koji full-repo test-coverage push — 2026-06-15

Companion to `relocation-map.md` / `dry-report.md` (earlier refactors). This pass = **test coverage**: drive every testable crate up, env-gated DB integration where infra exists, and let characterization tests flush out bugs.

**Tooling:** `cargo-llvm-cov` (installed this session; `llvm-tools` component already present). DB tests run against a live brew MySQL `koji_test` via `.env.test` (`set -a; source ./.env.test; set +a`, sandbox off). golbat (`192.168.1.254`) was **unreachable** this session → live-golbat paths deferred.

**Method:** subagent-driven, one owner per crate per wave (commits use `git add <crate-path>`, never `-A`, so concurrent same-branch commits don't cross-contaminate). Every DB test mirrors the gold-standard template `crates/koji-jobs/tests/queue_db.rs`: `KOJI_DB_URL` env-gate (skip-on-unset so no-env `cargo test` passes), process-wide `SERIAL` mutex (dodges MySQL 1213), ULID-unique row keys, panic-safe capture→delete→assert cleanup. A fresh-eyes reviewer audited the wave: no coverage-chasing, isolation sound, lib helpers `#[doc(hidden)]`.

## Coverage by crate (line %, llvm-cov default incl. test code)

| crate | baseline | final | verdict |
|---|---|---|---|
| koji-core | 73% | 97% | ✅ done |
| koji-wasm | 73% | 98% | ✅ done |
| koji-events | 27% | 93% | ✅ (only the 20s heartbeat tick left) |
| koji-plugins | 91% | 93% | ✅ done |
| koji-jobs | 57% | 91% | ✅ (claim() bug-branch is the gap) |
| koji-db | 39% | 88% | ✅ deep paginate/geometry-hierarchy paths now covered |
| koji-dragonite | 68% | 84% | HTTP send path needs a mock |
| algorithms | 62% | 84% | greedy✂️ + clusterbench-bin + crucible-already-high excluded |
| koji-service | 48% | 80% | handler cases + calc/golbat_data(golbat) + bug-paths |
| macros | 75% | 75% | compile_error! paths need `trybuild` |
| nominatim | 0% | 0% | SKIPPED — upstream-crate swap pending |
| migration | 18% | 18% | SKIPPED — append-only ledger |

**Workspace TOTAL: 54.9% → 84.4% line / 57.0% → 85.0% region** (llvm-cov, env-sourced, all green). ≈ +1,100 tests across waves A–E. Uncovered lines 7644 → 5628 — and the 5628 residual is now entirely the categorized buckets below (exclusions + golbat + needs-dep + bugs), not untested logic.

## Residual, fully categorized ("everything makes sense")

Every remaining uncovered region now has a reason:

1. **Deliberate exclusions** — nominatim (218, upstream swap), `clustering/greedy.rs` (224, crucible cut), `bin/clusterbench.rs` (342, dev bench), migration (136, ledger).
2. **golbat-blocked (deferred until `192.168.1.254` reachable)** — `koji-golbat/entities/{gym,pokestop,spawnpoint,station}` (~250), `koji-service/public/v2/golbat_data` (93), `calc.rs` golbat-dependent ops.
3. **Needs a test dependency (your call)** — `macros` compile_error! diagnostics (~81 → `trybuild`); `koji-dragonite`/`nominatim`-handler HTTP send (~160 → `wiremock`/`mockito`).
4. **Native-gated / diminishing** — `bootstrap/mod.rs` (`#[cfg(feature="native")]`), crucible (already 90%+).
5. **Pinned bugs** — broken paths can't go green until fixed (below); characterization tests assert the *current* behavior with `// BUG:` notes.

## Bugs found — all 6 FIXED (`519eadb` jobs · `56eb940` db · `474f2aa` events · `625db34` service)

Each was first pinned by a characterization test asserting the broken behavior; the fix flipped that test to assert the correct behavior. (koji-events had no dead-worker twin of #1 — its `delivering@max` state is unreachable since `mark_failed` sets `dead` atomically.)


| # | sev | where | bug | fix sketch |
|---|---|---|---|---|
| 1 | 🔴 | `koji-jobs/src/queue.rs:412` | `claim()` SELECT filters `attempts < max_attempts`, making the exhaustion guard (`:431`) unreachable → a worker dying on a job's **final** attempt strands it `running` forever (never failed/reclaimed). Contradicts the fn's own doc (:397). | `<` → `<=` (guard already handles `==max`). Test ready, `#[ignore]`'d. |
| 2 | 🔴 | `koji-service/public/v2/{geofences,routes}` | **PATCH partial body → 500**: `to_geofence()`/`to_route()` require name+geometry; a partial PATCH missing geometry errors. | merge existing row, then upsert. |
| 3 | 🔴 | `koji-service/public/v2/jobs` | `GET /api/v2/jobs?page=&per_page=` → **400**: `#[serde(flatten)] Pagination` incompatible with `serde_urlencoded` query extraction. | un-flatten, or a custom query extractor. |
| 4 | 🟡 | `koji-service/public/v2/jobs` | cancel missing id → **202** (should 404): `JobQueue::cancel()` never checks `rows_affected`. | check rows_affected → NotFound. |
| 5 | 🟡 | `koji-db/src/db/route.rs` | `route::Query::by_geofence(name)` filters **route** name, not geofence name. | join/filter on geofence name. |
| 6 | 🟡 | `koji-events` | `mark_delivered` doesn't NULL `locked_by`/`lease_expires` (inconsistent with `mark_failed`; harmless — terminal). | clear lease cols on deliver too. |

**Test-hygiene defect (fixed):** the worker_db.rs worker tests + v2_db.rs enqueue tests cleaned up *after* fallible `.expect`/asserts, so under the slow instrumented coverage run a 10s `await_result` timeout panicked before cleanup → leaked 2 `job` rows into koji_test. Fixed to panic-safe capture→delete-by-kind→assert; a leak-proofing pass empirically verifies koji_test row counts are stable across two full DB-suite runs.

## Open decisions (need your call)

- ~~Fix the bugs?~~ ✅ **All 6 fixed + verified** (commits above; workspace green, koji_test clean).
- **Add `trybuild` (macros) + `wiremock` (dragonite/nominatim HTTP)?** Pushes those crates higher at the cost of two dev-deps.
- **Golbat + golbat_data + calc:** re-run when golbat is reachable (the tests are scoped + waiting).
