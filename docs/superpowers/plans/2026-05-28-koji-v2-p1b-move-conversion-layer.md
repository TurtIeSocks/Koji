# Koji V2 — Phase 1b: move the geometry + conversion layer into koji-core

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Physically relocate the now-pure geometry types, conversion traits/impls, `GeoFormats`, `BBox`, `Precision`, and the pure utils they need (`TrimPrecision`, `sql_raw`, `sql_raw_bbox`) from `crates/model/src/api` into `crates/koji-core`. `model::api` becomes a thin re-export facade so `algorithms`/`api`/bins keep compiling unchanged.

**Architecture:** koji-core grows from "enums only" to the full geometry/GeoJSON domain layer. Two carve-outs: (1) `PointStruct` moves to core *pure* (no sea-orm `FromQueryResult`); a new `koji-db`... actually `model::db` gains a `LatLonRow` query-row type that derives `FromQueryResult` and converts to `PointStruct`, used by the 12 scanner SELECT sites. (2) `text.rs` splits — pure `To*` impls for `String` move to core; the RDM-coupled `TextHelpers`/`parse_scanner_instance` stays in `model` (removed in P6).

**Tech Stack:** Rust 2024, serde, geojson, geo, sea-orm (db only).

**Reference:** architecture spec §4; the P1b move-map (consumer surface, 12 PointStruct query sites in gym/pokestop/station, utils purity, text split).

**Consumer surface the facade must keep exposing** (from the move-map): `algorithms` uses `Precision, ToFeature, GetBbox, point_array::PointArray, single_vec::SingleVec` (+ the strategy-enum shims, already done); `api` uses `Precision, ToGeometry, ToSql, ToMultiStruct, ToMultiVec, ToPoracleVec, ToSingleStruct, ToSingleVec, ToText, GeoFormats` (+ `args::ReturnTypeArg`, `args::BoundsArg` — `args` stays in model, P1d).

---

## File moves (verbatim, via git mv) → koji-core/src/

```
model/src/api/single_vec.rs      → koji-core/src/geometry/single_vec.rs
model/src/api/multi_vec.rs       → koji-core/src/geometry/multi_vec.rs
model/src/api/single_struct.rs   → koji-core/src/geometry/single_struct.rs
model/src/api/multi_struct.rs    → koji-core/src/geometry/multi_struct.rs
model/src/api/point_array.rs     → koji-core/src/geometry/point_array.rs
model/src/api/point_struct.rs    → koji-core/src/geometry/point_struct.rs   (drop FromQueryResult)
model/src/api/poracle.rs         → koji-core/src/geometry/poracle.rs
model/src/api/feature.rs         → koji-core/src/geometry/feature.rs
model/src/api/geometry.rs        → koji-core/src/geometry/geometry.rs
model/src/api/collection.rs      → koji-core/src/geometry/collection.rs
```
The trait defs + `GeoFormats` + `BBox` + `Precision` currently in `model/src/api/mod.rs` move to `koji-core/src/geometry/mod.rs`. Pure utils move to `koji-core/src/util.rs`.

---

### Task 1: Add geometry deps to koji-core + scaffold the module tree

**Files:** `crates/koji-core/Cargo.toml`, `crates/koji-core/src/lib.rs`, `crates/koji-core/src/geometry/mod.rs`, `crates/koji-core/src/util.rs`

- [ ] **Step 1: Cargo.toml deps**

Add to `crates/koji-core/Cargo.toml` `[dependencies]` (workspace-managed):
```toml
geojson = { workspace = true }
geo = { workspace = true }
geo-types = "0.7.17"
regex = "1.11.2"
```
(geo-types/regex aren't in `[workspace.dependencies]` yet — add them there too, or inline-pin as shown; prefer adding to root `[workspace.dependencies]` and using `{ workspace = true }`.)

- [ ] **Step 2: lib.rs — declare modules + re-export the geometry surface flat**

Append to `crates/koji-core/src/lib.rs`:
```rust
pub mod geometry;
mod util;

pub use util::{sql_raw, sql_raw_bbox, TrimPrecision};
// Flat re-export so `koji_core::SingleVec`, `koji_core::ToFeature`, etc. work,
// matching how model::api exposed them.
pub use geometry::*;
```

- [ ] **Step 3: Create `koji-core/src/util.rs`** with the three pure helpers moved from `model/src/utils/mod.rs` (copy `TrimPrecision` trait body ~lines 18-30, `sql_raw` ~32-60, `sql_raw_bbox` ~62-90 verbatim). Fix their imports to `use crate::geometry::*;` for any conversion-trait use. Delete those three items from `model/src/utils/mod.rs` in Task 5.

---

### Task 2: Move the geometry files + traits into koji-core

- [ ] **Step 1: git mv the 10 conversion files**
```bash
cd /Users/rin/GitHub/Koji/.claude/worktrees/goofy-tu-5372ed
mkdir -p crates/koji-core/src/geometry
for f in single_vec multi_vec single_struct multi_struct point_array point_struct poracle feature geometry collection; do
  git mv crates/model/src/api/$f.rs crates/koji-core/src/geometry/$f.rs
done
```

- [ ] **Step 2: Create `koji-core/src/geometry/mod.rs`** — move the trait defs + `GeoFormats` + `BBox` + `Precision` from `model/src/api/mod.rs` (lines 26-234, the part after the `use`/`pub mod` block). Declare the moved files as submodules:
```rust
mod collection;
mod feature;
mod geometry;
mod multi_struct;
mod multi_vec;
mod point_array;
mod point_struct;
mod poracle;
mod single_struct;
mod single_vec;
mod text;   // pure String impls — Task 4

pub use multi_struct::MultiStruct;
pub use multi_vec::MultiVec;
pub use point_array::PointArray;
pub use point_struct::PointStruct;
pub use poracle::Poracle;
pub use single_struct::SingleStruct;
pub use single_vec::SingleVec;

use crate::FenceType;          // conversion traits take FenceType
use geojson::{Bbox, Feature, FeatureCollection, Geometry, Value};
use geo::Point;

pub type Precision = f64;
// ... (all the trait defs EnsurePoints..ToSql, GeoFormats, BBox — moved verbatim from model api/mod.rs) ...
```

- [ ] **Step 3: Fix the moved files' imports.** Each moved file used `use super::*;` (which resolved against `model::api`). Now `super` is `koji_core::geometry`, so `use super::*;` still resolves to the trait defs + sibling types in `geometry/mod.rs`. Files that used `use utils::TrimPrecision;` / `use self::utils::sql_raw;` change to `use crate::{TrimPrecision, sql_raw};`. The `FenceType` references resolve via `super::*` (re-exported in geometry/mod.rs). Run `cargo build -p koji-core` and fix residual import paths the compiler names.

- [ ] **Step 4: `point_struct.rs` — drop the sea-orm derive.** Remove `FromQueryResult` from its `#[derive(...)]` and any `use ...FromQueryResult`. PointStruct becomes a pure `{ lat, lon }` struct (keep `Serialize`/`Deserialize`).

---

### Task 3: db query-row type for the 12 PointStruct SELECT sites

**Files:** `crates/model/src/db/mod.rs` (or a new `crates/model/src/db/rows.rs`), `crates/model/src/db/{gym,pokestop,station}.rs`

- [ ] **Step 1: Define `LatLonRow`** in `crates/model/src/db/mod.rs`:
```rust
use sea_orm::FromQueryResult;

/// Query-row for `SELECT lat, lon` from scanner tables. Converts to the pure
/// koji-core PointStruct.
#[derive(Debug, FromQueryResult)]
pub struct LatLonRow {
    pub lat: f64,
    pub lon: f64,
}

impl From<LatLonRow> for koji_core::PointStruct {
    fn from(r: LatLonRow) -> Self {
        koji_core::PointStruct { lat: r.lat, lon: r.lon }
    }
}
```
> Confirm the column field names (`lat`/`lon`) match what the old PointStruct used for `FromQueryResult` (check `point_struct.rs` field names before the move).

- [ ] **Step 2: Update the 12 query sites** — in `gym.rs` (71,97,114,131), `pokestop.rs` (77,103,120,137), `station.rs` (65,86,104,122): change `.into_model::<api::point_struct::PointStruct>()` → `.into_model::<crate::db::LatLonRow>()`. The result `Vec<LatLonRow>` then maps to whatever the method returns; append `.into_iter().map(Into::into).collect::<Vec<koji_core::PointStruct>>()` (or to `SingleVec` if the method builds `[lat,lon]`). Run `cargo build -p model`; the compiler flags each downstream type mismatch — fix with the `.map(Into::into)` conversion to PointStruct.

---

### Task 4: Split text.rs

**Files:** `crates/koji-core/src/geometry/text.rs` (new — pure), `crates/model/src/api/text.rs` (remains — RDM)

- [ ] **Step 1: Create `koji-core/src/geometry/text.rs`** with the PURE impls moved from `model/src/api/text.rs`: `impl ToPointArray/ToSingleVec/ToMultiVec/ToPointStruct/ToSingleStruct/ToMultiStruct/ToFeature/ToCollection/ToPoracle for String` (lines 49-159). `use super::*;` for the traits.

- [ ] **Step 2: Trim `model/src/api/text.rs`** to keep ONLY the RDM-coupled `TextHelpers` trait + `impl TextHelpers for String` (lines 3-46, `parse_scanner_instance` using `InstanceParsing`/`RdmInstanceArea`). It now needs the To* traits from koji-core: change its imports to `use koji_core::{ToFeature, ToSingleVec, ...}` as the compiler requires. (This whole file is deleted in P6 with RDM.)

---

### Task 5: Turn `model::api` into a re-export facade

**Files:** `crates/model/src/api/mod.rs`, `crates/model/src/utils/mod.rs`

- [ ] **Step 1: Rewrite `crates/model/src/api/mod.rs`** as a facade. Remove the moved trait defs + GeoFormats/BBox/Precision (now in koji-core). Keep `pub mod args;` and the strategy-enum shim modules. Add:
```rust
// Geometry/conversion layer now lives in koji-core; re-exported for back-compat.
pub use koji_core::{
    BBox, EnsurePoints, EnsureProperties, FeatureHelpers, GeoFormats, GeometryHelpers, GetBbox,
    MultiStruct, MultiVec, PointArray, PointStruct, Poracle, Precision, SingleStruct, SingleVec,
    ToCollection, ToFeature, ToFeatureVec, ToGeometry, ToGeometryVec, ToMultiStruct, ToMultiVec,
    ToPointArray, ToPointStruct, ToPoracle, ToPoracleVec, ToSingleStruct, ToSingleVec, ToSql,
    ToText, ValueHelpers,
};
// Submodule paths consumers still use (model::api::single_vec::SingleVec, etc.):
pub mod single_vec { pub use koji_core::SingleVec; }
pub mod multi_vec { pub use koji_core::MultiVec; }
pub mod single_struct { pub use koji_core::SingleStruct; }
pub mod multi_struct { pub use koji_core::MultiStruct; }
pub mod point_array { pub use koji_core::PointArray; }
pub mod point_struct { pub use koji_core::PointStruct; }
pub mod poracle { pub use koji_core::Poracle; }
```
Keep `pub mod text;` (the trimmed RDM file). Keep the `db::{InstanceParsing, RdmInstanceArea}` import ONLY if `text.rs` still needs them via the glob — otherwise drop (they were unused in mod.rs per the map).

- [ ] **Step 2: Delete the 3 moved utils** from `crates/model/src/utils/mod.rs` (`TrimPrecision`, `sql_raw`, `sql_raw_bbox`) and re-export from core for back-compat: `pub use koji_core::{TrimPrecision, sql_raw, sql_raw_bbox};`. Update model-internal callers (e.g. db entities using `utils::sql_raw`) — compiler-driven.

---

### Task 6: Verify + commit

- [ ] **Step 1:** `cargo build --workspace` — fix compiler-named import/path issues (mostly `model::api::X` paths that the facade must cover; add missing re-exports).
- [ ] **Step 2:** `cargo test --workspace 2>&1 | tail -20` — expect the P1a counts (16 + 3 koji-core, 4 ignored) unchanged.
- [ ] **Step 3:** `cargo fmt -p koji-core && cargo fmt --check -p koji-core` clean.
- [ ] **Step 4:** Commit:
```bash
git add -A
git commit -m "refactor(core): move geometry + conversion layer from model::api into koji-core

Relocates the conversion traits, geometry types (Single/MultiVec, *Struct,
PointArray, PointStruct, Poracle), GeoFormats, BBox, Precision, and the pure
utils (TrimPrecision, sql_raw) into koji-core. model::api is now a re-export
facade (flat + submodule paths) so algorithms/api/bins are unchanged.
PointStruct is pure in core; model::db::LatLonRow carries the FromQueryResult
for the 12 scanner SELECT sites. text.rs split: pure String impls -> core,
RDM TextHelpers stays in model (removed in P6).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (§4):** ✅ geometry + conversions now in koji-core; ✅ model::api decoupled (facade only); ✅ PointStruct pure (sea-orm carve-out via LatLonRow); ✅ text RDM isolated. ⏭ remaining model::utils split (get_enum→ a db helper, name_modifier, get_database_struct) = P1c; ⏭ Args = P1d.

**Placeholder scan:** New code (`LatLonRow` + `From`, the facade re-exports, koji-core module wiring) is complete. Moves are verbatim (git mv); import fixes + the 12 query-site downstream conversions are compiler-driven sweeps (reliable for move-induced path changes), not placeholders. The geometry/mod.rs trait block is "moved verbatim from model api/mod.rs lines 26-234" — a literal relocation, not new content to invent.

**Type consistency:** Facade re-export list matches the consumer surface from the move-map (algorithms + api imports all covered: Precision, ToFeature, GetBbox, ToGeometry, ToSql, To{Multi,Single}{Struct,Vec}, ToPoracleVec, ToText, GeoFormats, + the type aliases + submodule paths). `LatLonRow` field names must be verified against the pre-move PointStruct (Task 3 Step 1 note).

**Risk:** Largest move so far. The facade may miss a re-export a consumer needs — Task 6 Step 1's workspace build is the safety net (every missing path errors with the exact name). `cargo test` must match P1a counts to prove behavior unchanged.
