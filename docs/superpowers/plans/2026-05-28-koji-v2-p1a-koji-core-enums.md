# Koji V2 — Phase 1a: koji-core foundation + break the `Type` coupling

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create the `koji-core` crate, give it the domain enums (`FenceType`, `Category`, and the relocated strategy enums `ClusterMode`/`CalculationMode`/`SortBy`), and rewire the `model::api` conversion traits off the sea-orm `Type` enum onto `koji_core::FenceType` — bridged by `From` impls in `model::db`. After this, the api conversion layer is decoupled from sea-orm at the *signature* level; physically relocating it into `koji-core` is P1b.

**Architecture:** `koji-core` is pure (serde only, no sea-orm/http). `model` gains a dependency on `koji-core` and re-exports the relocated enums from `model::api` for zero churn in `algorithms`/`api`. The db layer owns the `Type ↔ FenceType` mapping, so sea-orm stays confined to `model::db`.

**Tech Stack:** Rust edition 2024, serde, sea-orm (confined to db).

**Why this order:** The investigator mapping showed `Type` (sea-orm) is a *parameter in the conversion trait definitions* (`model/src/api/mod.rs:33,42,54,84,92`; `feature.rs:33,97`; `geometry.rs:151,160,169`; `args.rs:436`). Moving the conversion files to a pure crate is impossible until that parameter type is core-owned. So we introduce `FenceType` first.

**Reference:** architecture spec §4; `crates/model/src/db/sea_orm_active_enums.rs` (the `Type`/`Category` source of truth).

---

## File structure

```
crates/koji-core/Cargo.toml                 (create)
crates/koji-core/src/lib.rs                  (create — module wiring + re-exports)
crates/koji-core/src/fence_type.rs           (create — FenceType)
crates/koji-core/src/category.rs             (create — Category)
crates/koji-core/src/cluster_mode.rs         (move from model/src/api/cluster_mode.rs)
crates/koji-core/src/calc_mode.rs            (move from model/src/api/calc_mode.rs)
crates/koji-core/src/sort_by.rs              (move from model/src/api/sort_by.rs)
crates/model/Cargo.toml                      (add koji-core dep)
crates/model/src/db/enum_bridge.rs           (create — From<Type>↔FenceType, Category bridges)
crates/model/src/db/mod.rs                   (declare enum_bridge module)
crates/model/src/api/mod.rs                  (re-export core enums; Type→FenceType in trait sigs)
crates/model/src/api/{cluster_mode,calc_mode,sort_by}.rs   (delete; now re-exported from core)
crates/model/src/api/{feature,geometry}.rs   (Type→FenceType in impls)
crates/model/src/api/args.rs                 (ArgsUnwrapped.mode: Type→FenceType)
```

---

### Task 1: Create the `koji-core` crate skeleton

**Files:**
- Create: `crates/koji-core/Cargo.toml`, `crates/koji-core/src/lib.rs`

- [ ] **Step 1: Write the crate manifest**

`crates/koji-core/Cargo.toml`:
```toml
[package]
name = "koji-core"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "koji_core"
path = "src/lib.rs"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 2: Write a placeholder lib.rs** (modules added in later tasks)

`crates/koji-core/src/lib.rs`:
```rust
//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod calc_mode;
mod category;
mod cluster_mode;
mod fence_type;
mod sort_by;

pub use calc_mode::CalculationMode;
pub use category::Category;
pub use cluster_mode::ClusterMode;
pub use fence_type::FenceType;
pub use sort_by::SortBy;
```

> The `mod` lines reference files created in Tasks 2–3; this file will not compile until those land. That's fine — it's committed together in Task 6.

- [ ] **Step 3: Verify the workspace sees the new member**

Run: `cargo metadata --format-version=1 --no-deps 2>/dev/null | grep -o '"name":"koji-core"'`
Expected: `"name":"koji-core"` (the root `members = ["crates/*", ...]` glob picks it up automatically; no root manifest edit needed).

---

### Task 2: Define `FenceType` and `Category` in koji-core

**Files:**
- Create: `crates/koji-core/src/fence_type.rs`, `crates/koji-core/src/category.rs`

- [ ] **Step 1: Write `FenceType`** (mirrors `db::Type` 1:1, same `string_value`s)

`crates/koji-core/src/fence_type.rs`:
```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Domain equivalent of the scanner `type` column. Mirrors the legacy
/// sea-orm `Type` enum variant-for-variant; the db layer bridges the two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FenceType {
    CirclePokemon,
    CircleSmartPokemon,
    CircleRaid,
    CircleSmartRaid,
    AutoQuest,
    CircleQuest,
    CircleStation,
    PokemonIv,
    Leveling,
    AutoPokemon,
    AutoTth,
    /// Only valid in the Kōji database.
    Unset,
}

impl FenceType {
    pub const fn as_str(self) -> &'static str {
        match self {
            FenceType::CirclePokemon => "circle_pokemon",
            FenceType::CircleSmartPokemon => "circle_smart_pokemon",
            FenceType::CircleRaid => "circle_raid",
            FenceType::CircleSmartRaid => "circle_smart_raid",
            FenceType::AutoQuest => "auto_quest",
            FenceType::CircleQuest => "circle_quest",
            FenceType::CircleStation => "circle_station",
            FenceType::PokemonIv => "pokemon_iv",
            FenceType::Leveling => "leveling",
            FenceType::AutoPokemon => "auto_pokemon",
            FenceType::AutoTth => "auto_tth",
            FenceType::Unset => "unset",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        Some(match s {
            "circle_pokemon" => FenceType::CirclePokemon,
            "circle_smart_pokemon" => FenceType::CircleSmartPokemon,
            "circle_raid" => FenceType::CircleRaid,
            "circle_smart_raid" => FenceType::CircleSmartRaid,
            "auto_quest" => FenceType::AutoQuest,
            "circle_quest" => FenceType::CircleQuest,
            "circle_station" => FenceType::CircleStation,
            "pokemon_iv" => FenceType::PokemonIv,
            "leveling" => FenceType::Leveling,
            "auto_pokemon" => FenceType::AutoPokemon,
            "auto_tth" => FenceType::AutoTth,
            "unset" => FenceType::Unset,
            _ => return None,
        })
    }
}

impl Serialize for FenceType {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for FenceType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        FenceType::from_str_opt(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown fence type: {s}")))
    }
}
```

- [ ] **Step 2: Write `Category`** (mirrors `db::Category` 1:1)

`crates/koji-core/src/category.rs`:
```rust
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Boolean,
    String,
    Number,
    Object,
    Array,
    Database,
    Color,
}

impl Category {
    pub const fn as_str(self) -> &'static str {
        match self {
            Category::Boolean => "boolean",
            Category::String => "string",
            Category::Number => "number",
            Category::Object => "object",
            Category::Array => "array",
            Category::Database => "database",
            Category::Color => "color",
        }
    }
    pub fn from_str_opt(s: &str) -> Option<Self> {
        Some(match s {
            "boolean" => Category::Boolean,
            "string" => Category::String,
            "number" => Category::Number,
            "object" => Category::Object,
            "array" => Category::Array,
            "database" => Category::Database,
            "color" => Category::Color,
            _ => return None,
        })
    }
}

impl Serialize for Category {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for Category {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Category::from_str_opt(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown category: {s}")))
    }
}
```

---

### Task 3: Relocate the three strategy enums into koji-core

**Files:**
- Move: `crates/model/src/api/{cluster_mode,calc_mode,sort_by}.rs` → `crates/koji-core/src/`
- Modify: `crates/model/src/api/mod.rs` (drop the `mod` decls, re-export from core)

- [ ] **Step 1: Move the three files**

```bash
cd /Users/rin/GitHub/Koji/.claude/worktrees/goofy-tu-5372ed
git mv crates/model/src/api/cluster_mode.rs crates/koji-core/src/cluster_mode.rs
git mv crates/model/src/api/calc_mode.rs    crates/koji-core/src/calc_mode.rs
git mv crates/model/src/api/sort_by.rs      crates/koji-core/src/sort_by.rs
```

- [ ] **Step 2: Fix their imports for the new home**

In each moved file, replace any `use super::*;` / `use crate::api::...` with explicit `use serde::...` imports (they only need serde). Concretely:
- `cluster_mode.rs` line 1 stays `use serde::Deserialize;` (already correct).
- `sort_by.rs` line 1 stays `use serde::Deserialize;` (already correct).
- `calc_mode.rs` line 1: change `use super::*;` to `use serde::Deserialize;`.

- [ ] **Step 3: In `crates/model/src/api/mod.rs`, drop the three `mod` declarations and re-export from core**

Find and remove the lines declaring these modules (`mod cluster_mode;`, `mod calc_mode;`, `mod sort_by;` and their `pub use cluster_mode::*;` style re-exports). Replace with:
```rust
// Strategy enums now live in koji-core; re-exported here for back-compat.
pub use koji_core::{CalculationMode, ClusterMode, SortBy};
```
> This keeps every existing `use model::api::ClusterMode` (in `algorithms`, `api`) working unchanged.

---

### Task 4: Wire `model` to depend on `koji-core` + add the enum bridges

**Files:**
- Modify: `crates/model/Cargo.toml`
- Create: `crates/model/src/db/enum_bridge.rs`
- Modify: `crates/model/src/db/mod.rs` (declare the module)

- [ ] **Step 1: Add the dependency**

In `crates/model/Cargo.toml` `[dependencies]`, add:
```toml
koji-core = { path = "../koji-core" }
```

- [ ] **Step 2: Write the `Type ↔ FenceType` + `Category` bridges**

`crates/model/src/db/enum_bridge.rs`:
```rust
//! Boundary mapping between the sea-orm generated enums (`db::Type`,
//! `db::Category`) and the pure domain enums in koji-core. Keeps sea-orm
//! confined to the db layer.
use super::sea_orm_active_enums::{Category as DbCategory, Type as DbType};
use koji_core::{Category, FenceType};

impl From<DbType> for FenceType {
    fn from(t: DbType) -> Self {
        match t {
            DbType::CirclePokemon => FenceType::CirclePokemon,
            DbType::CircleSmartPokemon => FenceType::CircleSmartPokemon,
            DbType::CircleRaid => FenceType::CircleRaid,
            DbType::CircleSmartRaid => FenceType::CircleSmartRaid,
            DbType::AutoQuest => FenceType::AutoQuest,
            DbType::CircleQuest => FenceType::CircleQuest,
            DbType::CircleStation => FenceType::CircleStation,
            DbType::PokemonIv => FenceType::PokemonIv,
            DbType::Leveling => FenceType::Leveling,
            DbType::AutoPokemon => FenceType::AutoPokemon,
            DbType::AutoTth => FenceType::AutoTth,
            DbType::Unset => FenceType::Unset,
        }
    }
}

impl From<FenceType> for DbType {
    fn from(t: FenceType) -> Self {
        match t {
            FenceType::CirclePokemon => DbType::CirclePokemon,
            FenceType::CircleSmartPokemon => DbType::CircleSmartPokemon,
            FenceType::CircleRaid => DbType::CircleRaid,
            FenceType::CircleSmartRaid => DbType::CircleSmartRaid,
            FenceType::AutoQuest => DbType::AutoQuest,
            FenceType::CircleQuest => DbType::CircleQuest,
            FenceType::CircleStation => DbType::CircleStation,
            FenceType::PokemonIv => DbType::PokemonIv,
            FenceType::Leveling => DbType::Leveling,
            FenceType::AutoPokemon => DbType::AutoPokemon,
            FenceType::AutoTth => DbType::AutoTth,
            FenceType::Unset => DbType::Unset,
        }
    }
}

impl From<DbCategory> for Category {
    fn from(c: DbCategory) -> Self {
        match c {
            DbCategory::Boolean => Category::Boolean,
            DbCategory::String => Category::String,
            DbCategory::Number => Category::Number,
            DbCategory::Object => Category::Object,
            DbCategory::Array => Category::Array,
            DbCategory::Database => Category::Database,
            DbCategory::Color => Category::Color,
        }
    }
}
impl From<Category> for DbCategory {
    fn from(c: Category) -> Self {
        match c {
            Category::Boolean => DbCategory::Boolean,
            Category::String => DbCategory::String,
            Category::Number => DbCategory::Number,
            Category::Object => DbCategory::Object,
            Category::Array => DbCategory::Array,
            Category::Database => DbCategory::Database,
            Category::Color => DbCategory::Color,
        }
    }
}
```

- [ ] **Step 3: Declare the module in `crates/model/src/db/mod.rs`**

Add near the other module declarations:
```rust
mod enum_bridge;
```

---

### Task 5: Rewire the conversion layer from `Type` to `FenceType`

This is the core of P1a. The conversion traits take a fence-type parameter; switch that parameter from sea-orm `db::Type` to `koji_core::FenceType`, and map at db call sites.

**Files:**
- Modify: `crates/model/src/api/mod.rs`, `feature.rs`, `geometry.rs`, `args.rs`
- Modify: db call sites that pass a `Type` into a conversion (found via grep in Step 4)

- [ ] **Step 1: `crates/model/src/api/mod.rs` — trait signatures**

At the top, replace the coupled import:
```rust
// REMOVE: use super::{db::{sea_orm_active_enums::Type, InstanceParsing, RdmInstanceArea}, ...};
// Keep the InstanceParsing/RdmInstanceArea imports if still referenced here (RDM, removed in P6);
// drop only `sea_orm_active_enums::Type`. Add:
use koji_core::FenceType;
```
Then in every trait method signature currently taking `Option<Type>` / `Type` (lines 33, 42, 54, 84, 92 — traits `EnsureProperties`, `ValueHelpers`, `FeatureHelpers`, `ToFeature`, `ToCollection`), replace `Type` with `FenceType`. Example:
```rust
// before: fn to_feature(self, enum_type: Option<Type>) -> Feature;
// after:  fn to_feature(self, enum_type: Option<FenceType>) -> Feature;
```

- [ ] **Step 2: `feature.rs` — drop `sea_orm::ActiveEnum`, use FenceType methods**

Remove `use sea_orm::ActiveEnum;` (line 3). In the impls at lines 33 and 97 that take/handle `Type`, change the parameter to `FenceType` and replace any `enum_type.to_value()` / `Type::try_from` calls with `fence_type.as_str()` (FenceType's plain method). Add `use koji_core::FenceType;`.

- [ ] **Step 3: `geometry.rs` — Type in match → FenceType**

At lines 151, 160, 169, the `Type` references in the `to_feature` match become `FenceType` (variant names are identical, so the match arms are unchanged apart from the path). Add `use koji_core::FenceType;`; the impl signatures already inherit the trait's new `FenceType` param.

- [ ] **Step 4: Find + fix db call sites passing `Type` into conversions**

Run: `rg -n "to_feature\(|to_collection\(|add_instance_properties\(|set_property" crates/model/src/db crates/api/src crates/algorithms/src`
For each call that passes a `db::Type` value (e.g. `area.rs`, `instance.rs`, `geofence.rs` building features with a mode), append `.into()` to convert `Type → FenceType` at the boundary. Example:
```rust
// before: feat.to_feature(Some(Type::CirclePokemon))
// after:  feat.to_feature(Some(FenceType::CirclePokemon))   // if a literal
// before: feat.to_feature(Some(model.r#type.clone()))       // a db Type value
// after:  feat.to_feature(Some(model.r#type.clone().into()))
```
> The compiler is the checklist here: after Steps 1–3, every remaining call site that passes a `Type` will fail to typecheck. Fix each with `.into()` (db value) or the `FenceType` literal.

- [ ] **Step 5: `args.rs` — `ArgsUnwrapped.mode` field type**

At `args.rs:436`, change `pub mode: Type,` to `pub mode: FenceType,`. Add `use koji_core::FenceType;`. Update the place in `Args::init` that assigns `mode` (it sources from `get_enum(...)`, which returns a `db::Type`) to append `.into()`. (The full `Args` breakdown is P1d; this is the minimal change to keep the type flowing.)

> Note: `get_enum`/`get_enum_by_geometry_string` in `utils/mod.rs` return `db::Type`. Leave them returning `Type` for now (they're db-bound helpers); map to `FenceType` at the call site with `.into()`. Their relocation is P1c.

---

### Task 6: Verify + commit

- [ ] **Step 1: Build the workspace**

Run: `cargo build --workspace` (background if slow).
Expected: success. If a `Type`/`FenceType` mismatch remains, the error names the exact file:line — apply `.into()` or swap the literal per Task 5 Step 4.

- [ ] **Step 2: Test**

Run: `cargo test --workspace 2>&1 | tail -20`
Expected: same pass/fail counts as the P0 baseline (clean, 0 failures).

- [ ] **Step 3: Lint**

Run: `cargo clippy --workspace --all-targets`
Expected: no new errors.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor(core): introduce koji-core with domain enums; decouple conversion layer from sea-orm Type

Adds koji-core (pure) with FenceType, Category, and the relocated
ClusterMode/CalculationMode/SortBy strategy enums (re-exported from
model::api for back-compat). Conversion traits now take koji_core::FenceType
instead of sea-orm db::Type; model::db bridges the two via From impls.
sea-orm is now confined to the db layer at the conversion boundary.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage (architecture spec §4):** ✅ koji-core created (pure); ✅ domain enums defined; ✅ sea-orm `Type` coupling broken in the conversion layer (the §4 "break the db↔api coupling" goal, first slice); ⏭ physical relocation of geometry/conversions = P1b; ⏭ `Args` full breakdown = P1d; ⏭ `koji-db`/`koji-scanner` = P1c.

**Placeholder scan:** `FenceType`, `Category`, and all four `From` impls are complete. Trait-sig + call-site changes are located by exact file:line and (Task 5 Step 4) by a compiler-driven sweep — the standard, reliable way to find every site in a rename-style change; not a placeholder.

**Type consistency:** `FenceType`/`Category` variant sets match `db::Type`/`db::Category` exactly (verified against `sea_orm_active_enums.rs:8-35,61-76`). `as_str()` strings match the sea-orm `string_value`s. The `From` impls are total (every variant mapped both directions).

**Risk notes:** `PointStruct`'s `FromQueryResult` derive and `text.rs`'s RDM `InstanceParsing`/`RdmInstanceArea` usage are intentionally untouched here (deferred to P1b / P6) — P1a leaves those files in `model` and does not move query-result types into the pure crate.
