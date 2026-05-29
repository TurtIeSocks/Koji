# Koji V2 — Phase 1c: split `model` → `koji-db` + `koji-scanner` (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run); Assumptions section is the async review checkpoint.
**Parent:** [koji-v2-architecture-design](2026-05-28-koji-v2-architecture-design.md) §3–§4, §9. Roadmap: `refactor-workspace/roadmap.md` P1c.

## Goal

Dissolve the fused `model` crate's data layer into two focused crates that depend only on `koji-core`:

- **`koji-scanner`** — read-only access to golbat's data DB (`gym`, `pokestop`, `spawnpoint`, `station`).
- **`koji-db`** — Koji's own sea-orm entities + queries + db infra (enums, bridges, helpers, RDM types, the `KojiDb` connection holder + bootstrap, JSON→ActiveModel builders).

Split `model::utils` (pure → `koji-core`, db → `koji-db`, scanner-normalizers → `koji-scanner`). Migrate consumers (`api`, `algorithms`) as types move — **no transitional facades**.

## Why this is the riskiest P1 step

`model` is a tangle: golbat reads, Koji tables, RDM cruft, query-rows that double as API DTOs, and `utils` mixing pure string helpers with db-enum mappers + the connection bootstrap. The hard part is a **dependency cycle**, not the mechanical moves.

### The cycle

- koji-db's `geofence`/`area`/`route`/`instance` queries take **`GeoFormats`** (upsert-from-geometry) and **`ApiQueryArgs`** / **`AdminReqParsed`** (list/admin queries) — all currently in `model::api`.
- `model::api::text` (`TextHelpers::parse_scanner_instance`) needs koji-db's **RDM types** (`RdmInstance`, `InstanceParsing`).

So `model::api ↔ koji-db` would be circular.

### The break (decision)

Move types to their **final architectural home** (per §4 "koji-core is the universal sink"), so the new crates point only at `koji-core`:

1. **`GeoFormats` → `koji-core`.** The P1b note deferred this to P1d *only* because `GeoFormats::Bound(BoundsArg)` referenced `model`'s `BoundsArg`. Moving `BoundsArg` too (below) removes the blocker; GeoFormats moves now because it's **required** to break the cycle for a clean koji-db.
2. **Query/admin arg structs → `koji-core`:** `ApiQueryArgs`, `BoundsArg`, `SpawnpointTth`, `AdminReq`, `AdminReqParsed`. Verified self-contained — primitives + `SpawnpointTth`; **no** reference to `ReturnTypeArg`/`Args`, so this does **not** bleed into P1d's calc-`Args` breakup.
3. **`TextHelpers` (text.rs) → `koji-db`.** It's RDM/db-coupled (uses `RdmInstance`); it belongs with the RDM types and is deleted there in P6.

Result: `model::api` keeps only the **calc-args remainder** (`Args`, `ArgsUnwrapped`, `ReturnTypeArg`, `Auth`, `DataPointsArg`, `Search`, `get_return_type`) for P1d. `koji-db → koji-core` only; `model → koji-core + koji-db`; acyclic.

## Target dependency graph (after P1c)

```
koji-core  ← koji-scanner, koji-db, model, algorithms, api      (universal sink)
koji-db    ← model, api                                          (→ koji-core)
koji-scanner ← api                                               (→ koji-core)
model      ← api                                                 (→ koji-core, koji-db; api/args + api/text-less remainder)
algorithms ← api                                                 (→ koji-core; drops its db-enum dep, see §enums)
```
No cycles. `koji-scanner` and `koji-db` are independent siblings (no koji-db↔koji-scanner edge — confirmed: no koji-table query reads a scanner table, and scanner result types aren't used by koji-db queries).

## Crate contents

### `koji-core` gains (down payment, justified by cycle-break + utils-split)
- **GeoFormats** (`Bound` → `koji_core::BoundsArg`).
- **Query/admin args:** `ApiQueryArgs`, `BoundsArg`, `SpawnpointTth`, `AdminReq`, `AdminReqParsed`.
- **Pure utils** (from `model::utils`): `clean`, `separate_by_comma`, `json_related_sort`, `get_mode_acronym`, `name_modifier` (+ its private helpers `remove_symbols`, `convert_polish_to_ascii`).
- **Domain-enum mappers, return domain enums** (not db enums): `get_enum`→`FenceType`, `get_category_enum`→`koji_core::Category`, `get_enum_by_geometry`/`get_enum_by_geometry_string`→`FenceType`. Callers needing the sea-orm `Type`/`Category` convert via `enum_bridge` `.into()` at the db boundary.
- **Pure normalize bits:** `HasLatLon`, `AreaPolygons`, `count_in_area`.

### `koji-scanner` (new; → koji-core)
- Entities + queries: `gym`, `pokestop`, `spawnpoint`, `station` (all read the `scanner` connection).
- Scanner result/row types: `LatLonRow`, `Spawnpoint`, `GenericData`, `GenericDataToVec`.
- Scanner normalizers (from `model::utils::normalize`): `fort`, `fort_filtered`, `spawnpoint`, `spawnpoint_filtered`.
- **`ScannerError`** (own thiserror error; wraps `DbErr`/`geojson::Error`). Pre-stages P7's per-crate error types and keeps `koji-scanner → koji-core` only.
- Queries take `&DatabaseConnection` (the caller passes `KojiDb.scanner`); the crate does not own connections.

### `koji-db` (new; → koji-core)
- Koji entities + queries: `area`, `geofence`, `geofence_project`, `geofence_property`, `instance`, `project`, `property`, `route`, `tile_server`.
- DB enums: `sea_orm_active_enums` (`Type`, `Category`, `FenceMode`, `RouteMode`) + `enum_bridge` (kept; add a `Category` bridge if missing).
- DB helpers: `NameId`, `NameType`, `NameTypeId`, `AreaRef`, `Total`, `PaginateResults`, `InsertsUpdates`, `VecToJson`, `ToFeatureFromModel`, `JsonToModel` (+ `parse_property_value`, `determine_category_by_value`), `parse_order`.
- RDM types: `RdmInstance`, `RdmInstanceArea`, `InstanceParsing`, **`TextHelpers`** (moved from `model::api::text`).
- Connection layer: **`KojiDb`** struct, `ScannerType`, `get_database_struct` (RDM detection stays until P6).
- **`ModelError`** (moved from `model::error`; db-centric, wraps `DbErr`). Keep the name for P1c to bound churn; per-crate rename is P7.

### `model` remainder (shrinks; dissolves in P1d)
- `api/args.rs`: `Args`, `ArgsUnwrapped`, `ReturnTypeArg`, `Auth`, `DataPointsArg`, `Search`, `get_return_type`.
- `api/mod.rs`: emptied of `GeoFormats` (re-export from `koji_core` if convenient for consumers, else drop).
- `lib.rs`: loses `KojiDb`/`ScannerType` (→ koji-db). Becomes a thin `pub mod api;`.

## `entity = DTO` — staged, not fully resolved here

§4's "explicit response DTOs in `koji-service`" lands in **P4** (koji-service doesn't exist yet). P1c achieves the achievable part of "stop entity=DTO":
- koji-db **owns** its entities (no longer fused into `model`).
- **Domain enums at the boundary:** the moved mappers return `koji-core` `FenceType`/`Category`; db `Type`/`Category` stay db-internal, crossed via `enum_bridge`.
- The query-result structs (`NameId`, `AreaRef`, `NameTypeId`, …) **keep their `Serialize` derives** and still double as response shapes for now. Decoupling them into explicit response DTOs is **deferred to P4**.

## Consumer migration (compiler-driven, like P1-conv)
- `api` crate: `model::db::{gym,pokestop,spawnpoint,station}` → `koji_scanner::…`; `model::db::{area,geofence,instance,project,route,…}` → `koji_db::…`; `model::{KojiDb,ScannerType}` → `koji_db::…`; `model::error::ModelError` → `koji_db::ModelError`; `model::api::{GeoFormats, args::{ApiQueryArgs,BoundsArg,…}}` → `koji_core::…`; calc `Args`/`ReturnTypeArg` stay `model::api::args::…`.
- `algorithms`: `model::db::sea_orm_active_enums::Type` → switch to `koji_core::FenceType` (drops the db dep entirely → `algorithms → koji-core`). `vrp.rs`'s `model::` is the external `vrp_pragmatic` crate (file unused) — untouched.

## Workspace / naming
- New members `crates/koji-db`, `crates/koji-scanner` in `[workspace.members]`; deps via `{ workspace = true }`.
- Existing `model` keeps its package name (dissolves in P1d). `macros`/`api`/`algorithms` keep their names (renamed in their own rewrite phases per the P0 deviation).
- Bump touched crates' external deps to latest + workspace-manage (cross-cutting convention) — apply pragmatically; skip if a bump forces unrelated breakage that balloons the phase (note any skipped).

## Sub-commit plan (each compiles + tests green)
1. **koji-core additions** — move GeoFormats + query/admin args + pure utils + domain-enum mappers into koji-core; re-point `model`/`api`/`algorithms` imports; delete the moved items from `model`. Green.
2. **koji-scanner** — new crate; move scanner entities + result types + normalizers + `ScannerError`; re-point `api` scanner imports. Green.
3. **koji-db** — new crate; move koji entities + db infra + RDM + TextHelpers + ModelError + KojiDb + bootstrap + json builders + db utils; re-point all consumers. Green.
4. **model shrink** — reduce `model` to the calc-args remainder; final import cleanup; full workspace test parity (16+3 pass, 4+1 ignored). Green.

(Detailed steps come from the writing-plans pass.)

## Assumptions (DELEGATE CHECKPOINT — review these)
1. **GeoFormats + 5 query/admin arg structs move to koji-core now** (not P1d). Justified: required to break the koji-db↔model cycle without a facade; the P1b note pre-authorized GeoFormats→core. The calc `Args` super-struct breakup stays P1d.
2. **`koji-scanner` gets its own `ScannerError`; `koji-db` keeps `ModelError`.** Avoids `koji-scanner → koji-db` and pre-stages P7 per-crate errors. (Alternative considered: one shared error in koji-db — rejected as it couples scanner to Koji's db.)
3. **Domain-enum mappers return `koji-core` enums** (`FenceType`/`Category`), with `enum_bridge` at the db boundary. (Alternative: keep returning db `Type` — rejected; pushes db enums into pure consumers, against §4.)
4. **`entity = DTO` only partially resolved** — crate ownership + enums-at-boundary now; explicit response DTOs deferred to P4 (koji-service). The `Serialize` query-result structs stay as-is.
5. **`TextHelpers`/RDM types live in `koji-db`** (deleted there in P6), so `model` keeps no db coupling.
6. **`ModelError` name retained** for P1c (rename to a per-crate error is P7).
7. **`KojiDb` (3 connections incl. `controller`) and `get_database_struct` RDM detection stay intact** — RDM removal is P6, after the event/Dragonite write-path replacement.
8. **Dep bumps applied opportunistically**, skipped where they'd balloon the phase (each skip noted in the commit).
