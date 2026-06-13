# KojiGeometry Phase 2 — boundary rewire, s2 consolidation, Type→Mode, recursive hierarchy

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `KojiGeometry`/`KojiGeometryCollection` the universal boundary type across Koji — rewiring DB-model output, the API request/response edge, and event emission onto it — while consolidating all s2 primitives into `koji-core::s2`, migrating the `Type` column to `Mode`, replacing the parent-arg zoo with a recursive `?depth=N` hierarchy, and deleting the entire N×M `To*` conversion matrix plus `GeoFormats`/`FeatureCtx`/the mode→shape inference.

**Architecture:** **Boundary-only.** `KojiGeometry` lives at three boundaries — DB Query output, HTTP request/response, event emit — and nowhere else. Algorithms/plugins/wasm/events keep `SingleVec` (`Vec<[lat,lon]>`) as their internal compute currency, converting at their edge. The Phase 1 geojson↔Koji conversions and the Phase 1B `KojiGeometryCollection` outbound adapters become the **only** conversion path; the 77-impl `To*` matrix is deleted once nothing routes through it. s2's scattered bridge consolidates into a single `koji-core::s2` module (koji-core already depends on `s2`). `Type→Mode` is a breaking, atomic column migration.

**Tech Stack:** Rust (edition 2024), `geo`/`geo-types`, `geojson` 0.24, `sea-orm` (MySQL), `s2` 0.0.13, `serde`/`serde_json`, axum.

---

## Locked decisions (from participate-mode design, 2026-06-13)

- **Boundary-only scope.** `SingleVec` stays the internal currency for algorithms/plugins/wasm/events. Do NOT push `KojiGeometry` into compute kernels.
- **s2 → `koji-core::s2` module.** Absorb `koji-core::s2grid` + the bridge primitives from `algorithms/src/s2.rs`; **dedup** `from_array_to_cell_id` (currently identical in both crates). Genuinely algorithm-internal cell usage (BootstrapS2, partition, crucible, rtree, routing, stats) stays in `algorithms` and imports the primitives. koji-service HTTP s2 endpoints + koji-plugins consume from koji-core. No new dependency (koji-core already deps `s2`).
- **Type→Mode: breaking, irreversible-down.** Up-migration backfills `geofence.mode` + `route.mode` via the `from_legacy` 12→4 mapping. Down-migration restores all rows to `'unset'` (the 12→4 collapse is lossy and cannot be exactly inverted — documented, acceptable on this unpushed pre-prod branch). The model field switch lands **atomically** with the column migration.
- **LineString divergence is an intentional improvement.** The Phase 1B adapters correctly emit LineString coords; the old matrix silently dropped them (`log::warn!` + empty vec). The Phase 2 swap is therefore NOT byte-parity for line geometries — it fixes a coordinate-dropping bug. The final parity gate **whitelists** line geometries as a known intentional divergence (do not cripple the adapter to replicate the drop).
- **Recursive `?depth=N` hierarchy.** Replace the name-munge args (`parentstart`/`parentend`/`parentreplace`) with a MySQL `WITH RECURSIVE` descendant query returning a flat `KojiGeometryCollection` + `ancestors[]` in each `KojiMeta`. KEEP the `parent`/`group` Feature properties and the `parent`/`mode`/`geotype` filters (real data, not munging). Routes have no hierarchy (they belong to one geofence via `geofence_id`).
- **Events keep their external payload shapes.** `RouteUpdated.route: SingleVec` and `GeofenceUpdated.geofence: geojson::Feature` are an external Dragonite contract — do NOT change them to raw `KojiGeometry`. The producer holds `KojiGeometry` and serializes to the existing payload shapes at emit (`to_single_vec()` / `From<&KojiGeometry> for Feature`).
- **`geo_type` column KEPT** (fast-filter denorm, derived from the geo variant).
- **Delivery: one comprehensive plan, sectioned by workstream**, each section independently committable, one final gate.

---

## Section ordering & dependencies

Sections leave the workspace green at each commit. Order:

1. **S1 — s2 consolidation** (separable cleanup; lowest coupling; do first).
2. **S2 — DB models emit `KojiGeometry`** (foundation for S3/S7/S8).
3. **S3 — API response swap** (`response.rs` → Phase 1B adapters).
4. **S4 — inbound normalization** (drop `GeoFormats` + bare-array wire forms).
5. **S5 — delete the `To*` matrix + `FeatureCtx` + inference** (now unused; re-home utility ops).
6. **S6 — `Type→Mode` breaking migration** (atomic model + column switch).
7. **S7 — recursive `?depth=N` hierarchy** (drop name-munge args).
8. **S8 — events emit-from-`KojiGeometry`**.
9. **S9 — final verification gate.**

---

## File structure

| File | Responsibility | Action |
|------|----------------|--------|
| `crates/koji-core/src/s2/mod.rs` | consolidated s2 primitives (bridge + coverage + grid) | Create |
| `crates/koji-core/src/s2grid.rs` | folded into `s2/` | Delete |
| `crates/algorithms/src/s2.rs` | reduced to algorithm-internal re-exports or deleted | Modify/Delete |
| `crates/koji-db/src/db/route.rs` | `Model::to_koji_geometry`; Query methods → `KojiGeometry*` | Modify |
| `crates/koji-db/src/db/geofence.rs` | Query methods → `KojiGeometry*`; recursive descendant query | Modify |
| `crates/koji-service/src/utils/response.rs` | `send()` takes `KojiGeometryCollection`, dispatches Phase 1B adapters | Modify |
| `crates/koji-service/src/public/v2/{geofences,routes,geo}.rs` | consume `KojiGeometry*`; `?depth=N`; emit-from-Koji | Modify |
| `crates/koji-service/src/public/v1/s2.rs`, `public/v2/geo.rs` | s2 endpoints call `koji_core::s2` | Modify |
| `crates/model/src/api/args.rs`, `crates/koji-core/src/query_args.rs` | drop `GeoFormats`/name-munge args; `area`→Koji; `?depth` | Modify |
| `crates/koji-core/src/geometry/*.rs` | delete `To*` matrix + `wrapper_conversions!`; keep type aliases + utility ops | Modify/Delete |
| `crates/koji-core/src/{geo_formats.rs,feature_ctx.rs,enum_map.rs,fence_type.rs}` | deleted | Delete |
| `crates/migration/src/mXXXXXXXX_type_to_mode.rs` | `Type→Mode` column migration | Create |
| `crates/koji-db/src/db/sea_orm_active_enums.rs` | drop `Type`/`FenceType`/`FenceMode`/`RouteMode`; keep `Mode` | Modify |

**Reference:** spec `docs/superpowers/specs/2026-06-13-koji-geometry-universal-type-design.md`; Phase 1 plan + Phase 1B plan (same dir). The surface maps below each task name the exact call sites discovered during planning.

---

# Section 1 — s2 consolidation into `koji-core::s2`

**Goal:** one home for all s2↔geo primitives; dedup the doubled `from_array_to_cell_id`; algorithms/service/plugins consume from koji-core. Behavior-preserving move — the existing s2 tests + algorithm tests are the oracle.

**Primitives that MOVE to `koji-core::s2`** (from `algorithms/src/s2.rs` unless noted): `Covered`, `S2Response`, `Dir`, trait `ToGeo` (for `CellID`), trait `ToPointArray` (for `CellID`), `get_region_cells`, `get_cells`, `get_polygon`, `get_client_polygon`, `get_polygons`, `circle_coverage`, `check_neighbors`, `s2_grid`, `cell_intersects_polygon`, `cell_coverage`, `from_array_to_cell_id` (single deduped copy), `from_cell_id_to_array`, plus `create_cell_map` (already in `koji-core::s2grid`).

**STAYS in `algorithms`** (imports primitives from `koji_core::s2`): `BootstrapS2` + `block_center_cell`/`cell_center_latlng`/`cells_to_nearest_face_edges` (`bootstrap/s2.rs`), `cluster_s2_cells` (`clustering/s2.rs`), `PartitionedCluster`/`cell_bbox_lat_lon`/`contains_latlng` (`partition.rs`), crucible `dedupe`/`coarse_grid`, `greedy` cell usage, rtree `Point.cell_id`/`dedupe_by_cell_id`, routing `join`/`sorting`, `stats.rs`. These keep `algorithms`' direct `s2` dep.

### Task 1.1: Create `koji-core::s2` module, fold in `s2grid`

**Files:**
- Create: `crates/koji-core/src/s2/mod.rs`
- Delete: `crates/koji-core/src/s2grid.rs` (move its `create_cell_map` + the 4 tests into `s2/mod.rs`)
- Modify: `crates/koji-core/src/lib.rs` (`mod s2;` replacing `mod s2grid;`; re-export `create_cell_map`, `from_array_to_cell_id`)

- [ ] **Step 1:** Move `crates/koji-core/src/s2grid.rs` content into `crates/koji-core/src/s2/mod.rs` verbatim (the private `from_array_to_cell_id` at `s2grid.rs:17`, `create_cell_map` at `:28`, the 4 tests at `:48-78`). Update `lib.rs`: replace `mod s2grid;` + its `pub use` with `mod s2;` + `pub use s2::{create_cell_map, from_array_to_cell_id};`.
- [ ] **Step 2:** Run `cargo test -p koji-core s2::` — the 4 moved tests (`buckets_preserve_all_points`, `nearby_points_share_a_coarse_cell`, `distant_points_split_into_separate_cells`, `empty_input_yields_empty_map`) must pass unchanged.
- [ ] **Step 3:** Fix the cross-crate consumers of the old path: `crates/koji-plugins/src/plugin.rs:108` (`koji_core::create_cell_map` — re-export keeps this working; verify) and `crates/algorithms/src/clustering/partition.rs:18,262,288`. Build: `cargo build -p koji-plugins -p algorithms`.
- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "refactor(s2): fold koji-core::s2grid into a koji-core::s2 module

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

### Task 1.2: Move the s2 bridge primitives from `algorithms` → `koji-core::s2`

**Files:**
- Modify: `crates/koji-core/src/s2/mod.rs` (paste the primitives listed above)
- Modify: `crates/algorithms/src/s2.rs` (remove the moved defs; this file should end up empty-or-thin — if every symbol moved, delete it and drop `mod s2;` from `crates/algorithms/src/lib.rs`)
- Modify: `crates/koji-core/Cargo.toml` (confirm `geo`/`geojson` available for `ToGeo`; `s2` already present)

- [ ] **Step 1:** Move the primitive fns/traits/types (the MOVE list above) from `algorithms/src/s2.rs` into `koji_core::s2`. Delete the now-duplicate private `from_array_to_cell_id` (the koji-core copy from Task 1.1 and the algorithms copy are identical — keep ONE, make it `pub`). Re-export all moved public items from `koji_core::s2`.
- [ ] **Step 2:** Resolve imports inside the moved code — the bridge uses `s2::cell::Cell`, `s2::cellid::CellID`, `s2::cellunion::CellUnion`, `s2::latlng::LatLng`, `s2::rect::Rect`, `s2::region::RegionCoverer`, and `geo`/`geojson` types. These all resolve in koji-core (it deps `s2`, `geo`, `geojson`).
- [ ] **Step 3:** Run `cargo build -p koji-core`. Expected: compiles. If `algorithms/src/s2.rs` is now empty, delete it + its `mod s2;`.
- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "refactor(s2): move the s2<->geo bridge primitives into koji-core::s2

Dedups from_array_to_cell_id (was identical in algorithms + koji-core).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

### Task 1.3: Repoint algorithms-internal s2 callers to `koji_core::s2`

**Files (modify imports only):** `crates/algorithms/src/clustering/s2.rs:5,8`, `clustering/greedy.rs:6,27`, `clustering/crucible/{components.rs:10,15, mod.rs:22, refine.rs:23,1290}`, `clustering/partition.rs:5-7`, `bootstrap/s2.rs:10-15`, `routing/{join.rs:5-6, sorting.rs:9}`, `rtree/point.rs:11`, `stats.rs:442`.

- [ ] **Step 1:** Rewrite each `use crate::s2::…` → `use koji_core::s2::…` for the MOVED primitives (`cell_coverage`, `s2_grid`, `cell_intersects_polygon`, `get_region_cells`, `ToPointArray`, `ToGeo`, `from_*`). Direct `use s2::…` raw-crate imports (CellID/LatLng/Cell/Rect/RegionCoverer) stay — algorithms keeps its `s2` dep for internal cell usage.
- [ ] **Step 2:** Run `cargo build -p algorithms` then `cargo test -p algorithms` (the clustering/bootstrap/routing tests are the behavior oracle). Expected: green — pure import repoint, no logic change.
- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "refactor(s2): algorithms consume s2 primitives from koji-core::s2

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

### Task 1.4: Repoint koji-service + koji-plugins s2 callers

**Files:** `crates/koji-service/src/public/v1/s2.rs:32,53,71,96`, `crates/koji-service/src/public/v2/geo.rs:117,131,142,161`, `crates/koji-plugins/src/plugin.rs:20,108`.

- [ ] **Step 1:** Change `algorithms::s2::{circle_coverage,cell_coverage,get_polygons,get_cells}` → `koji_core::s2::…` in both the v1 (`s2.rs`) and v2 (`geo.rs`) endpoint handlers. Confirm koji-plugins uses `koji_core::s2::create_cell_map`.
- [ ] **Step 2:** Run `cargo build -p koji-service -p koji-plugins`. The 8 HTTP s2 endpoints (4×v1 `lib.rs:251-254`, 4×v2 `geo.rs:188-191`) now route through koji-core; koji-service no longer reaches into `algorithms::s2`.
- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "refactor(s2): service + plugins consume koji-core::s2; unwind service->algorithms s2 coupling

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

### Task 1.5: Section 1 verify

- [ ] Run (single batch): `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace`. Expected: all green; s2 lives in one place; `git grep "from_array_to_cell_id" crates/` shows exactly one definition (in `koji-core::s2`).

---

# Section 2 — DB models emit `KojiGeometry`

**Goal:** geofence + route Query methods return `KojiGeometryCollection`/`KojiGeometry` instead of geojson `FeatureCollection`/`Feature`. Phase 1 already shipped `geofence::Model::to_koji_geometry`; route needs the same. The old `to_feature`/`to_collection(FeatureCtx)` paths stay alive until Section 5 (some callers still use them mid-transition).

### Task 2.1: `route::Model::to_koji_geometry`

**Files:**
- Modify: `crates/koji-db/src/db/route.rs` (add the method near `to_feature` at `:77`)

- [ ] **Step 1: Write the failing test** (in `route.rs` `#[cfg(test)]`):

```rust
#[test]
fn route_model_to_koji_geometry_maps_mode_and_geometry() {
    // A route Model with mode = Type::CircleRaid and a LineString geometry json.
    let m = sample_route_model(); // helper: id, geofence_id, name, mode=Type::CircleRaid, geometry=<LineString>
    let kg = m.to_koji_geometry().unwrap();
    assert_eq!(kg.meta.mode, koji_core::Mode::Fort);      // circle_raid -> fort
    assert_eq!(kg.meta.name.as_deref(), Some(m.name.as_str()));
    assert!(matches!(kg.geometry, geo::Geometry::LineString(_)));
}
```

- [ ] **Step 2:** Run `cargo test -p koji-db route::…to_koji_geometry` — FAIL (method missing).
- [ ] **Step 3:** Implement `pub fn to_koji_geometry(&self) -> Result<koji_core::KojiGeometry, ModelError>` mirroring `geofence::Model::to_koji_geometry` (`geofence.rs:306`): parse `self.geometry` (Json) → `geo::Geometry` via the Phase 1 path, build `KojiMeta { id: Some(self.id), name: Some(self.name.clone()), mode: koji_core::Mode::from_legacy(&self.mode.to_value()), ..Default::default() }`. Route has no `parent` — leave `parent_id`/`ancestors` default.
- [ ] **Step 4:** Run the test — PASS.
- [ ] **Step 5: Commit** `feat(db): route Model::to_koji_geometry (mirrors geofence, additive)`.

### Task 2.2: geofence Query methods return `KojiGeometry*`

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs` (`get_all_collection` `:436`, `get_one`/`get_one_feature` `:343`/`:414`)

- [ ] **Step 1:** Add `get_all_koji(db, args) -> Result<KojiGeometryCollection, DbErr>` and `get_one_koji(db, id) -> Result<KojiGeometry, DbErr>` that fetch the same rows as `get_all_collection`/`get_one_feature` but map each `Model` via `to_koji_geometry()` into a `KojiGeometryCollection` (via `FromIterator`, Phase 1). Keep the old methods for now. Test: fetch a mocked/known row set → assert collection length + first item's `meta.mode`/geometry variant.
- [ ] **Step 2:** Run `cargo test -p koji-db geofence::` — PASS.
- [ ] **Step 3: Commit** `feat(db): geofence get_all_koji/get_one_koji return KojiGeometry (additive)`.

### Task 2.3: route Query methods return `KojiGeometry*`

**Files:**
- Modify: `crates/koji-db/src/db/route.rs` (`as_collection`/`get_one_feature`/`by_geofence_feature` `:535`)

- [ ] **Step 1:** Add `as_koji_collection(db, args) -> Result<KojiGeometryCollection, DbErr>` and `get_one_koji(db, id) -> Result<KojiGeometry, DbErr>` mirroring Task 2.2 over `route::Model::to_koji_geometry`. Test analogously.
- [ ] **Step 2:** Run `cargo test -p koji-db route::` — PASS.
- [ ] **Step 3: Commit** `feat(db): route as_koji_collection/get_one_koji (additive)`.

---

# Section 3 — API response swap

**Goal:** `response.rs::send()` takes `KojiGeometryCollection` and dispatches `ReturnTypeArg` through the Phase 1B inherent adapters + Phase 1 `From` impls. The v2 handlers feed it the Section 2 Query output. The old matrix path is the parity oracle (golden snapshot) until Section 5 deletes it.

The `ReturnTypeArg` dispatch (`response.rs:62-89`): `SingleStruct→to_single_struct`, `MultiStruct→to_multi_struct`, `Text→to_text(",","\n",true)`, `AltText→to_text(" ",",",false)`, `SingleArray→to_single_vec`, `MultiArray→to_multi_vec`, `Geometry→to_geometry` (Phase 1 `From<&KojiGeometryCollection> for geojson::Geometry`), `Poracle→to_poracle_vec`, `Sql→to_sql`, and the default `FeatureCollection`/`Feature` via Phase 1 `From`.

### Task 3.1: `send()` accepts `KojiGeometryCollection`

**Files:**
- Modify: `crates/koji-service/src/utils/response.rs:47` (`send`)

- [ ] **Step 1: Write the golden-parity test** — build a representative `KojiGeometryCollection` `c`; for each `ReturnTypeArg`, assert the body produced by the new `send(c, rt)` equals the old `send(FeatureCollection::from(&c), rt)` body. (Use `koji_core`'s adversarial `sample_rich()`-style value; serialize the `HttpResponse` body to bytes/json for comparison.) Whitelist: skip line-only geometries (intentional divergence — see locked decisions).
- [ ] **Step 2:** Run — FAIL (signature mismatch).
- [ ] **Step 3:** Change `send` to take `coll: KojiGeometryCollection`. Replace each match arm's `feature_collection.to_X(..)` with `coll.to_X(..)` (Phase 1B inherent methods) / `geojson::FeatureCollection::from(&coll)` / `geojson::Feature` / `geojson::Geometry::from(&coll)` for the geojson return types. `Poracle`/`PoracleSingle`: `coll.to_poracle_vec()` (single = `.first()`).
- [ ] **Step 4:** Run — PASS (all `ReturnTypeArg` arms parity-match the oracle, lines whitelisted).
- [ ] **Step 5: Commit** `feat(api): response::send() consumes KojiGeometryCollection via Phase 1B adapters`.

### Task 3.2: v2 handlers feed `KojiGeometryCollection` into `send()`

**Files:**
- Modify: `crates/koji-service/src/public/v2/geofences.rs:27,39,62,74,79`; `routes.rs:27,41,62,76,81`; `geo.rs:23,51,68,88`

- [ ] **Step 1:** In `geofences::list`/`get_one`, call `geofence::Query::get_all_koji`/`get_one_koji` (Section 2) and pass the result to `send()`. Same for `routes::list`/`get_one` via `route::Query::as_koji_collection`/`get_one_koji`. For `geo.rs` convert/simplify/merge_points, normalize the inbound geometry to `KojiGeometryCollection` (Phase 1 `TryFrom`) and return via `send()`.
- [ ] **Step 2:** Run `cargo test -p koji-service` + the existing API E2E (`P5 verify` mock harness, if present). Expected: green.
- [ ] **Step 3: Commit** `feat(api): v2 geofence/route/geo handlers return KojiGeometryCollection`.

---

# Section 4 — inbound normalization; drop `GeoFormats` + bare-array wire forms

**Goal:** request geometry (`area`) normalizes to `KojiGeometryCollection` via the Phase 1 `TryFrom`; drop the `GeoFormats` dispatch enum and the bare-array `FeatureVec`/`GeometryVec` wire encodings (spec — migrate callers to `FeatureCollection`/`GeometryCollection`). `data_points` (raw clustering points) stays `SingleVec` (boundary-only).

### Task 4.1: `area` input → `KojiGeometryCollection`

**Files:**
- Modify: `crates/model/src/api/args.rs:24,30,307-323` (the `Args.area: Option<GeoFormats>` + return-type inference); `crates/koji-service/src/public/v2/calc.rs:275-330`

- [ ] **Step 1: Write the failing test** — deserialize an `Args` JSON whose `area` is a geojson `FeatureCollection`; assert it normalizes to a `KojiGeometryCollection` with the expected item count + modes. Add a case for a bare `Feature`.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Change `area` to accept geojson `FeatureCollection`/`Feature`/`GeometryCollection` (serde-untagged or an explicit input enum) and expose `Args::area_koji(&self) -> Option<KojiGeometryCollection>` via Phase 1 `TryFrom`. Remove the `FeatureVec`(bare `[Feature]`)/`GeometryVec`(bare `[Geometry]`) variants. Update `calc.rs` consumers to call `area_koji()`.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5: Commit** `feat(api): normalize inbound area to KojiGeometryCollection; drop bare-array wire forms`.

### Task 4.2: remove `GeoFormats` construction sites

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs:769`, `route.rs:472` (`upsert_from_geometry` arg); `crates/koji-service/src/public/v1/{calculate.rs:183,263, geofence.rs:68, route.rs:104}`

- [ ] **Step 1:** Change `upsert_from_geometry` to take `&KojiGeometryCollection` (iterate items → upsert rows; geometry via `From<&KojiGeometry> for geojson::Geometry`/the stored Json shape; `mode` from `meta.mode`). Repoint the v1 construction sites to pass a `KojiGeometryCollection` (built via Phase 1 `TryFrom` from the inbound geojson). `GeoFormats` should now have zero remaining references outside its own def.
- [ ] **Step 2:** Run `cargo build --workspace`; `git grep -n "GeoFormats" crates/ | grep -v geo_formats.rs` → expect no matches.
- [ ] **Step 3: Commit** `refactor(api): route all geometry input through KojiGeometryCollection; GeoFormats unreferenced`.

---

# Section 5 — delete the `To*` matrix, `FeatureCtx`, inference; re-home utility ops

**Goal:** with S2–S4 routing everything through Phase 1/1B conversions, the `To*` cross-product, `GeoFormats`, `FeatureCtx`, `enum_map`, `ValueHelpers::get_geojson_value`, `wrapper_conversions!`, and `FenceType` are unused — delete them. **KEEP** the type aliases (`SingleVec`, `MultiVec`, `PointArray`, `PointStruct`, `SingleStruct`, `MultiStruct`, `Poracle`) — they're still boundary/return types for the Phase 1B adapters, events, and plugins. **RE-HOME** the geometry utility ops that survive.

### Task 5.1: Re-home surviving utility ops onto geo types

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_geometry.rs` (add inherent `simplify`/`bbox`/`trim_precision`/`merge_points` as needed); `crates/koji-service/src/public/v2/geo.rs:51,68` (simplify/merge-points endpoints)

- [ ] **Step 1:** Identify the survivors by grepping callers of `GeometryHelpers::simplify`, `TrimPrecision::trim_precision`, `GetBbox::get_bbox`, `EnsurePoints::ensure_first_last`, the `merge_points` logic. The geo endpoints (`geo.rs` simplify/merge-points) are the live consumers. Re-home: `simplify` → `geo::algorithm::Simplify` on `geo::Geometry` (geo crate provides it) wrapped as `KojiGeometry::simplify(self, epsilon)`; `bbox` already exists (`KojiGeometryCollection::bbox`); `trim_precision`/`merge_points` → inherent methods on `KojiGeometry`/`KojiGeometryCollection` porting the existing impls. Write a test per re-homed op asserting it matches the old trait output on a sample.
- [ ] **Step 2:** Run the new tests — PASS. Repoint `geo.rs` to the re-homed methods.
- [ ] **Step 3: Commit** `refactor(geometry): re-home simplify/trim/merge_points onto KojiGeometry`.

### Task 5.2: Delete the `To*` matrix + machinery

**Files:**
- Delete: `crates/koji-core/src/{geo_formats.rs, feature_ctx.rs, enum_map.rs, fence_type.rs}`
- Modify: `crates/koji-core/src/geometry/{mod.rs, single_vec.rs, multi_vec.rs, single_struct.rs, multi_struct.rs, feature.rs, collection.rs, geometry.rs, point_array.rs, point_struct.rs, poracle.rs, text.rs}` — remove the `To*` trait DECLARATIONS + IMPLS + `wrapper_conversions!` macro + the inference (`get_geojson_value`, the `enum_type` plumbing). KEEP the type-alias definitions + the surviving utility traits/ops from Task 5.1 + the Phase 1 `koji_geojson.rs` + Phase 1B `koji_output.rs`.
- Modify: `crates/koji-core/src/lib.rs` (drop the deleted `mod`/`pub use`); `crates/model/src/api/args.rs:299` (drop `get_enum_by_geometry_string` + `FenceType` geometry-type detection — geometry is self-describing now); `crates/koji-db/src/utils/mod.rs:21` (drop the `get_enum_by_geometry` wrapper)

- [ ] **Step 1:** Delete the four files + the trait machinery. The mode→shape inference sites (`geometry.rs:158`, `multi_vec.rs:8`, `multi_vec.rs:122`, and the `get_geojson_value` callers in `point_array.rs:38`/`point_struct.rs:56`/`poracle.rs:134`/`single_struct.rs:53`/`single_vec.rs:95`/`text.rs:71`/`multi_struct.rs:64`) all die with their enclosing `to_feature` impls.
- [ ] **Step 2:** Run `cargo build --workspace`. Fix every "unresolved import"/"cannot find" by deleting the dead reference (these are the matrix call sites S2–S4 already bypassed). Iterate until green.
- [ ] **Step 3:** Run `cargo test --workspace`. `git grep -n "trait ToSingleVec\|wrapper_conversions\|FeatureCtx\|GeoFormats\|get_enum_by_geometry\|FenceType" crates/` → expect zero (outside deleted files). The Phase 1B `koji_output.rs` parity tests still pass (they assert against `geojson::FeatureCollection::from(&c)` which is Phase 1 outbound, NOT the deleted matrix — verify the oracle in those tests still compiles; if any test referenced a deleted `To*` trait import, replace with the inherent method).
- [ ] **Step 4: Commit** `refactor(geometry)!: delete the To* conversion matrix, GeoFormats, FeatureCtx, mode->shape inference`.

> **Note on the Phase 1B oracle:** `koji_output.rs` tests call `fc(&c).to_single_vec()` etc. — those `to_*` are the matrix `To*` traits being deleted here. Before deletion these tests proved parity; after deletion the oracle is gone. **Convert each Phase 1B parity test into a golden-snapshot test**: capture the current adapter output as a checked-in expected value (the adapters are now the source of truth). Do this as the first move in Step 1 so the tests survive the deletion.

### Task 5.3: Section 5 verify

- [ ] Run the workspace gate (build/clippy/fmt/test). Confirm the `koji-core/src/geometry/` tree is now: Phase 1 types + `koji_geojson.rs` + `koji_output.rs` (golden) + type aliases + re-homed utility ops. No `To*` matrix.

---

# Section 6 — `Type→Mode` breaking migration

**Goal:** migrate `geofence.mode` + `route.mode` columns from the 12-value `Type` enum to the 4-value `Mode` enum, backfilling via `from_legacy`; switch the model field types atomically; delete the legacy enums. Irreversible-down (default `'unset'`).

Mapping (CASE): `pokemon` ← `circle_pokemon`,`circle_smart_pokemon`,`pokemon_iv`,`auto_pokemon`,`auto_tth`; `fort` ← `circle_raid`,`circle_smart_raid`,`circle_station`; `quest` ← `auto_quest`,`circle_quest`; `unset` ← `unset`,`leveling`.

### Task 6.1: Write the migration

**Files:**
- Create: `crates/migration/src/m20260613_000001_type_to_mode.rs`
- Modify: `crates/migration/src/lib.rs` (register it last)

- [ ] **Step 1:** Author a `MigrationTrait` following the pattern of `m20230221_130117_geofence_mode_enums.rs`. `up`: (a) extend each column's MySQL ENUM to include the 4 new values alongside the 12 old; (b) `UPDATE geofence SET mode = CASE … END` and the same for `route` (the mapping above); (c) shrink the ENUM to just `('unset','pokemon','fort','quest')` with default `'unset'`. `down`: set the column ENUM back to the 12-value `Type` set with every row defaulted to `'unset'` (document the lossy collapse). Use raw SQL via `manager.get_connection().execute_unprepared(...)` for the CASE, matching the precedent migration.
- [ ] **Step 2:** Build `cargo build -p migration`. (Applying against a live DB is part of S9 / manual — never echo `KOJI_DB_URL`.)
- [ ] **Step 3: Commit** `feat(migration): Type->Mode column migration (geofence.mode, route.mode; lossy-down)`.

### Task 6.2: Switch model field types atomically

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs:41` (`mode: Type`→`mode: Mode`), `route.rs:31`; `to_koji_geometry` in both (drop `Mode::from_legacy` — the field is already `Mode`); the `mode` filter in `paginate` (`geofence.rs:496`, `route.rs:137`)

- [ ] **Step 1:** Change both `Model.mode` fields `Type → Mode`. In `to_koji_geometry`, replace `Mode::from_legacy(&self.mode.to_value())` with `self.mode.into()` (the `enum_bridge!`'d `koji_db::Mode → koji_core::Mode`). Update the `mode` query filters to compare against `Mode`.
- [ ] **Step 2:** Run `cargo test -p koji-db`. Update the Task 2.1 route test (`Type::CircleRaid` sample → now construct `Mode::Fort` directly). Expected: green.
- [ ] **Step 3: Commit** `feat(db)!: geofence/route Model.mode is Mode (atomic with migration)`.

### Task 6.3: Delete the legacy enums

**Files:**
- Modify: `crates/koji-db/src/db/sea_orm_active_enums.rs` (delete `Type`, `FenceMode`, `RouteMode`; keep `Mode` + `Category`); `crates/koji-core/` (any `FenceType` remnant already gone in S5)

- [ ] **Step 1:** Delete the `Type`/`FenceMode`/`RouteMode` `DeriveActiveEnum` blocks + any `enum_bridge!` for them. `git grep -n "Type\b\|FenceMode\|RouteMode" crates/koji-db/src/db/` → only `Mode` + unrelated `Type` (e.g. type aliases) remain.
- [ ] **Step 2:** Run the workspace gate. Commit `refactor(db)!: drop legacy Type/FenceMode/RouteMode enums; Mode is canonical`.

---

# Section 7 — recursive `?depth=N` hierarchy

**Goal:** replace the name-munge args with a recursive descendant query. `?depth=N` returns the target geofence + descendants down to depth N as a flat `KojiGeometryCollection`, each item's `KojiMeta.ancestors` populated; drop `parentstart`/`parentend`/`parentreplace`; keep `parent`/`group` properties + `parent`/`mode`/`geotype` filters.

### Task 7.1: Recursive descendant query

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs` (add `descendants(db, root, depth) -> Result<KojiGeometryCollection, DbErr>`)

- [ ] **Step 1: Write the failing test** — seed (mock or test-DB) a small tree (country→state→county); call `descendants(root, depth=2)`; assert the returned collection contains root+children+grandchildren and each item's `meta.ancestors` is the ordered name path (e.g. `["country","state"]` for the county) and `meta.parent_id` is set.
- [ ] **Step 2:** Run — FAIL.
- [ ] **Step 3:** Implement via raw MySQL `WITH RECURSIVE`:

```sql
WITH RECURSIVE descendants AS (
  SELECT id, name, parent, geometry, mode, geo_type, 0 AS depth,
         CAST(name AS CHAR(1024)) AS ancestry
  FROM geofence WHERE id = ?
  UNION ALL
  SELECT g.id, g.name, g.parent, g.geometry, g.mode, g.geo_type, d.depth + 1,
         CONCAT(d.ancestry, '/', g.name)
  FROM geofence g JOIN descendants d ON g.parent = d.id
  WHERE d.depth < ?
)
SELECT * FROM descendants;
```

Map each row → `KojiGeometry` (via `to_koji_geometry`), splitting `ancestry` on `'/'` (drop the row's own trailing name) into `meta.ancestors`. Execute with `Statement::from_sql_and_values` + the `?depth` bind.
- [ ] **Step 4:** Run — PASS.
- [ ] **Step 5: Commit** `feat(db): recursive geofence descendants query (?depth=N, ancestors[])`.

### Task 7.2: Wire `?depth=N` through the API; drop name-munge args

**Files:**
- Modify: `crates/koji-core/src/query_args.rs:49-53` (delete `parentstart`/`parentend`/`parentreplace`), `crates/koji-core/src/` `name_modifier` (text_utils.rs `:103-112` — delete); `crates/koji-service/src/public/v2/geofences.rs` (`?depth` param → `descendants`)

- [ ] **Step 1:** Add `depth: Option<u32>` to the geofence list query args; when present, call `descendants(root, depth)` and return the flat collection. Delete `parentstart`/`parentend`/`parentreplace` fields + the `name_modifier` call site (`geofence.rs:254`) + the helper. Keep `parent`/`group` (`query_args.rs:25,34`) and the `parent`/`mode`/`geotype` filters (`AdminReqParsed`).
- [ ] **Step 2:** Run `cargo test -p koji-service -p koji-core`. Update any test referencing the deleted args. Update OpenAPI/docs for the new `?depth` param + removed args.
- [ ] **Step 3: Commit** `feat(api)!: ?depth=N recursive hierarchy; drop parentstart/parentend/parentreplace`.

---

# Section 8 — events emit-from-`KojiGeometry`

**Goal:** emit sites hold `KojiGeometry` and serialize to the existing external payload shapes at the boundary — no change to `RouteUpdated.route: SingleVec` / `GeofenceUpdated.geofence: Feature` (Dragonite contract).

### Task 8.1: Produce event payloads from `KojiGeometry`

**Files:**
- Modify: `crates/koji-service/src/public/v2/geofences.rs:121-160` (publish), `routes.rs:137-174` (publish), `calc.rs` persist path; `crates/koji-service/src/dragonite.rs:35,44` (payload structs unchanged)

- [ ] **Step 1: Write the failing test** — given a `KojiGeometry` (polygon) and a `KojiGeometry` (route line), assert `GeofenceUpdated` built from it carries the identical `Feature` that the old path produced (`From<&KojiGeometry> for Feature`, Phase 1), and `RouteUpdated` carries the identical `SingleVec` (`to_single_vec()` over the new collection). Golden-snapshot the payload JSON.
- [ ] **Step 2:** Run — FAIL (publish sites still build from `Feature`/`Query::get_one_feature`).
- [ ] **Step 3:** Repoint the publish handlers to fetch `get_one_koji` (Section 2), then build `GeofenceUpdated.geofence = geojson::Feature::from(&kg)` and `RouteUpdated.route = collection.to_single_vec()`. Payload structs stay as-is.
- [ ] **Step 4:** Run — PASS (payloads byte-identical; Dragonite contract preserved).
- [ ] **Step 5: Commit** `feat(events): emit Dragonite payloads from KojiGeometry (contract unchanged)`.

---

# Section 9 — final verification gate

**Files:** none (verification only)

- [ ] **Step 1:** Full workspace gate (single batch, background if slow): `cargo build --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`; `cargo test --workspace`. Expected: all green.
- [ ] **Step 2: Matrix-deletion proof:** `git grep -n "trait ToSingleVec\|trait ToCollection\|wrapper_conversions\|struct FeatureCtx\|enum GeoFormats\|get_enum_by_geometry\|FenceType\|enum Type\b\|FenceMode\|RouteMode" crates/` → zero matches. The only geometry conversion paths are `koji_geojson.rs` (Phase 1) + `koji_output.rs` (Phase 1B golden).
- [ ] **Step 3: s2-consolidation proof:** `git grep -c "from_array_to_cell_id" crates/` shows one definition; no `algorithms::s2::` references in koji-service.
- [ ] **Step 4: Format parity (golden):** confirm the S3 golden snapshots cover every `?rt=` format (SingleStruct/MultiStruct/Text/AltText/SingleArray/MultiArray/Geometry/Poracle/Sql/FeatureCollection/Feature). Document the LineString divergence as the one intentional non-parity (adapters emit line coords the old matrix dropped).
- [ ] **Step 5: Migration sanity** (manual / if a disposable test DB is wired): apply `m20260613_000001_type_to_mode` up, spot-check a few `geofence.mode`/`route.mode` rows mapped per the table, then leave the schema migrated. **Never echo `KOJI_DB_URL` or any `mysql://` string.**
- [ ] **Step 6: Commit** any fmt/clippy fixups: `chore(geometry): Phase 2 verification fixups`.

---

## Self-review

**Spec coverage (Phase 2 slice of `2026-06-13-koji-geometry-universal-type-design.md`):**
- §4/§6 universal type at boundaries → S2 (DB), S3/S4 (API), S8 (events). ✓ (boundary-only per locked decision)
- §6 collapse the To* matrix → S5. ✓
- §Mode unify `Type`/`FenceType`/`FenceMode`/`RouteMode` → S6. ✓
- §delete mode→shape inference → S5 Task 5.2. ✓
- §recursive `?depth=N` + `ancestors[]`, drop arg zoo → S7. ✓
- §keep `geo_type` → preserved (S6 untouched). ✓
- §drop bare-array wire forms → S4 Task 4.1. ✓
- §s2 friction (re-org per participate decision) → S1. ✓ (revises spec: koji-core::s2, not deferred)
- **Deferred/flagged:** total `SingleVec` replacement in algorithms = OUT (boundary-only). LineString parity = intentional divergence (whitelisted, S9). Events payloads = unchanged external contract (S8).

**Placeholder scan:** matching/parity work (S3 response, S5 oracle→golden, S8 payloads) specifies the exact oracle + golden-snapshot approach rather than pre-baking byte output (the snapshot is the precise spec, captured at implementation time). Migration CASE + recursive SQL are given concretely. No TBD/TODO.

**Type consistency:** `to_koji_geometry` (geofence existing + route Task 2.1) → feeds `get_*_koji` (S2) → `send(KojiGeometryCollection)` (S3) → Phase 1B inherent adapters (`to_single_vec`/…/`to_poracle_vec`) + Phase 1 `From` impls. `Mode` is the single mode enum post-S6; `Model.mode: Mode` matches `KojiMeta.mode: Mode` via the `enum_bridge!`. Type aliases (`SingleVec` etc.) survive S5 as boundary types. Names consistent across sections.

**Risk notes:** S5 is the irreversible "delete" — gated behind S2–S4 having already bypassed every matrix call site (S4 Task 4.2 + S3 prove zero live callers before deletion). S6 is the only DB-data-touching step — irreversible-down by decision. S1 is behavior-preserving (existing s2/algorithm tests are the oracle).
