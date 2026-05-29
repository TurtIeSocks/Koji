# Koji V2 — Phase 1-conv: conversion-layer modernization

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or superpowers:executing-plans. Steps use checkbox (`- [ ]`).

**Goal:** Modernize the `koji-core` conversion layer: bundle the param-carrying conversions behind a `FeatureCtx`, collapse the repetitive wrapper impls with a `wrapper_conversions!` macro, and use `From`/`Into` where the target is a local type. Geometry types stay aliases (decided) → no hot-path churn in `algorithms`.

**Architecture:** `FeatureCtx { name, fence_type }` replaces the awkward `(Option<String>, Option<FenceType>)` arg pairs on `to_feature`/`to_collection`/`ensure_properties`/`add_instance_properties`. A declarative `wrapper_conversions!` macro generates the 4 identical wrapper conversions (`to_multi_vec`, `to_multi_struct`, `to_collection`, `to_poracle`) for the single-feature types from their hand-written bases (`to_single_vec`/`to_struct`/`to_single_struct`/`to_feature`). `From`/`Into` is added only where the *target* is local (`PointStruct`) — the orphan rule blocks it for the `Vec`-alias types (that's why the `To*` traits exist; they stay).

**Tech Stack:** Rust 2024, koji-core, koji-macros (declarative macro).

**Scope (surveyed):** 80 param-carrying call sites across 26 files (FeatureCtx migration, compiler-driven); ~55 `To*` impls, of which the 4-per-type wrappers (~20 impls across 5 single-feature types) collapse into the macro.

**Reference:** the orphan-rule constraint (geometry aliases can't take `From`); `point_array.rs` as the canonical wrapper pattern.

---

### Task 1: `FeatureCtx` + migrate the param-carrying trait signatures

**Files:** `crates/koji-core/src/geometry/mod.rs`, `crates/koji-core/src/feature_ctx.rs` (new), `lib.rs`

- [ ] **Step 1: Create `crates/koji-core/src/feature_ctx.rs`**
```rust
use crate::FenceType;

/// Bundles the optional metadata applied when building a GeoJSON Feature /
/// FeatureCollection — replaces the old `(Option<String>, Option<FenceType>)`
/// argument pairs.
#[derive(Debug, Default, Clone)]
pub struct FeatureCtx {
    pub name: Option<String>,
    pub fence_type: Option<FenceType>,
}

impl FeatureCtx {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
    pub fn with_type(mut self, fence_type: FenceType) -> Self {
        self.fence_type = Some(fence_type);
        self
    }
}
```
Add `mod feature_ctx; pub use feature_ctx::FeatureCtx;` to `lib.rs`.

- [ ] **Step 2: Change the four trait signatures** in `geometry/mod.rs`:
```rust
pub trait ToFeature       { fn to_feature(self, ctx: &FeatureCtx) -> Feature; }
pub trait ToCollection    { fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection; }
pub trait EnsureProperties { fn ensure_properties(self, ctx: &FeatureCtx) -> Self; }
pub trait FeatureHelpers {
    fn add_instance_properties(&mut self, ctx: &FeatureCtx);
    fn remove_last_coord(self) -> Self;
    fn remove_internal_props(self) -> Self;
}
```
Add `use crate::FeatureCtx;` to `geometry/mod.rs`.

- [ ] **Step 3: Update the conversion impls in koji-core** (feature.rs, geometry.rs, collection.rs, single_vec.rs, multi_vec.rs, single_struct.rs, multi_struct.rs, point_array.rs, point_struct.rs, poracle.rs, text.rs) so each `to_feature(self, enum_type)` → `to_feature(self, ctx: &FeatureCtx)`, reading `ctx.fence_type` / `ctx.name` instead of the old params. (Most `to_feature` bodies use `enum_type` once for `get_geojson_value`; replace with `ctx.fence_type`.) Build `cargo build -p koji-core` and fix each impl the compiler flags.

---

### Task 2: Migrate the 80 call sites to FeatureCtx (compiler-driven)

**Files:** all 26 files from the survey (koji-core/geometry/*, model/{api,db}/*, api/*, algorithms/bootstrap/*).

- [ ] **Step 1: Build the workspace** — `cargo build --workspace`. Every call site now mismatches. Apply the mechanical translation the compiler points to:
  - `x.to_feature(None)` → `x.to_feature(&FeatureCtx::new())`
  - `x.to_feature(Some(ft))` → `x.to_feature(&FeatureCtx::new().with_type(ft))`
  - `x.to_collection(None, None)` → `x.to_collection(&FeatureCtx::new())`
  - `x.to_collection(Some(name), Some(ft))` → `x.to_collection(&FeatureCtx::new().with_name(name).with_type(ft))`
  - `feat.add_instance_properties(name, ft)` → `feat.add_instance_properties(&FeatureCtx{ name, fence_type: ft })`
  Repeat build→fix until green. (Internal koji-core wrapper impls are handled by the macro in Task 3, so fix non-macro sites here.)

- [ ] **Step 2: Spot-check** the hottest call sites (calculate.rs, geofence.rs, route.rs) read naturally with the builder form.

---

### Task 3: `wrapper_conversions!` macro

**Files:** `crates/koji-core/src/geometry/mod.rs` (macro), the 5 single-feature impl files.

- [ ] **Step 1: Define the macro** (declarative; in `geometry/mod.rs` or a `conversions_macro.rs`):
```rust
/// Generates the four conversions that are identical for every single-feature
/// type, from its hand-written `to_single_vec` / `to_single_struct` / `to_feature`.
macro_rules! wrapper_conversions {
    ($t:ty) => {
        impl ToMultiVec for $t {
            fn to_multi_vec(self) -> MultiVec { vec![self.to_single_vec()] }
        }
        impl ToMultiStruct for $t {
            fn to_multi_struct(self) -> MultiStruct { vec![self.to_single_struct()] }
        }
        impl ToCollection for $t {
            fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection {
                let feature = self.to_feature(ctx);
                FeatureCollection {
                    bbox: feature.bbox.clone(),
                    features: vec![feature],
                    foreign_members: None,
                }
            }
        }
        impl ToPoracle for $t {
            fn to_poracle(self) -> Poracle {
                Poracle { path: Some(self.to_single_vec()), ..Default::default() }
            }
        }
    };
}
```

- [ ] **Step 2: Apply + delete the hand-written wrappers.** In each of `point_array.rs`, `point_struct.rs`, `single_vec.rs`, `single_struct.rs`, `text.rs` (String): delete the hand-written `impl ToMultiVec`/`ToMultiStruct`/`ToCollection`/`ToPoracle` blocks and add `wrapper_conversions!(TypeName);`. Keep the hand-written `to_single_vec`/`to_struct`/`to_single_struct`/`to_feature`/`to_text`/`to_point_array`. **Before deleting each, confirm its body matches the macro output verbatim** (point_array.rs confirmed; verify the other 4 — if a body differs, leave it hand-written and note why).

- [ ] **Step 3: Build + test** `cargo test -p koji-core` — the 3 enum tests + any conversion tests pass; behavior identical.

---

### Task 4: `From`/`Into` for local-target conversions

**Files:** `point_struct.rs`

- [ ] **Step 1:** Replace `impl ToPointStruct for PointArray`/`SingleVec` element conversions with idiomatic `From` where the target is the local `PointStruct`:
```rust
impl From<PointArray> for PointStruct {
    fn from(p: PointArray) -> Self { PointStruct { lat: p[0], lon: p[1] } }
}
```
Keep `ToPointStruct` as the trait for the alias types (orphan rule blocks `From<X> for Vec<…>`). This is the *only* clean `From` win — document that the rest stay traits by necessity, not preference.

> Scope honesty: the orphan rule limits `From`/`Into` to local targets (`PointStruct`). The bulk of the modernization value is `FeatureCtx` (Task 1-2) + the macro (Task 3), not `From`/`Into`.

---

### Task 5: Verify + commit

- [ ] **Step 1:** `cargo build --workspace` green.
- [ ] **Step 2:** `cargo test --workspace 2>&1 | tail -20` — counts identical to P1b (16+3 pass, 4+1 ignored).
- [ ] **Step 3:** `rustfmt --edition 2024` the new/changed koji-core files.
- [ ] **Step 4:** Commit:
```bash
git add -A
git commit -m "refactor(core): modernize conversion layer — FeatureCtx + wrapper_conversions! macro

- FeatureCtx bundles (name, fence_type) for to_feature/to_collection/
  ensure_properties/add_instance_properties (80 call sites migrated).
- wrapper_conversions! macro collapses the 4 identical wrapper impls
  (to_multi_vec/to_multi_struct/to_collection/to_poracle) across the 5
  single-feature types.
- From<PointArray> for PointStruct (the one orphan-rule-allowed From win).
Geometry types stay aliases (no algorithm churn); behavior unchanged.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:** ✅ FeatureCtx (param bundling); ✅ macro for repetitive wrappers (your macros-first instinct); ✅ From/Into where the orphan rule allows; ✅ aliases kept → no hot-path churn. Honest scope note: `From`/`Into` is limited to local targets by the orphan rule (explained to user pre-plan).

**Placeholder scan:** `FeatureCtx`, the trait sig changes, and `wrapper_conversions!` are complete. Task 2 (80 sites) + Task 1 Step 3 (impl bodies) are compiler-driven sweeps — the proven pattern, not placeholders. Task 3 Step 2 requires verifying 4 impl bodies match the macro before deleting (explicit guard against silent behavior change).

**Risk:** Largest is the 80-site FeatureCtx migration churn (mechanical, compiler-driven). The macro could mask a body that *almost* matches — Task 3 Step 2's "confirm verbatim before delete" guards it. `cargo test` parity with P1b is the behavioral safety net.
