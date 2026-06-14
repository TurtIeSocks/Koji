# ApiQueryArgs Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split the flat 25-field `ApiQueryArgs` god-struct into koji-core domain value-types (`NameModifier`, `Filters`, `PropertySelection`, `OutputSpec`) via a resolve layer + a `FeatureRenderSpec` bundle that `to_feature` consumes — with the query-string wire byte-identical and behavior byte-identical.

**Architecture:** `ApiQueryArgs` stays a flat `web::Query` DTO (serde_urlencoded can't deserialize `#[serde(flatten)]`). Grouping lives only in the resolve layer. Each task is a green sub-commit (`fmt`+`clippy`+`test`, host target — **never** `--target wasm32`).

**Tech Stack:** Rust 2024, koji-core (domain), koji-db (query layer), koji-service (actix HTTP).

**Design reference:** `docs/superpowers/specs/2026-06-14-apiqueryargs-restructure-design.md`.

## ⚠️ Two parity landmines (read before Task 1)

1. **Name-mod bools are PRESENCE-triggered, not value-triggered.** The old `name_modifier` (text_utils.rs:89) does `if modifiers.alphanumeric.is_some()` / `lowercase.is_some()` / `uppercase.is_some()` / `capfirst.is_some()` / `unpolish.is_some()` — so `Some(false)` **still triggers** the transform. Only `trimstart`/`trimend`/`replace`/`capitalize`/`underscore`/`dash`/`space` use the inner value (`if let Some(_)`). The `From<&ApiQueryArgs> for NameModifier` MUST set those bool flags via `.is_some()`, NOT `.unwrap_or(false)`.
2. **`internal` is read three ways in `to_feature`** (geofence.rs): `args.internal.unwrap_or(false)` for the `__`-prefix props (line 279); `args.internal.is_some()` for the precision branch (330, OR'd with `fullcoords.is_some()`) and the feature-id tag (351). `OutputSpec` must preserve all three via named methods, not a single bool.

---

## Task 1: koji-core `NameModifier`

**Files:**
- Modify: `crates/koji-core/src/text_utils.rs` (extract `NameModifier`; make `name_modifier` a thin wrapper for now)
- Modify: `crates/koji-core/src/lib.rs` (export `NameModifier`)

- [ ] **Step 1: Write the failing parity test** in `text_utils.rs` `#[cfg(test)] mod tests`:

```rust
use crate::ApiQueryArgs;

fn args_with(json: serde_json::Value) -> ApiQueryArgs {
    serde_json::from_value(json).unwrap()
}

#[test]
fn name_modifier_presence_triggered_bools() {
    // lowercase: Some(false) STILL lowercases (is_some semantics, parity-critical)
    let nm = NameModifier::from(&args_with(serde_json::json!({ "lowercase": false })));
    assert_eq!(nm.apply("HeLLo"), "hello");
}

#[test]
fn name_modifier_value_triggered_and_order() {
    // trimstart then alphanumeric then space-replace, in order
    let nm = NameModifier::from(&args_with(serde_json::json!({
        "trimstart": 2, "alphanumeric": true, "space": "_"
    })));
    assert_eq!(nm.apply("XX a!b c"), "ab_c"); // drop "XX", strip "!", space->"_"
}

#[test]
fn name_modifier_empty_guard_returns_original() {
    let nm = NameModifier::from(&args_with(serde_json::json!({ "trimstart": 100 })));
    assert_eq!(nm.apply("short"), "short"); // empty result -> original
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p koji-core name_modifier_presence_triggered_bools` → `NameModifier` undefined.

- [ ] **Step 3: Implement `NameModifier`.** Add the struct + `From<&ApiQueryArgs>` + `apply`. The struct:
```rust
pub struct NameModifier {
    trimstart: Option<usize>, trimend: Option<usize>,
    alphanumeric: bool, replace: Option<String>,
    lowercase: bool, uppercase: bool, capfirst: bool,
    capitalize: Option<String>, underscore: Option<String>, dash: Option<String>,
    space: Option<String>, unpolish: bool,
}
```
`From<&ApiQueryArgs>`: bool fields via `.is_some()` (e.g. `alphanumeric: a.alphanumeric.is_some()`), Option fields cloned. `apply(&self, name: &str) -> String` reproduces text_utils.rs:90-162 EXACTLY (same 12-step order, `clean()` on replace/capitalize/underscore/dash/space, the trim()+empty-guard returning the original). Keep `remove_symbols`/`convert_polish_to_ascii`/`clean` where they are (same module). Make `name_modifier(string, args)` a one-line wrapper: `NameModifier::from(args).apply(&string)` (its single caller stays untouched until Task 3). Export `NameModifier` from `lib.rs`.

- [ ] **Step 4: Run, expect PASS** — `cargo test -p koji-core name_modifier`.

- [ ] **Step 5: fmt + commit**
```bash
cargo fmt -p koji-core && git add crates/koji-core
git commit -m "feat(core): extract NameModifier value-type (presence-triggered parity)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: koji-core `Filters` / `PropertySelection` / `OutputSpec` + resolve layer

**Files:**
- Modify: `crates/koji-core/src/query_args.rs` (add the domain types + resolve accessors + `FeatureRenderSpec`)
- Modify: `crates/koji-core/src/lib.rs` (exports)

- [ ] **Step 1: Write failing tests** in `query_args.rs`:

```rust
#[test]
fn filters_built_once_reject_predicates() {
    let a: ApiQueryArgs = serde_json::from_value(serde_json::json!({
        "exclude": "foo,bar", "excludeparents": "P", "excludeproperties": "secret"
    })).unwrap();
    let f = a.filters();
    assert!(f.excludes_name("foo"));
    assert!(!f.excludes_name("baz"));
    assert!(f.excludes_parent(Some("P")));
    assert!(f.excludes_any_property(&["secret", "ok"]));
}

#[test]
fn output_spec_internal_three_semantics() {
    let a: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "internal": false })).unwrap();
    let o = a.output_spec();
    assert!(!o.adds_internal_props());   // internal.unwrap_or(false) == false
    assert!(o.skip_precision_trim());    // internal.is_some() == true
    assert!(o.tag_internal_id());        // internal.is_some() == true
}

#[test]
fn property_selection_unwrap_or_false() {
    let a: ApiQueryArgs = serde_json::from_value(serde_json::json!({ "id": true })).unwrap();
    let p = a.property_selection();
    assert!(p.id && !p.name && !p.mode && !p.parent && !p.group);
}
```

- [ ] **Step 2: Run, expect FAIL** — `cargo test -p koji-core filters_built_once_reject_predicates`.

- [ ] **Step 3: Implement.** Add to `query_args.rs`:
```rust
pub struct Filters { exclude: Vec<String>, exclude_parents: Vec<String>, exclude_properties: Vec<String> }
impl Filters {
    pub fn excludes_name(&self, name: &str) -> bool { self.exclude.iter().any(|x| x == name) }
    pub fn excludes_parent(&self, parent: Option<&str>) -> bool {
        parent.is_some_and(|p| self.exclude_parents.iter().any(|x| x == p))
    }
    pub fn excludes_any_property(&self, prop_names: &[&str]) -> bool {
        prop_names.iter().any(|n| self.exclude_properties.iter().any(|x| x == n))
    }
}
pub struct PropertySelection { pub id: bool, pub name: bool, pub mode: bool, pub parent: bool, pub group: bool }
pub struct OutputSpec { internal: Option<bool>, fullcoords: Option<bool> }
impl OutputSpec {
    pub fn adds_internal_props(&self) -> bool { self.internal.unwrap_or(false) }     // line 279 parity
    pub fn skip_precision_trim(&self) -> bool { self.internal.is_some() || self.fullcoords.is_some() } // line 330
    pub fn tag_internal_id(&self) -> bool { self.internal.is_some() }                // line 351
}
pub struct FeatureRenderSpec {
    pub properties: PropertySelection, pub filters: Filters,
    pub name_modifier: NameModifier, pub output: OutputSpec,
}
impl ApiQueryArgs {
    pub fn filters(&self) -> Filters { /* separate_by_comma(&self.exclude) etc., parsed ONCE */ }
    pub fn property_selection(&self) -> PropertySelection { /* each .unwrap_or(false) */ }
    pub fn output_spec(&self) -> OutputSpec { OutputSpec { internal: self.internal, fullcoords: self.fullcoords } }
    pub fn feature_render_spec(&self) -> FeatureRenderSpec {
        FeatureRenderSpec { properties: self.property_selection(), filters: self.filters(),
            name_modifier: NameModifier::from(self), output: self.output_spec() }
    }
}
```
`PropertySelection` does NOT include `geofence_id` — the audit confirms `to_feature` never reads it; it stays on the `ApiQueryArgs` wire DTO (back-compat) but resolves nowhere. Add a doc comment on the field noting it's accepted-but-unused. Export `Filters`, `PropertySelection`, `OutputSpec`, `FeatureRenderSpec`.

- [ ] **Step 4: Run, expect PASS** — `cargo test -p koji-core query_args`.

- [ ] **Step 5: fmt + commit**
```bash
cargo fmt -p koji-core && git add crates/koji-core
git commit -m "feat(core): ApiQueryArgs resolve layer (Filters/PropertySelection/OutputSpec + FeatureRenderSpec)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Migrate consumers (koji-db `to_feature` + koji-service handlers)

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs` (`to_feature` → `&FeatureRenderSpec`; `project_as_feature`/`project_as_koji` resolve + pass; drop `get_all_koji` dead `&ApiQueryArgs` param)
- Modify: `crates/koji-core/src/text_utils.rs` (remove the `name_modifier` wrapper now its caller is gone)
- Modify: `crates/koji-service/src/public/v1/geofence.rs`, `public/v1/route.rs` (pass `FeatureRenderSpec` where `get_all_koji`/`project_as_koji` are called; drop the arg from the `get_all_koji` call)

This is the cohesive cross-crate consumer migration; do it as one green commit.

- [ ] **Step 1: Baseline** — `cargo test -p koji-db project_as_koji > /tmp/aqbase.log 2>&1; echo $?` (expect 0; these are the golden parity net: `project_as_koji_transform_preserves_id_and_name_props`, `project_as_koji_mode_preserved_with_props`).

- [ ] **Step 2: Rewrite `to_feature`** (geofence.rs:227). New signature `fn to_feature(self, property_map: &HashMap<u32, Vec<FullPropertyModel>>, name_map: &HashMap<u32, String>, spec: &FeatureRenderSpec) -> Result<Feature, ModelError>`. Body changes (preserve order + the three reject positions + messages):
  - line 251 `separate_by_comma(&args.exclude).contains(&self.name)` → `spec.filters.excludes_name(&self.name)`
  - line 254-259 excludeproperties → collect the prop names, `spec.filters.excludes_any_property(&names)`
  - line 271-277 excludeparents → `spec.filters.excludes_parent(parent_name.as_deref())`
  - lines 297/303/309/315/321 `args.name/id/mode/group/parent.unwrap_or(false)` → `spec.properties.name/id/mode/group/parent`
  - line 279 `args.internal.unwrap_or(false)` → `spec.output.adds_internal_props()`
  - line 330 `args.internal.is_some() || args.fullcoords.is_some()` → `spec.output.skip_precision_trim()`
  - line 348 `name_modifier(geofence_name.to_string(), args)` → `spec.name_modifier.apply(geofence_name)`
  - line 351 `args.internal.is_some()` → `spec.output.tag_internal_id()`

- [ ] **Step 3: Update `project_as_feature`/`project_as_koji`** — resolve `let spec = args.feature_render_spec();` ONCE before the per-geofence loop, pass `&spec` to each `to_feature`. (They keep receiving `&ApiQueryArgs` from the handlers; resolving inside keeps koji-service callers working without a signature change.) **Drop the unused `&ApiQueryArgs` param from `get_all_koji` (`:708`)** and remove the argument at its koji-service call sites (`v1/geofence.rs` `all()`/`specific_return_type` per the audit).

- [ ] **Step 4: Remove the `name_modifier` wrapper** from `text_utils.rs` (its only caller is now gone) + drop its `lib.rs` export. Build the workspace to catch any other caller: `cargo build --workspace`.

- [ ] **Step 5: Parity** — `cargo test -p koji-db project_as_koji` (PASS, identical) + `cargo test -p koji-service` (PASS). `cargo fmt --all`. `cargo clippy --workspace` (0 new).

- [ ] **Step 6: Commit**
```bash
git add -A
git commit -m "refactor(db,service): to_feature takes FeatureRenderSpec; drop get_all_koji dead arg; remove name_modifier wrapper

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Verification gate + docs

**Files:**
- Modify: any read-endpoint doc note if present (`grep -rln "ApiQueryArgs\|api/v1/geofence\|api/v2/geofences" docs/ crates/koji-service`)

- [ ] **Step 1: Full workspace gate (parallel)**:
```bash
cargo fmt --all --check; cargo clippy --workspace --all-targets; cargo test --workspace
```
Expected: fmt 0; clippy 0 (modulo the known third-party `num-bigint-dig`/`proc-macro-error2` note); test 0 failures.

- [ ] **Step 2: Grep proof**:
  - `grep -rn "name_modifier" crates/` → zero (folded into `NameModifier`).
  - `grep -rn "args: &ApiQueryArgs\|&ApiQueryArgs" crates/koji-db crates/koji-service` → `&ApiQueryArgs` appears only where the DTO is genuinely needed (the resolve accessors / handler entry), NOT threaded into `to_feature`/`get_all_koji`.
  - `grep -rn "geofence_id" crates/` → confirm it remains only on the `ApiQueryArgs` DTO (accepted-unused), nowhere resolved.

- [ ] **Step 3: Doc note** — add a short line wherever the read-filter args are documented (OpenAPI if it enumerates them) that the query-string wire is unchanged; the internal structure now resolves into `FeatureRenderSpec`. Keep it short.

- [ ] **Step 4: Commit**
```bash
git add -A
git commit -m "docs(apiqueryargs): note resolve-layer refactor; final verification gate

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Self-Review

**Spec coverage:** §1 domain types → Tasks 1 (NameModifier) + 2 (Filters/PropertySelection/OutputSpec). §2 resolve layer + FeatureRenderSpec → Task 2. §3 consumer refactor (to_feature, get_all_koji dead param, project_*) → Task 3. §5 homes (koji-core types, HierarchySpec stays koji-db, name_modifier removed) → Tasks 1-3. §6 testing (NameModifier order, Filters, golden) → per-task. §7 non-goals: AdminReq untouched (no task); wire unchanged (flat DTO, no task touches the struct fields). **Assumptions:** A5 refined — `geofence_id` confirmed unread, kept on wire, excluded from `PropertySelection` (Task 2 Step 3). **Type consistency:** `FeatureRenderSpec { properties, filters, name_modifier, output }` used identically in Task 2 (def) and Task 3 (consumed); `OutputSpec` method names (`adds_internal_props`/`skip_precision_trim`/`tag_internal_id`) consistent. **Placeholders:** resolve bodies reference exact old line numbers + the two parity landmines; tests are concrete. **Ordering:** each task green (NameModifier wrapper keeps Task 1-2 callers working; wrapper removed only in Task 3 once `to_feature` stops calling it).
