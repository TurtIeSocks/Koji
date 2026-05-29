# Koji V2 — Phase 1c: split `model` → `koji-db` + `koji-scanner`

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans or superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`).

**Goal:** Extract `koji-scanner` (golbat read-only) and `koji-db` (Koji's own entities + db infra + RDM + `KojiDb`) out of the fused `model` crate; move the cycle-breaking shared types (`GeoFormats` + query/admin args + pure utils + domain-enum mappers) into `koji-core`; migrate all consumers. No facades.

**Architecture:** Compiler-driven structural refactor. Each task is a green sub-commit (`cargo build --workspace` + `cargo test --workspace` parity with P1b: **16+3 pass, 4+1 ignored**). Final dep graph: everything → `koji-core`; `api` → core + koji-db + koji-scanner + model + algorithms; no cycles. Design: `docs/superpowers/specs/2026-05-29-koji-v2-p1c-db-scanner-split-design.md`.

**Tech Stack:** Rust 2024, sea-orm 1.1, cargo workspace (`members = ["crates/*","bins/*"]`).

**Method note (not a placeholder):** import fixes after a move are *compiler-discovered*. Where a step says "build → fix the flagged imports → repeat until green," that is the proven P1-conv method: run `cargo build --workspace --message-format=short 2>&1 | grep -E "error\[|error:"`, apply the mechanical translation in the step's table, rebuild. Behavior is unchanged; the existing test suite is the safety net.

---

### Task 1: Move cycle-breaking shared types into `koji-core`

Moves `GeoFormats`, the query/admin arg structs, pure utils, and the domain-enum mappers from `model` into `koji-core`, so `koji-db`/`koji-scanner` (Task 2/3) can depend only on `koji-core`.

**Files:**
- Create: `crates/koji-core/src/geo_formats.rs`, `crates/koji-core/src/query_args.rs`, `crates/koji-core/src/text_utils.rs`, `crates/koji-core/src/enum_map.rs`, `crates/koji-core/src/normalize.rs`
- Modify: `crates/koji-core/src/lib.rs`, `crates/koji-core/Cargo.toml`, root `Cargo.toml`
- Modify (delete moved items + re-point): `crates/model/src/api/mod.rs`, `crates/model/src/api/args.rs`, `crates/model/src/utils/mod.rs`, `crates/model/src/utils/normalize.rs`

- [ ] **Step 1: Add koji-core deps.** Root `Cargo.toml` `[workspace.dependencies]`: add `regex = "1.11.2"`. `crates/koji-core/Cargo.toml`: add under `[dependencies]`: `regex = { workspace = true }` and promote `serde_json = { workspace = true }` from dev-deps to deps (keep it in dev-deps too is fine, but it must be a normal dep now).

- [ ] **Step 2: Create `query_args.rs`** — move verbatim from `model/src/api/args.rs`: `ApiQueryArgs` (+ its `Default` impl), `BoundsArg`, `SpawnpointTth`, `AdminReq` (+ its `parse` impl), `AdminReqParsed`. Add `use crate::{Precision, SingleVec};` as needed (BoundsArg uses `Precision`). Use `serde` derives (already a dep). These reference no other `model` types (verified: primitives + `SpawnpointTth`).

- [ ] **Step 3: Create `geo_formats.rs`** — move `GeoFormats` enum + its `impl ToCollection for GeoFormats` from `model/src/api/mod.rs`. Change `Bound(args::BoundsArg)` → `Bound(crate::BoundsArg)` (or `BoundsArg` via local use). Imports: `use crate::{...geometry traits..., BoundsArg, FeatureCtx, MultiStruct, MultiVec, Poracle, SingleStruct, SingleVec, ...};` plus `geojson::{Feature, FeatureCollection, Geometry}`. The `ToCollection` impl body is already FeatureCtx-based (P1-conv).

- [ ] **Step 4: Create `text_utils.rs`** — move from `model/src/utils/mod.rs`: `clean`, `separate_by_comma`, `json_related_sort`, `get_mode_acronym`, `name_modifier`, and its privates `remove_symbols`, `convert_polish_to_ascii`. `name_modifier` takes `&ApiQueryArgs` → `use crate::ApiQueryArgs;`. `remove_symbols` uses `regex::Regex`. `json_related_sort` uses `serde_json`.

- [ ] **Step 5: Create `enum_map.rs`** — move `get_enum`, `get_enum_by_geometry`, `get_category_enum`, `get_enum_by_geometry_string` from `model/src/utils/mod.rs`, **rewritten to return koji-core domain enums**:
  - `get_enum(Option<String>) -> FenceType` (map the same strings to `FenceType` variants; default `FenceType::Unset`).
  - `get_enum_by_geometry(&geojson::Value) -> FenceType` (Point→Leveling, MultiPoint→CircleSmartPokemon, Polygon→PokemonIv, MultiPolygon→AutoQuest, else Unset+warn).
  - `get_category_enum(String) -> Category` (koji-core `Category`).
  - `get_enum_by_geometry_string(Option<String>) -> Option<FenceType>` (point→Leveling, multipoint→CirclePokemon, multipolygon→AutoQuest, else None).
  Imports: `use crate::{FenceType, Category}; use geojson::Value;`.

- [ ] **Step 6: Create `normalize.rs`** — move the **pure** bits from `model/src/utils/normalize.rs`: trait `HasLatLon`, its `impl HasLatLon for PointStruct`, struct `AreaPolygons` (+ impl), `count_in_area<T: HasLatLon>`. Imports: `use crate::PointStruct; use geo::{Contains, MultiPolygon, Point, Polygon}; use geojson::{FeatureCollection, Value};`. (The `Spawnpoint`/`LatLonRow` impls + `fort`/`spawnpoint*` go to koji-scanner in Task 2; `instance`/`area`/`area_ref` go to koji-db in Task 3.)

- [ ] **Step 7: Wire `lib.rs`** — add to `crates/koji-core/src/lib.rs`:
```rust
mod enum_map;
mod geo_formats;
mod normalize;
mod query_args;
mod text_utils;

pub use enum_map::{get_category_enum, get_enum, get_enum_by_geometry, get_enum_by_geometry_string};
pub use geo_formats::GeoFormats;
pub use normalize::{AreaPolygons, HasLatLon, count_in_area};
pub use query_args::{AdminReq, AdminReqParsed, ApiQueryArgs, BoundsArg, SpawnpointTth};
pub use text_utils::{clean, get_mode_acronym, json_related_sort, name_modifier, separate_by_comma};
```

- [ ] **Step 8: Delete moved items from `model`** and leave the rest. In `model/src/api/mod.rs` delete `GeoFormats` + its impl (keep the enum's old consumers compiling via re-export only if needed — prefer no re-export; consumers re-point in Step 10). In `model/src/api/args.rs` delete the 5 moved structs. In `model/src/utils/mod.rs` delete the 5 moved fns + the moved enum-mappers (keep `get_database_struct`, `parse_order`, env bootstrap — those go to koji-db in Task 3, still here for now). In `model/src/utils/normalize.rs` delete the moved pure items (keep `fort`/`spawnpoint*`/`instance`/`area`/`area_ref` + scanner HasLatLon impls for now).

- [ ] **Step 9: Build koji-core alone.** Run: `cargo build -p koji-core`. Expected: PASS. Fix any missing `use`/`Precision`/derive in the new files.

- [ ] **Step 10: Re-point `model` + `api` + `algorithms` to koji-core for the moved items.** Build → fix → repeat:

| Old path | New path |
|---|---|
| `model::api::GeoFormats` | `koji_core::GeoFormats` |
| `model::api::args::{ApiQueryArgs,BoundsArg,SpawnpointTth,AdminReq,AdminReqParsed}` | `koji_core::{…}` |
| `model::utils::{clean,separate_by_comma,json_related_sort,get_mode_acronym,name_modifier}` | `koji_core::{…}` |
| `model::utils::{get_enum,get_enum_by_geometry,get_category_enum,get_enum_by_geometry_string}` | `koji_core::{…}` (now return domain enums) |
| `model::utils::normalize::{HasLatLon,AreaPolygons,count_in_area}` | `koji_core::{…}` |

At call sites that previously did `get_enum_by_geometry_string(x).map(Into::into)` to get `FenceType`, drop the `.map(Into::into)` (the mapper now returns `FenceType`). Where db code needs the sea-orm `Type`/`Category` from a mapper result, add `.into()` (enum_bridge) at that site — handle in Task 3.

Run: `cargo build --workspace --message-format=short 2>&1 | grep -E "error\[|error:"` → fix → repeat until empty.

- [ ] **Step 11: Test + commit.** Run: `cargo test --workspace 2>&1 | tail -20`. Expected: 16+3 pass, 4+1 ignored. Then:
```bash
git add -A
git commit -m "refactor(core): move GeoFormats + query args + shared utils into koji-core

Breaks the model<->koji-db cycle ahead of the P1c crate split: GeoFormats,
ApiQueryArgs/BoundsArg/SpawnpointTth/AdminReq(Parsed), the pure string utils,
the pure normalize helpers, and the geometry/category enum mappers (now
returning koji-core FenceType/Category) move into koji-core.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Extract `koji-scanner` (golbat read-only)

**Files:**
- Create: `crates/koji-scanner/Cargo.toml`, `crates/koji-scanner/src/lib.rs`, `crates/koji-scanner/src/error.rs`, `crates/koji-scanner/src/entities/{mod.rs,gym.rs,pokestop.rs,spawnpoint.rs,station.rs}`, `crates/koji-scanner/src/rows.rs`, `crates/koji-scanner/src/normalize.rs`
- Modify: root `Cargo.toml` (workspace.dependencies), consumers (`api`)

- [ ] **Step 1: Scaffold crate.** Create `crates/koji-scanner/Cargo.toml`:
```toml
[package]
name = "koji-scanner"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "koji_scanner"
path = "src/lib.rs"

[dependencies]
koji-core = { path = "../koji-core" }
sea-orm = { version = "1.1.16", features = ["sqlx-mysql", "runtime-actix-native-tls", "macros"] }
geojson = { workspace = true }
geo = { workspace = true }
serde = { workspace = true }
log = { workspace = true }
thiserror = { workspace = true }
```
Add to root `Cargo.toml` `[workspace.dependencies]`: `koji-scanner = { path = "crates/koji-scanner" }` and `koji-db = { path = "crates/koji-db" }` (used in Task 3).

- [ ] **Step 2: `error.rs`** — own error:
```rust
use sea_orm::DbErr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScannerError {
    #[error("Database Error: {0}")]
    Database(#[from] DbErr),
    #[error("Geojson Error: {0}")]
    Geojson(#[from] geojson::Error),
    #[error("{0}")]
    Custom(String),
}
```

- [ ] **Step 3: Move entities.** `git mv crates/model/src/db/{gym,pokestop,spawnpoint,station}.rs crates/koji-scanner/src/entities/`. Create `entities/mod.rs` with `pub mod gym; pub mod pokestop; pub mod spawnpoint; pub mod station;`. In each entity file: replace `use super::*;` and `use crate::...` with explicit imports — they need `sea_orm` prelude, `koji_core::{BoundsArg, SpawnpointTth, PointStruct, SingleVec, ...}`, the row types (Step 4), and `ScannerError`. Replace `ModelError` → `ScannerError`, `model::api::args::BoundsArg` → `koji_core::BoundsArg`.

- [ ] **Step 4: `rows.rs`** — move from `model/src/db/mod.rs`: `LatLonRow` (+ its `From<LatLonRow> for koji_core::PointStruct`), `Spawnpoint`, `GenericData` (+ its `koji_core::ToPointArray`/`ToPointStruct` impls), `GenericDataToVec` (+ impl). `use koji_core::{PointArray, PointStruct, SingleVec, ToPointArray};`.

- [ ] **Step 5: `normalize.rs`** — move from `model/src/utils/normalize.rs`: `impl HasLatLon for Spawnpoint`, `impl HasLatLon for LatLonRow`, and fns `fort`, `fort_filtered`, `spawnpoint`, `spawnpoint_filtered`. `use koji_core::{AreaPolygons, HasLatLon, count_in_area}; use crate::rows::{GenericData, LatLonRow, Spawnpoint}; use geojson::FeatureCollection;`.

- [ ] **Step 6: `lib.rs`:**
```rust
pub mod entities;
mod error;
mod normalize;
mod rows;

pub use error::ScannerError;
pub use normalize::{fort, fort_filtered, spawnpoint, spawnpoint_filtered};
pub use rows::{GenericData, GenericDataToVec, LatLonRow, Spawnpoint};
```
(Re-export entities as `koji_scanner::entities::gym` etc.; adjust to match how `api` referenced `model::db::gym` — see Step 8.)

- [ ] **Step 7: Build koji-scanner alone.** Run: `cargo build -p koji-scanner`. Fix imports/feature-gates until PASS.

- [ ] **Step 8: Re-point consumers.** In `api` (and anywhere): `model::db::{gym,pokestop,spawnpoint,station}` → `koji_scanner::entities::{…}`; `model::db::{GenericData,GenericDataToVec,Spawnpoint,LatLonRow}` → `koji_scanner::{…}`; `model::utils::normalize::{fort,fort_filtered,spawnpoint,spawnpoint_filtered}` → `koji_scanner::{…}`. Add `koji-scanner = { workspace = true }` to `crates/api/Cargo.toml`. The scanner queries take `&conn.scanner` (a `&DatabaseConnection`) — unchanged at call sites. Build → fix → repeat.

- [ ] **Step 9: Test + commit.** `cargo test --workspace 2>&1 | tail -20` (16+3 / 4+1). Commit:
```bash
git add -A
git commit -m "refactor(scanner): extract koji-scanner crate (golbat read-only)

gym/pokestop/spawnpoint/station entities + scanner row types (LatLonRow,
Spawnpoint, GenericData) + scanner normalizers + own ScannerError, moved out
of model into koji-scanner (-> koji-core only). Consumers re-pointed.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Extract `koji-db` (Koji entities + db infra + RDM + KojiDb)

**Files:**
- Create: `crates/koji-db/Cargo.toml`, `crates/koji-db/src/lib.rs`, plus moved modules (entities, enums, helpers, error, conn, text, json, normalize-db, db-utils).
- Modify: consumers (`api`, `algorithms` already off db after Task 1's FenceType switch — verify), `model`.

- [ ] **Step 1: Scaffold crate.** `crates/koji-db/Cargo.toml`:
```toml
[package]
name = "koji-db"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "koji_db"
path = "src/lib.rs"

[dependencies]
koji-core = { path = "../koji-core" }
chrono = { version = "0.4.42", features = ["serde"] }
futures = "0.3.31"
geojson = { workspace = true }
geo = { workspace = true }
log = { workspace = true }
sea-orm = { version = "1.1.16", features = ["sqlx-mysql", "runtime-actix-native-tls", "macros"] }
serde = { workspace = true }
serde_json = { workspace = true }
serde_with = { workspace = true }
thiserror = { workspace = true }
```

- [ ] **Step 2: Move db modules.** `git mv` the Koji-owned entity files + infra from `crates/model/src/db/` to `crates/koji-db/src/`: `area.rs geofence.rs geofence_project.rs geofence_property.rs instance.rs project.rs property.rs route.rs tile_server.rs enum_bridge.rs sea_orm_active_enums.rs prelude.rs`. Move `model/src/db/mod.rs`'s remaining shared items (after Task 2 removed scanner rows) into `crates/koji-db/src/helpers.rs`: `NameId, NameType, NameTypeId, AreaRef, Total, PaginateResults, InsertsUpdates, VecToJson, ToFeatureFromModel, RdmInstanceArea, RdmInstance, InstanceParsing`.

- [ ] **Step 3: Move error + conn + text + json + db-utils + db-normalize.**
  - `git mv crates/model/src/error.rs crates/koji-db/src/error.rs` (`ModelError`).
  - Create `crates/koji-db/src/conn.rs`: move `KojiDb`, `ScannerType` (from `model/src/lib.rs`) + `get_database_struct`, `parse_order` (from `model/src/utils/mod.rs`).
  - `git mv crates/model/src/api/text.rs crates/koji-db/src/text.rs` (`TextHelpers`; uses `RdmInstance`/`InstanceParsing` now local, `koji_core::ToFeature`/`FeatureCtx`).
  - `git mv crates/model/src/utils/json.rs crates/koji-db/src/json.rs` (`JsonToModel`, `parse_property_value`, `determine_category_by_value`).
  - Create `crates/koji-db/src/normalize.rs`: move `instance`, `area`, `area_ref` from `model/src/utils/normalize.rs` (use `TextHelpers`, entity models, `AreaRef`, `sea_orm_active_enums::Type`).

- [ ] **Step 4: `lib.rs`** — declare modules + re-exports mirroring how consumers used `model::db::*` and `model::{KojiDb,ScannerType}`:
```rust
pub mod area;
pub mod geofence;
pub mod geofence_project;
pub mod geofence_property;
pub mod instance;
pub mod project;
pub mod property;
pub mod route;
pub mod tile_server;
mod enum_bridge;
pub mod prelude;
pub mod sea_orm_active_enums;
mod helpers;
mod conn;
mod error;
mod json;
mod text;
pub mod normalize;

pub use conn::{get_database_struct, parse_order, KojiDb, ScannerType};
pub use error::ModelError;
pub use helpers::*;
pub use json::JsonToModel;
pub use text::TextHelpers;
```
(`sea_orm_active_enums` stays `pub` for P1c — full internalization is P4. Add a `Category` `enum_bridge!` if not already present so consumers can convert `koji_core::Category` ↔ db `Category`.)

- [ ] **Step 5: Fix koji-db internals.** Within moved files: `use super::*;`/`crate::` paths now resolve within koji-db. `ModelError` stays `crate::ModelError` / `koji_db::ModelError`. Geometry/`GeoFormats`/args now come from `koji_core::`. The enum-mapper results: where db code needs sea-orm `Type`/`Category`, call `koji_core::get_enum(...).into()` (enum_bridge). Build: `cargo build -p koji-db` → fix → repeat until PASS.

- [ ] **Step 6: Re-point consumers (`api`, `model`).** Add `koji-db = { workspace = true }` to `crates/api/Cargo.toml`. Translation:

| Old | New |
|---|---|
| `model::db::{area,geofence,geofence_project,geofence_property,instance,project,property,route,tile_server}` | `koji_db::{…}` |
| `model::db::sea_orm_active_enums::{Type,Category}` | `koji_db::sea_orm_active_enums::{…}` |
| `model::db::{NameId,NameTypeId,AreaRef,Total,PaginateResults,…}` | `koji_db::{…}` |
| `model::{KojiDb,ScannerType}` | `koji_db::{KojiDb,ScannerType}` |
| `model::error::ModelError` | `koji_db::ModelError` |
| `model::api::text::TextHelpers` | `koji_db::TextHelpers` |
| `model::utils::{get_database_struct,parse_order}` | `koji_db::{…}` |
| `model::utils::normalize::{instance,area,area_ref}` | `koji_db::normalize::{…}` |

Build → fix → repeat.

- [ ] **Step 7: Test + commit.** `cargo test --workspace 2>&1 | tail -20` (16+3 / 4+1). Commit:
```bash
git add -A
git commit -m "refactor(db): extract koji-db crate (Koji entities + db infra + RDM + KojiDb)

Koji-owned sea-orm entities, db enums + bridges, query helpers, RDM types,
TextHelpers, JsonToModel, the KojiDb connection holder + get_database_struct,
and ModelError move out of model into koji-db (-> koji-core only). Consumers
re-pointed; db enums stay internal-ish (full DTO split is P4).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Shrink `model` + final verification

**Files:** `crates/model/src/lib.rs`, `crates/model/src/api/mod.rs`, `crates/model/Cargo.toml`

- [ ] **Step 1: Trim `model`.** `model/src/lib.rs` → drop the `db`, `error`, `utils` modules and the `KojiDb`/`ScannerType` defs (all moved); keep `pub mod api;`. Delete now-empty `crates/model/src/db/` and `crates/model/src/utils/` directories (all files moved). `model/src/api/mod.rs` → keep only what remains (after `GeoFormats` + `text` moved): re-export nothing db. `model/src/api/args.rs` retains `Args, ArgsUnwrapped, ReturnTypeArg, Auth, DataPointsArg, Search, get_return_type`.

- [ ] **Step 2: Trim `model/Cargo.toml` deps.** Remove now-unused deps (`sea-orm`, `regex`, `futures`, likely `chrono`); keep `koji-core`, `geojson`, `geo`/`geo-types` (if args uses them), `serde`, `serde_json`, `serde_with`. Run `cargo build -p model` → add back any dep the compiler still needs. Confirm `model` depends on **koji-core only** (no koji-db/koji-scanner).

- [ ] **Step 3: Full green build + test.** Run in parallel:
  - `cargo build --workspace`
  - `cargo test --workspace 2>&1 | tail -20` → expect 16+3 pass, 4+1 ignored.
  - `cargo tree -p model -e normal --depth 1` → confirm only `koji-core` among internal crates.

- [ ] **Step 4: rustfmt the touched/new files.** `rustfmt --edition 2024` on the new koji-db/koji-scanner/koji-core files (scoped; no repo-wide fmt). Rebuild to confirm green.

- [ ] **Step 5: Commit.**
```bash
git add -A
git commit -m "refactor(model): shrink model to the calc-args remainder (-> koji-core only)

db/scanner/error/utils all extracted to koji-db/koji-scanner/koji-core. model
now holds only api::args (Args/ArgsUnwrapped/ReturnTypeArg/Auth/DataPointsArg/
get_return_type) pending the P1d Args breakup. Dep graph acyclic; tests parity.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

- [ ] **Step 6: Update roadmap** RESUME block + P1c entry → done with the four commit SHAs; set NEXT to P1d. Commit as `docs(v2): mark P1c done; set NEXT to P1d`.

---

## Self-Review

**Spec coverage:** ✅ koji-scanner (Task 2), koji-db (Task 3), utils split (Task 1 pure→core, Task 2 scanner-normalizers, Task 3 db-utils+db-normalize), cycle break (Task 1 GeoFormats+args→core; TextHelpers→koji-db Task 3), enums-at-boundary (Task 1 mappers return domain enums; Task 3 keeps sea-orm enums + bridge). Refinement vs spec: `model` ends `→ koji-core only` (TextHelpers moving to koji-db removes model's last db edge) — strictly better than the spec's conservative "core + koji-db".

**Placeholder scan:** Cargo.toml/lib.rs/error.rs contents are concrete. Import fixes use the explicit translation tables + compiler-driven sweep (the approved P1-conv method), not "fix as needed".

**Type consistency:** crate lib names `koji_db`/`koji_scanner`; `ScannerError` (scanner) vs `ModelError` (koji-db); mappers return `FenceType`/`Category` (core) with `.into()` bridge at db sites. `entities::{gym,…}` path for scanner.

**Risks:** (1) a hidden koji-db↔koji-scanner edge (a koji-db query using a scanner row type) — investigator found none; if one surfaces, move that helper to koji-core. (2) `enum_bridge` may lack a `Category` arm — add it in Task 3 Step 4. (3) dep bumps deferred to avoid ballooning (note any skipped). (4) `prelude.rs` may reference scanner entities — split it in Task 3 if so.
