# ApiQueryArgs Restructure — Design

- **Date:** 2026-06-14
- **Branch:** `claude/v2` (local, unpushed — rides the API security rework)
- **Status:** Approved-by-delegation (autonomous run; user picked Approach A + delegate mode, then went AFK). **The Assumptions section (§8) is the async review checkpoint.**
- **Follows:** the calc-`Args` restructure (`2026-06-14-args-restructure-design.md`, code-complete). This is its queued sibling — the *read*-filter god-struct.

## Problem

`ApiQueryArgs` (`crates/koji-core/src/query_args.rs`) is a flat 25-field god-struct threaded by reference into the geofence/route **read** endpoints (v1 + v2) and the koji-db query layer. One struct carries five unrelated concerns — name transforms, property inclusion, exclusion filters, hierarchy, and output flags. The mega-consumer `koji_db::geofence::to_feature` reads ~15 of its fields inline across a 120-line body; `name_modifier` is a free function taking the whole struct; one db method (`get_all_koji`) takes `&ApiQueryArgs` and never reads it.

This is the same class of mess the calc `Args` was — but a **read** path, and (critically) a different wire.

## The wire constraint (drives the whole design)

`ApiQueryArgs` is deserialized **only** via `web::Query<ApiQueryArgs>` (a flat query string), which actix backs with **`serde_urlencoded`**. `serde_urlencoded` does **not** support deserializing `#[serde(flatten)]` sub-structs. So — unlike the calc body, which we nested — the **wire must stay one flat struct**. Grouping the fields into nested wire sub-structs is not an option here.

**Therefore the grouping lives entirely in a resolve-to-domain layer**, not in the wire type. `ApiQueryArgs` stays a flat DTO (byte-identical query string, `rename_all = "camelCase"`, all current fields); it gains accessors that *resolve* its fields into focused domain value-types. Consumers take the domain types instead of `&ApiQueryArgs`. Zero wire change, all the cleanup value.

## Goals

- Stop threading the 25-field god-struct through koji-db + the handlers.
- Extract reusable, independently-testable domain value-types (especially `NameModifier`).
- Make `to_feature`'s five-concern body read from focused, named inputs.
- Remove dead weight (`get_all_koji`'s unused `&ApiQueryArgs`; investigate `geofence_id`).
- No wire change. No behavior change (returned Features byte-identical). No lingering debt.

## §1 — The domain value-types (koji-core)

New module `crates/koji-core/src/query/` (or extend `query_args.rs`); pure domain, no http/db:

```rust
/// The 12 name-transform flags + the canonical apply order. Extracted verbatim
/// from `text_utils::name_modifier`.
pub struct NameModifier {
    pub trimstart: Option<usize>, pub trimend: Option<usize>,
    pub alphanumeric: bool, pub replace: Option<String>,
    pub lowercase: bool, pub uppercase: bool, pub capfirst: bool,
    pub capitalize: Option<String>, pub underscore: Option<String>,
    pub dash: Option<String>, pub space: Option<String>, pub unpolish: bool,
}
impl NameModifier {
    /// Applies the transforms in EXACTLY this order (parity with the old fn):
    /// trimstart, trimend, alphanumeric, replace, lowercase, uppercase, capfirst,
    /// capitalize, underscore, dash, space, unpolish, then trim()+empty-guard
    /// (empty result → return the original, log warn).
    pub fn apply(&self, name: &str) -> String;
    pub fn is_noop(&self) -> bool; // all-default → skip the work
}

/// The comma-list exclusion filters (early-reject stage of `to_feature`).
pub struct Filters { exclude: Vec<String>, exclude_parents: Vec<String>, exclude_properties: Vec<String> }
impl Filters {
    /// True if this geofence should be DROPPED (name in `exclude`, OR parent name
    /// in `exclude_parents`, OR any property name in `exclude_properties`).
    pub fn rejects(&self, name: &str, parent_name: Option<&str>, property_names: &[&str]) -> bool;
}

/// Which properties to attach to the returned Feature.
pub struct PropertySelection { pub id: bool, pub name: bool, pub mode: bool,
    pub geofence_id: bool, pub parent: bool, pub group: bool }

/// Per-feature output flags `to_feature` reads (the `__`-prefix props + coord
/// precision). NOTE: the `rt` return-type is NOT here — it shapes the response
/// envelope, not individual features, so it stays the handler's existing
/// `get_return_type` call (see §3).
pub struct OutputSpec { pub internal: bool, pub fullcoords: bool }
```

`HierarchySpec` (already in koji-db, enum `Depth|Level`, `from_args` enforcing the depth-XOR-level 400) **stays put** — it's a DB-query concept; only the v2 handlers use it.

## §2 — The resolve layer (on `ApiQueryArgs`, koji-core)

```rust
impl ApiQueryArgs {
    pub fn name_modifier(&self) -> NameModifier;
    pub fn filters(&self) -> Filters;                 // comma-split via separate_by_comma
    pub fn property_selection(&self) -> PropertySelection;
    pub fn output_spec(&self) -> OutputSpec;          // internal + fullcoords only
}

/// The bundle `to_feature` consumes — resolved ONCE per request, borrowed per geofence.
pub struct FeatureRenderSpec {
    pub properties: PropertySelection,
    pub filters: Filters,
    pub name_modifier: NameModifier,
    pub output: OutputSpec,
}
impl ApiQueryArgs { pub fn feature_render_spec(&self) -> FeatureRenderSpec; }
```

`rt` is resolved separately by the handler (`get_return_type`, unchanged) and never enters `FeatureRenderSpec` — keeping the render bundle purely about per-feature rendering.

The flat DTO is the deserialization edge; the resolve methods are the only place fields are read. `text_utils::name_modifier(string, &ApiQueryArgs)` is replaced by `args.name_modifier().apply(&string)` at its single caller; the free fn (+ its private helpers `remove_symbols`, `convert_polish_to_ascii`) fold into `NameModifier`.

## §3 — Consumer refactor

| Consumer | Today | After |
|---|---|---|
| `koji_db::geofence::to_feature` (`:227`) | `(self, property_map, name_map, &ApiQueryArgs)` — reads 15 fields inline | `(self, property_map, name_map, &FeatureRenderSpec)` — reads the resolved groups; the 251-277 reject stage calls `spec.filters.rejects(...)`; the 279-326 prop stage reads `spec.properties`/`spec.output`; line 348 calls `spec.name_modifier.apply(...)` |
| `get_all_koji` (`:708`) | takes `&ApiQueryArgs`, never reads it | **drop the dead param** |
| `project_as_feature`/`project_as_koji` | pass `&ApiQueryArgs` to `to_feature` | resolve `FeatureRenderSpec` once, pass `&spec` |
| v1 `geofence.rs` handlers | pass `args` + read `rt` | build `FeatureRenderSpec` (+ rt) from the query, pass down |
| v1 `route.rs` handlers | `args` intentionally unused (wire compat) | unchanged — kept + documented (the query is accepted for back-compat) |
| v2 `geofences.rs`/`routes.rs` | `HierarchySpec::from_args` + `rt` | unchanged (hierarchy + rt resolution already focused) |

`HierarchySpec` and the `rt`→`get_return_type` path are already focused — left as-is.

## §4 — Data flow

```
GET ?id=true&lowercase=true&depth=2&exclude=foo
  → web::Query<ApiQueryArgs>            (FLAT DTO — unchanged, serde_urlencoded-safe)
  → handler: args.feature_render_spec()  +  HierarchySpec::from_args(&args)  +  get_return_type(rt, default)
  → koji-db query: project_as_koji(.., &FeatureRenderSpec, hierarchy)
  → to_feature(.., &spec): filters.rejects → properties → name_modifier.apply → output precision
  → byte-identical geojson Feature
```

## §5 — Co-location / homes

- `NameModifier`, `Filters`, `PropertySelection`, `OutputSpec`, `FeatureRenderSpec` → **koji-core** (pure domain; mirrors the calc configs).
- `ApiQueryArgs` (flat DTO) stays in koji-core `query_args.rs`.
- `HierarchySpec` stays in koji-db.
- `text_utils::name_modifier` is removed; its logic lives in `NameModifier::apply`.

## §6 — Testing

- **Wire parity:** an `ApiQueryArgs` deserializes from a representative flat query string identically to today (no field renamed/dropped); `rename_all = camelCase` preserved.
- **`NameModifier::apply` parity:** a table test pinning the 12-step order + the empty-guard (e.g. `trimstart` before `alphanumeric` before `lowercase`…), asserting byte-identical output to the old `name_modifier` for a spread of inputs.
- **`Filters::rejects` parity:** name/parent/property exclusion each reject correctly; comma-split honored.
- **`to_feature` golden parity:** the existing `project_as_koji_*` tests (geofence.rs:1450/1474) must pass unchanged — they are the end-to-end net that the resolved-bundle refactor preserves byte output.
- **Workspace gate** each sub-commit: `fmt` + `clippy` + `test` (host target — never `--target wasm32`).

## §7 — Non-goals

- **No wire change** — the flat query string + camelCase keys are frozen.
- **`AdminReq`** (sibling in `query_args.rs`) is out of scope — it already uses a `parse() → AdminReqParsed` pattern and isn't coupled to `ApiQueryArgs`.
- **No new validation semantics** — resolution reproduces the current behavior exactly (depth/level mutual-exclusion stays in `HierarchySpec::from_args`).
- **No `#[serde(flatten)]` on `ApiQueryArgs`** — unsupported by `serde_urlencoded`.

## §8 — Assumptions (DELEGATE CHECKPOINT — review these)

1. **Flat wire DTO, grouping in the resolve layer** — forced by `serde_urlencoded` (no flatten on `web::Query`). This revises the earlier "flatten sub-structs" idea: the wire struct stays flat; only the *resolved* domain types are grouped.
2. **Domain value-types live in koji-core**; **`HierarchySpec` stays in koji-db** (DB-query concept, v2-only).
3. **`to_feature` takes one resolved `FeatureRenderSpec`** (not the god-struct, not 4 separate group params).
4. **`get_all_koji`'s dead `&ApiQueryArgs` param is dropped.**
5. **`geofence_id`:** the audit did not find it read in `to_feature`. Assumption: **keep it on the wire** (back-compat, no query-string change) and include it in `PropertySelection`; if implementation confirms it is genuinely never consumed, leave the field present + flag it, but do NOT remove it from the wire. (Conservative — zero wire risk.)
6. **v1 `route.rs` args stay accepted-but-unused** for wire back-compat (documented), not deleted.
7. **`NameModifier::apply` preserves the exact 12-step order** + the empty-result-returns-original guard.
8. **`internal` + `fullcoords`** live in `OutputSpec`; `to_feature` reads `output.internal` for both the `__`-prefixed props and the coord-precision branch.
9. **`AdminReq` untouched** (out of scope).
10. **Parity, not improvement** — no field semantics change; the only behavior delta is internal structure.

## §9 — Task sequence (for the implementation plan)

1. **koji-core `NameModifier`** — type + `apply()` (verbatim 12-step parity) + `From<&ApiQueryArgs>`; fold in `remove_symbols`/`convert_polish_to_ascii`; repoint the single `name_modifier` caller; remove the free fn. Unit-test the order. Green.
2. **koji-core `Filters`/`PropertySelection`/`OutputSpec` + resolve accessors + `FeatureRenderSpec`** on `ApiQueryArgs`. Unit tests. Green.
3. **koji-db `to_feature` → `&FeatureRenderSpec`**; drop `get_all_koji` dead param; repoint `project_as_feature`/`project_as_koji`. Golden parity. Green.
4. **koji-service handlers** build `FeatureRenderSpec` and pass it down; v1/v2 read paths repointed; rt/hierarchy unchanged. Parity. Green.
5. **Verification gate** — workspace fmt/clippy/test; grep proof (no `&ApiQueryArgs` threaded past the resolve layer except the DTO itself); confirm `geofence_id` disposition; doc note.
