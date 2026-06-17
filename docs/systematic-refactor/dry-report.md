# Koji DRY Scan — duplication map — 2026-06-14

Companion to `relocation-map.md` (move-to-use refactor, done). This pass = **duplication**:
same logic/structure written more than once → candidate for a shared abstraction.

**Method.** Two lenses, union of findings:
- **Mechanical** — `jscpd` (Rust `cpd` port, `--format rust --min-tokens 60 --min-lines 8 --mode strict`).
  Token-level copy-paste. Whole workspace. Result: **135 clones · 1690 dup-lines · 5.2%**.
- **Semantic** — 4 `cavecrew-investigator` agents over crate clusters. Catches same-logic /
  parallel-structure dup that jscpd misses (different identifiers, divergent text, same shape).

**jscpd per-crate (clones / dup-lines):** koji-service 49/674 · koji-db 30/431 · migration 24/254 ·
koji-golbat 17/218 · algorithms 9/121 · nominatim 4/106 · koji-core 4/89 · events 1/12 · jobs 1/12.

> ⚠️ **Test-vs-prod split matters.** jscpd counts `#[cfg(test)]` clones as equal to prod. Verified:
> koji-core's 89 dup-lines + the largest cross-crate clone (35+23+22L, `koji_output.rs` ↔
> `utils/response.rs`) are the **`sample_rich()` test fixture**, duplicated on purpose. Re-ranked
> below as *test-support*, not prod logic.

---

## Tier 1 — high-value production dup (jscpd + agent agree)

### T1a · koji-service handlers — response/dispatch/error boilerplate
Biggest prod hotspot. jscpd #1 crate. Agent-C top-3.
| dup | sites | ~LOC | locations |
|---|---|---|---|
| `Ok(HttpResponse::Ok().json(Response{data:Some(..),message:"Success",status:"ok",..}))` | 18 | 7 ea (~126) | admin.rs ×9, geofence.rs ×5, instance.rs ×3, misc ×1 |
| `match resource.to_lowercase() { "geofence"=>db::geofence::Query::M(..), .. }` | 12 | 3 ea (~36) | admin.rs, points.rs |
| `.map_err(actix_web::error::ErrorInternalServerError)?` / `ErrorBadRequest(e.to_string())` | 26+ | 1 ea | admin/geofence/instance/points/v2 |
| v1↔v2 endpoint dup (`public/v1/s2.rs` ≈ `public/v2/geo.rs`) | 4 blocks | ~50 | s2.rs:22-113 ~ geo.rs:92-160 |
| `geofences.rs` ≈ `routes.rs` (same CRUD skeleton) | 5 blocks | ~70 | geofences.rs ~ routes.rs |
| jobs.rs / plugins.rs self-dup | 5 | ~60 | jobs.rs:60-121/303, plugins.rs:98-167 |

**Abstractions:** `ok_response(data)` + `ok_response_with(data,stats)` helper (extract-fn) · resource-dispatch
macro or fn-ptr table · `KojiError`→`actix Error` via `ResponseError`/`From` impl (kills the 26 `map_err`) ·
fold v1 s2 endpoints onto v2 (or shared inner fn).

### T1b · koji-db + koji-golbat — per-entity Query boilerplate
jscpd: project.rs↔tile_server.rs 30+25L, geofence↔project 28L, gym↔pokestop 24+24L. Agent-A top-3.
| dup | sites | ~LOC | locations |
|---|---|---|---|
| Fort geo-query chain `all/bound/area/stats` (table+prefix vary) | 3 entities | ~73 | golbat/entities gym.rs:53-136, pokestop.rs:61-142, spawnpoint.rs:30-123 |
| CRUD `get_one/get_one_json/delete/search` | 3 entities | ~23 | db/property.rs, project.rs, tile_server.rs |
| `upsert_related_by_geofence_id` ≈ `upsert_related_by_project_id` (mirror) | 2 | 39 ea (~78) | db/geofence_project.rs:174,214 |
| normalize `fort/fort_filtered/spawnpoint/spawnpoint_filtered` (enumerate+prefix) | 4 | ~59 | golbat/normalize.rs:71-141 |
| raw-SQL `SELECT lat,lon FROM {t} WHERE enabled=1 AND deleted=0 AND updated>={} AND ({})` | 2+ | — | gym.rs:109, pokestop.rs:114 |

**Abstractions:** generic `EntityQuery` trait w/ default `get_one/delete/search` (entity-param) ·
`query_geo_points(table, filters, normalizer, prefix)` extract-fn · generic
`upsert_related_by<K>(db, items, id, key_fn, insert_fn)` · `normalize_enumerated!` macro.

### T1c · nominatim — endpoint request/parse dup
jscpd: lookup↔search 57L, lookup↔reverse 22L. Agent-D #2. Self-contained crate, clean win.
| dup | sites | ~LOC | locations |
|---|---|---|---|
| build-url → `client.get` → status-check → `text()` → `from_str` | 3 | ~44 | search.rs:125, reverse.rs:101, lookup.rs:94 |
| Query-struct + `serde_utils::serialize_*_as_string` stack | 3 | ~57 | lookup.rs:23-79 ~ search.rs:54-110 |

**Abstraction:** `Client::exec_endpoint<Q,R>(path, query) -> Result<R>` generic + serde derive-helper.

---

## Tier 2 — medium

### T2a · algorithms mode-enum Serde/PartialEq/Display (just-distributed configs)
Agent-B #1. ~81 LOC across the 3 mode files I split in the relocation pass.
`cluster_mode.rs`, `calc_mode.rs`, `sort_by.rs` each hand-roll: custom `Deserialize` (`.to_lowercase()`
match + `Custom` fallthrough), `Serialize`, `matches!`-style `PartialEq`, `Display`.
**Abstraction:** one **derive-macro** (or decl-macro) generating all four from a variant↔string table.
*(Lands cleanly atop the relocation — these are now siblings under algorithms.)*

### T2b · events ↔ jobs — heartbeat + claim-loop  ⚠️ RISK
Agent-D. jscpd undercounts (1 clone ea, 12L) — text diverged, **shape identical**.
- `spawn_heartbeat()`: skip-first-tick → `select(stop ⊕ interval.tick())` → lease-renew UPDATE. dispatcher.rs:403 ≈ worker.rs:279 (~30 ea).
- claim-loop: `FOR UPDATE SKIP LOCKED` claim → heartbeat → execute → persist outcome/backoff. dispatcher.rs:182 ≈ worker.rs:145 (~120 ea).

**Abstraction:** generic `spawn_heartbeat<..>()` + a `ClaimLoop` trait (`claim/execute/persist`).
**⚠️ Concurrency/lease/locking correctness — semantics must be byte-identical after extract. High-care, separate commit, extra test.** Recommend its own gated step, not bundled.

### T2c · koji-core coord/bbox swaps (correctness, not LOC)
Agent-B. ~15 LOC but bug-prone: `r.min().y`/`r.min().x` (geo-types x=lon,y=lat) repeated with
inconsistent swaps across `koji_bbox.rs` + `koji_geometry.rs:118`.
**Abstraction:** sealed trait method on `Rect` (`.lat_min()/.lon_min()/..`) — removes the swap-footgun.

---

## Tier 3 — small / cosmetic
- `crucible/refine.rs:1516 ≈ 1617` (21L internal self-dup) — local extract-fn.
- koji-cli header setup `if !cookie.is_empty(){req=req.header(..)}` ×5 (koji-cli.rs:82-234) — extract-fn.
- repeated `#[derive(Debug,Clone,..,Serialize,Deserialize)]` stacks (4+) — cosmetic, skip unless a shared alias is wanted.
- hand `impl Default` field-by-field (poracle/point_struct) — minor.

## Test-support bucket (not prod)
- **`sample_rich()` fixture** duplicated koji-core `koji_output.rs:472` ↔ koji-service `response.rs:116` (~80L, the big cross-crate clone). Fix: shared test-support ctor (e.g. `#[cfg(any(test,feature="test-fixtures"))] pub fn sample_rich()` in koji-core, re-used by service). Low-risk, test-only.
- `serde_json::from_value(json!(..)).unwrap()` + asserts ×3 in `query_args.rs` tests — inline helper/macro.

## SKIP / DEFER (flagged, not recommended)
- **migration/ create-table scaffold (24 clones / 254 dup-lines).** sea-orm migrations are an
  **append-only historical ledger** — each is a frozen record of an applied change. Refactoring
  *applied* migrations risks altering replay semantics for no runtime benefit; the "dup" is inherent
  to the pattern. **Recommend SKIP** (at most: a helper template for *future* migrations only).

---

## Recommended cut-line
**Do T1 (a/b/c) + T2a + T2c.** Highest real LOC + clarity, low risk, lands clean on the relocation.
**Gate T2b separately** (concurrency risk). **T3 = optional polish. Skip migration. Test bucket = nice-to-have.**
Rough reclaim if T1+T2a+T2c done: **~450–500 prod LOC** of duplication collapsed.
