# Args Restructure Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Dissolve the `model` crate, split the fat calc `Args` into per-operation nested request types (wire-layer in koji-service that `resolve()`s into the pure-domain koji-core config structs), keep v1 as a frozen shim, and kill the worst primitive-arg chains.

**Architecture:** Two-layer. koji-service owns `Option`-field wire structs (`ClusteringArgs`, …) that resolve into koji-core's existing `ClusteringConfig`/`RoutingConfig`/`BootstrapConfig`/`DataFilter`/`OutputConfig`/`DevConfig` (which stay serde-free). Per-op requests compose only the groups they use; `/api/v2/calc` takes an internally-tagged `CalcRequest`. v1 keeps its flat wire via a frozen `LegacyArgs` that funnels through the same resolution helpers. Each task is a green sub-commit (`fmt`+`clippy`+`test` on the host target — **never** `--target wasm32`).

**Tech Stack:** Rust 2024, actix-web, serde, koji-core (domain), koji-service (HTTP), algorithms.

**Design reference:** `docs/superpowers/specs/2026-06-14-args-restructure-design.md` — especially the **§2 resolution-defaults table**, which is the parity contract for every `resolve()` body. Read it before Task 2.

---

## File Structure (decomposition)

- `crates/koji-core/src/return_type.rs` — gains `get_return_type` (Task 1).
- `crates/koji-service/src/requests/mod.rs` — new module root (Task 2a).
- `crates/koji-service/src/requests/inputs.rs` — `GeoInput`, `DataPointsArg` + their conversions (moved from `model`) (Task 2a).
- `crates/koji-service/src/requests/resolve.rs` — shared default constants + `validate_s2_cell`, `resolve_data_points`, area→`default_return_type`, plugin-arg string-build (Task 2a).
- `crates/koji-service/src/requests/groups.rs` — `ClusteringArgs`/`RoutingArgs`/`BootstrapArgs`/`DataFilterArgs`/`OutputArgs`/`DevArgs` + `resolve()` each (Task 2a).
- `crates/koji-service/src/requests/ops.rs` — `CalcRequest` (tagged) + `ClusterReq`/`RerouteReq`/`BootstrapReq`/`StatsReq` + geo `ConvertReq`/`SimplifyReq`/`MergePointsReq` (Task 2b).
- `crates/koji-service/src/public/v2/{calc,jobs,geo}.rs` — rewired onto the requests (Task 3).
- `crates/koji-service/src/public/v1/legacy.rs` — frozen `LegacyArgs`/`LegacyResolved` (moved fat `Args`) (Task 4).
- `crates/koji-service/src/private/auth.rs` — `Auth`, `Search` (moved) (Task 4).
- `crates/model/` — **deleted** (Task 5).
- `crates/algorithms/src/clustering/crucible/refine.rs` — `for_each_in_bbox` folds to `KojiBbox` (Task 6).

---

## Task 1: Move `get_return_type` into koji-core

**Files:**
- Modify: `crates/koji-core/src/return_type.rs` (add the fn + a test)
- Modify: `crates/koji-core/src/lib.rs:28` (export)
- Modify: `crates/model/src/api/args.rs` (delete the fn def; use `koji_core::get_return_type` at its internal caller ~line 425)
- Modify importers: `crates/koji-service/src/public/v1/route.rs:9`, `public/v1/geofence.rs:9`, `public/v2/routes.rs:21`, `public/v2/geofences.rs:22`

- [ ] **Step 1: Write the failing test** in `crates/koji-core/src/return_type.rs` (append to/﻿create a `#[cfg(test)] mod tests`):

```rust
#[test]
fn get_return_type_maps_aliases_and_falls_back() {
    use super::{ReturnTypeArg, get_return_type};
    assert_eq!(get_return_type("feature_collection".into(), &ReturnTypeArg::SingleArray), ReturnTypeArg::FeatureCollection);
    assert_eq!(get_return_type("alt-text".into(), &ReturnTypeArg::SingleArray), ReturnTypeArg::AltText);
    // unknown string falls back to the provided default
    assert_eq!(get_return_type("nonsense".into(), &ReturnTypeArg::Feature), ReturnTypeArg::Feature);
}
```

- [ ] **Step 2: Run it, expect FAIL** (`get_return_type` not yet in this module):

Run: `cargo test -p koji-core get_return_type_maps_aliases_and_falls_back`
Expected: FAIL — `cannot find function get_return_type`.

- [ ] **Step 3: Move the function.** Cut the `pub fn get_return_type(return_type: String, default_return_type: &ReturnTypeArg) -> ReturnTypeArg { … }` body verbatim from `crates/model/src/api/args.rs` (currently ~line 506) into `crates/koji-core/src/return_type.rs`. Add `pub use return_type::{ReturnTypeArg, get_return_type};` to `crates/koji-core/src/lib.rs:28` (replace the existing `pub use return_type::ReturnTypeArg;`).

- [ ] **Step 4: Repoint callers.** In `model/src/api/args.rs`, delete the moved fn and change its internal call site (~line 425) to `koji_core::get_return_type(...)`. In the 4 koji-service importers, drop `get_return_type` from the `use model::api::args::{…}` lists and add `use koji_core::get_return_type;` (or fold into an existing `use koji_core::{…}`).

- [ ] **Step 5: Run tests + build**:

Run: `cargo test -p koji-core get_return_type_maps_aliases_and_falls_back && cargo build -p koji-service -p model`
Expected: PASS; both build.

- [ ] **Step 6: Commit**

```bash
git add crates/koji-core crates/model crates/koji-service
git commit -m "refactor(core): move get_return_type into koji-core beside ReturnTypeArg"
```

---

## Task 2a: koji-service `requests/` — inputs, resolve helpers, arg-groups

**Files:**
- Create: `crates/koji-service/src/requests/mod.rs`, `inputs.rs`, `resolve.rs`, `groups.rs`
- Modify: `crates/koji-service/src/lib.rs` (add `pub mod requests;`)
- Modify: `crates/koji-service/Cargo.toml` (ensure `geojson`, `log` available — they already are via existing deps)

Read the spec's **§2 defaults table** first; every default below is sourced from it.

- [ ] **Step 1: Move the wire inputs.** Create `requests/inputs.rs` and move `GeoInput` (+ `to_koji`, `geometry_to_koji`) and `DataPointsArg` verbatim from `model/src/api/args.rs`. Keep their derives (`Deserialize, Serialize, Clone`, `#[serde(untagged)]`). Add `mod inputs; pub use inputs::{GeoInput, DataPointsArg};` to `requests/mod.rs`.

- [ ] **Step 2: Move the resolution helpers.** Create `requests/resolve.rs`; move `validate_s2_cell` and `resolve_data_points` verbatim from `model`. Add a `pub(crate)` constants block + helpers used by the groups:

```rust
// requests/resolve.rs
pub(crate) const DEFAULT_RADIUS: f64 = 70.0;
pub(crate) const DEFAULT_S2_LEVEL: u8 = 15;
pub(crate) const DEFAULT_S2_SIZE: u8 = 9;
pub(crate) const DEFAULT_MIN_POINTS: usize = 1;

/// `None`/`0` → `usize::MAX`, else the value (parity with old `init()`).
pub(crate) fn resolve_max_clusters(v: Option<usize>) -> usize {
    match v { Some(0) | None => usize::MAX, Some(n) => n }
}

/// Appends the clustering plugin-arg tail exactly as the old `init()` did.
pub(crate) fn clustering_plugin_args(base: Option<String>, radius: f64, min_points: usize, max_clusters: usize) -> String {
    let mut s = base.unwrap_or_default();
    s += &format!(" --radius {radius}");
    s += &format!(" --min_points {min_points}");
    s += &format!(" --max_clusters {max_clusters}");
    s
}

/// Appends the bootstrap plugin-arg tail exactly as the old `init()` did.
pub(crate) fn bootstrap_plugin_args(base: Option<String>, radius: f64) -> String {
    let mut s = base.unwrap_or_default();
    s += &format!(" --radius {radius}");
    s
}
```

- [ ] **Step 3: Write the failing arg-group parity test** in `requests/groups.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clustering_args_resolve_matches_legacy_defaults() {
        // All-None wire -> the documented defaults + the plugin-arg tail.
        let cfg = ClusteringArgs::default().resolve();
        assert_eq!(cfg.radius, 70.0);
        assert_eq!(cfg.min_points, 1);
        assert_eq!(cfg.max_clusters, usize::MAX);
        assert_eq!(cfg.mode, koji_core::ClusterMode::Balanced);
        assert_eq!(cfg.calculation_mode, koji_core::CalculationMode::Radius);
        assert_eq!(cfg.s2.level, 15);
        assert_eq!(cfg.s2.size, 9);
        assert_eq!(cfg.plugin_args, " --radius 70 --min_points 1 --max_clusters 18446744073709551615");
    }
    #[test]
    fn nested_wire_deserializes_into_group() {
        let g: ClusteringArgs = serde_json::from_str(r#"{"radius":50,"minPoints":3}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.radius, 50.0);
        assert_eq!(cfg.min_points, 3);
    }
}
```

- [ ] **Step 4: Run it, expect FAIL**:

Run: `cargo test -p koji-service clustering_args_resolve`
Expected: FAIL — `ClusteringArgs` undefined.

- [ ] **Step 5: Implement the groups.** In `requests/groups.rs` define the six `Option`-field wire structs (camelCase, `#[serde(default)]`) and their `resolve()` → koji-core config. Field lists mirror the configs (see spec §2 sketch); use the `resolve.rs` helpers + the defaults table. Signatures:

```rust
#[derive(Debug, Default, Deserialize, Serialize)] #[serde(rename_all = "camelCase", default)]
pub struct ClusteringArgs { /* radius, min_points, max_clusters, mode, calculation_mode,
    s2_level, s2_size, cluster_split_level, center_clusters, genetic_post_processing, plugin_args */ }
impl ClusteringArgs { pub fn resolve(self) -> koji_core::ClusteringConfig { /* defaults table */ } }

pub struct RoutingArgs   { /* sort_by, route_split_level, plugin_args */ }   // resolve -> RoutingConfig
pub struct BootstrapArgs { /* calculation_mode, radius, s2_level, s2_size, plugin_args */ } // -> BootstrapConfig
pub struct DataFilterArgs{ /* last_seen, tth */ }                            // -> DataFilter
pub struct OutputArgs    { /* return_type: Option<String>, save_to_db, save_to_scanner,
    save_to_scanner_only, simplify */ }                                     // resolve(default_rt) -> OutputConfig
pub struct DevArgs       { /* bypass_adaptive_partition, benchmark_mode */ }  // -> DevConfig
```

Note `OutputArgs::resolve` takes the `default_return_type: ReturnTypeArg` param (derived from the inbound area container — see Task 2b) and calls `koji_core::get_return_type` when `return_type` is `Some`.

- [ ] **Step 6: Run tests, expect PASS**:

Run: `cargo test -p koji-service requests::`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add crates/koji-service
git commit -m "feat(koji-service): requests/ wire arg-groups + resolve helpers (inputs, resolve, groups)"
```

---

## Task 2b: per-operation request types + tagged `CalcRequest`

**Files:**
- Create: `crates/koji-service/src/requests/ops.rs`
- Modify: `crates/koji-service/src/requests/mod.rs` (`mod ops; pub use ops::*;`)

- [ ] **Step 1: Write the failing dispatch + parity test** in `requests/ops.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calc_request_dispatches_on_mode_tag() {
        let json = r#"{"mode":"cluster","clustering":{"radius":40},"routing":{"sortBy":"geoHash"}}"#;
        let req: CalcRequest = serde_json::from_str(json).unwrap();
        match req {
            CalcRequest::Cluster(c) => assert_eq!(c.clustering.resolve().radius, 40.0),
            _ => panic!("expected Cluster variant"),
        }
    }
    #[test]
    fn cluster_default_return_type_follows_area_container() {
        // no area -> SingleArray (parity with old init())
        let req: ClusterReq = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(req.default_return_type(), koji_core::ReturnTypeArg::SingleArray);
    }
}
```

- [ ] **Step 2: Run it, expect FAIL**:

Run: `cargo test -p koji-service calc_request_dispatches_on_mode_tag`
Expected: FAIL — `CalcRequest` undefined.

- [ ] **Step 3: Implement `ops.rs`.** Define the tagged enum + per-op structs:

```rust
#[derive(Debug, Deserialize, Serialize)] #[serde(tag = "mode", rename_all = "camelCase")]
pub enum CalcRequest {
    Cluster(ClusterReq), Route(ClusterReq), Reroute(RerouteReq),
    Bootstrap(BootstrapReq), RouteStats(StatsReq),
}
#[derive(Debug, Deserialize, Serialize)] #[serde(rename_all = "camelCase")]
pub struct ClusterReq {
    pub area: Option<GeoInput>, pub data_points: Option<DataPointsArg>,
    #[serde(default)] pub clustering: ClusteringArgs,
    #[serde(default)] pub routing: RoutingArgs,
    #[serde(default)] pub output: OutputArgs,
    #[serde(default)] pub dev: DevArgs,
    pub instance: Option<String>,
}
// RerouteReq { data_points, clusters, routing, output, radius, instance }
// BootstrapReq { area, bootstrap, routing, output, instance }
// StatsReq { data_points, clusters, radius, min_points, output, instance }
```

Add a `default_return_type(&self) -> ReturnTypeArg` helper on each op that has an `area` (container shape → RT, per spec §2: `FeatureCollection`→FC, `Feature`→Feature, `Geometry`→Geometry, `None`→`SingleArray`). The geo-family requests (`ConvertReq`, `SimplifyReq`, `MergePointsReq`) compose `{ area, output, simplify, instance, benchmark_mode }` only.

- [ ] **Step 4: Run tests, expect PASS**:

Run: `cargo test -p koji-service requests::ops`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-service
git commit -m "feat(koji-service): per-op request types + tagged CalcRequest"
```

---

## Task 3: Rewire v2 handlers (calc, jobs, geo) onto the requests

**Files:**
- Modify: `crates/koji-service/src/public/v2/calc.rs` (the `CalcPayload.request` type + `JobHandler::run` dispatch + the `run_*` helper signatures)
- Modify: `crates/koji-service/src/public/v2/jobs.rs` (the enqueue site — `grep -n CalcPayload crates/koji-service/src/public/v2/jobs.rs`)
- Modify: `crates/koji-service/src/public/v2/geo.rs` (the 3 geo handlers)
- Modify v2 handler/integration tests that post **flat** calc/geo JSON → update to the **nested** wire.

- [ ] **Step 1: Type the job payload.** In `calc.rs`, change `CalcPayload.request: serde_json::Value` → `request: CalcRequest` (it's `Serialize`). Update the enqueue sites (grep `CalcPayload {` across `public/v1/calculate.rs` + `public/v2/jobs.rs`) to pass the typed `CalcRequest`. Remove the `mode: String` *dispatch* role — keep `mode` only if the job framework keys on it; dispatch now matches the `CalcRequest` variant.

- [ ] **Step 2: Rewrite `run` dispatch.** Replace the `let args: Args = …; let ArgsUnwrapped{…} = args.init(…)` block + the inline `ClusteringConfig{…}`/`RoutingConfig{…}`/`BootstrapConfig{…}` literals (`calc.rs:108-214`) with a `match payload.request { CalcRequest::Cluster(c) => { let clustering = c.clustering.resolve(); let routing = c.routing.resolve(); … } … }`. Use `payload.area`/`payload.data_points`/`payload.clusters` (already async-resolved) for inputs. Preserve the `route`-mode `sort_by Unset → Custom("tsp")` override (spec §2 defaults table).

- [ ] **Step 3: Collapse the arg-chains.** Change `run_cluster_route` (`calc.rs:253`) to take `(&SingleVec, FeatureCollection, &ClusteringConfig, &RoutingConfig, bool /*bypass*/, &str /*instance*/)` — **drop** the 4 loose dups (`radius`, `cluster_mode`, `calculation_mode`, `min_points`); read `radius` from `clustering_config.radius` and build the Stats label from `clustering_config.mode`/`calculation_mode`. Give `run_bootstrap`/`run_reroute`/`run_route_stats` the same treatment (take the resolved configs they need; no loose dup fields).

- [ ] **Step 4: Rewire geo handlers.** In `geo.rs`, change `convert`/`simplify`/`merge_points` from `web::Json<Args>` + `.init()` to `web::Json<ConvertReq>` / `SimplifyReq` / `MergePointsReq`, resolving `output`/`simplify`/`area` from the typed request. Keep the existing `KojiGeometryCollection`/`send` logic.

- [ ] **Step 5: Update tests to the nested wire.** Find v2 calc/geo tests posting flat JSON (`grep -rn '"radius"' crates/koji-service/src` and the handler test modules) and convert request bodies to the nested shape (`{"mode":"cluster","clustering":{...},"routing":{...}}`).

- [ ] **Step 6: Build + test**:

Run: `cargo test -p koji-service`
Expected: PASS (v2 calc/geo behavior unchanged; only the wire nests).

- [ ] **Step 7: Commit**

```bash
git add crates/koji-service
git commit -m "refactor(koji-service): v2 calc/geo onto per-op requests; typed job payload; collapse run_* arg-chains"
```

---

## Task 4: v1 shim — `LegacyArgs` + move `Auth`/`Search`

**Files:**
- Create: `crates/koji-service/src/public/v1/legacy.rs` (`LegacyArgs`, `LegacyResolved`, `init`)
- Create: `crates/koji-service/src/private/auth.rs` (`Auth`, `Search`)
- Modify: v1 handlers `public/v1/{calculate,convert,geofence,route}.rs` (import swap `model::api::args::{Args, ArgsUnwrapped}` → `crate::public::v1::legacy::{LegacyArgs, LegacyResolved}`)
- Modify: `private/misc.rs:14`, `private/admin.rs` (import `Auth`/`Search` from the new module)

- [ ] **Step 1: Move the fat type frozen.** Move `Args` → `LegacyArgs` and `ArgsUnwrapped`/`DevArgsUnwrapped` → `LegacyResolved`/`LegacyDevResolved` into `legacy.rs` verbatim. Repoint their internals: `GeoInput`/`DataPointsArg` from `crate::requests`, `get_return_type` from `koji_core`, the resolution helpers from `crate::requests::resolve`. Keep `LegacyArgs::init` producing `LegacyResolved` (the flat shape) so v1 handlers barely change. Document the module as the frozen v1 compat layer.

- [ ] **Step 2: Move Auth/Search.** Move `Auth` + `Search` (verbatim) into `private/auth.rs`; add `mod auth; pub use auth::{Auth, Search};` to the `private` module root. Repoint `misc.rs`/`admin.rs` imports.

- [ ] **Step 3: Swap v1 imports.** In each v1 handler, replace `use model::api::args::{…}` with the new paths. The handler bodies (which destructure `LegacyResolved`'s flat fields) stay as-is.

- [ ] **Step 4: Build + test (v1 goldens)**:

Run: `cargo test -p koji-service`
Expected: PASS — v1 wire + outputs unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-service
git commit -m "refactor(koji-service): freeze v1 fat Args as LegacyArgs; move Auth/Search to private/"
```

---

## Task 5: Delete the `model` crate

**Files:**
- Delete: `crates/model/`
- Modify: workspace `Cargo.toml` (remove `crates/model` from `members`)
- Modify: `crates/koji-service/Cargo.toml` (remove the `model` dependency)

- [ ] **Step 1: Prove nothing imports it**:

Run: `grep -rn "use model\b\|model::\|model = \|model\.workspace" crates/ bins/ Cargo.toml`
Expected: ZERO matches (all repointed in Tasks 1–4).

- [ ] **Step 2: Delete + de-register.** `git rm -r crates/model`; remove the member line + the koji-service dep line.

- [ ] **Step 3: Build the workspace**:

Run: `cargo build --workspace`
Expected: builds clean (no `model`).

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "refactor: delete the model crate (dissolved into koji-service + koji-core)"
```

---

## Task 6: Surgical arg-chain fold — `for_each_in_bbox` → `KojiBbox`

**Files:**
- Modify: `crates/algorithms/src/clustering/crucible/refine.rs:220` (the `for_each_in_bbox` method) + its single caller at `:1300`

Scope note: `koji-core::s2::check_neighbors` (spec §4) is **left as-is** — on inspection it takes a center *point* (`lat`, `lon`), not a bbox, so a `KojiBbox` fold doesn't apply and at 5 params it's below the egregious bar. Document this in the commit body.

- [ ] **Step 1: Confirm the parity net exists.** The crucible refiner is covered by the clustering goldens.

Run: `cargo test -p algorithms clustering`
Expected: PASS (baseline before the refactor).

- [ ] **Step 2: Fold the signature.** Change `for_each_in_bbox(&self, reps, min_lat, max_lat, min_lon, max_lon, f)` → `for_each_in_bbox(&self, reps: &SingleVec, bbox: koji_core::KojiBbox, f: impl FnMut(u32))`. Inside, read `bbox.min_lat`/`max_lat`/`min_lon`/`max_lon`. At the caller (`:1300`), construct `KojiBbox { min_lat, min_lon, max_lat, max_lon }` from the values it currently passes. Pure refactor — no behavior change.

- [ ] **Step 3: Build + test (parity)**:

Run: `cargo test -p algorithms clustering`
Expected: PASS — identical results.

- [ ] **Step 4: Commit**

```bash
git add crates/algorithms
git commit -m "refactor(algorithms): fold for_each_in_bbox 4-float chain into KojiBbox

check_neighbors left as-is: it takes a center point, not a bbox (no KojiBbox fold applies)."
```

---

## Task 7: Verification gate + docs

**Files:**
- Modify: the OpenAPI doc / changelog for the nested v2 calc/geo wire (`grep -rln "api/v2/calc\|openapi\|swagger" crates/ docs/ bins/`)
- Check: in-repo v2 calc clients (`bins/koji-cli`, react-admin) for flat request construction

- [ ] **Step 1: Full workspace gate (parallel)**:

```bash
cargo fmt --all --check; cargo clippy --workspace --all-targets; cargo test --workspace
```
Expected: fmt clean; clippy 0 (modulo the pre-existing third-party `num-bigint-dig`/`proc-macro-error2` future-incompat note); tests 0 failures.

- [ ] **Step 2: Grep proof**:

Run: `grep -rn "ArgsUnwrapped\|model::api::args\|crate model" crates/ bins/`
Expected: only `LegacyResolved` references inside `public/v1/legacy.rs`; no `model::` anywhere.

- [ ] **Step 3: Update the v2 wire docs.** Note the nested calc/geo request shape in the OpenAPI/changelog source found in Step 0. Update any in-repo v2 calc client that posts flat JSON to nest under `clustering`/`routing`/`output`.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "docs(args): document nested v2 calc/geo wire; final verification gate"
```

---

## Self-Review

**Spec coverage:** §1 re-homing → Tasks 1 (get_return_type), 2a (inputs/helpers), 4 (Auth/Search/LegacyArgs), 5 (delete model). §2 request/resolve layer → Tasks 2a/2b. §3 v1 shim → Task 4. §4 chain cleanups → Task 3 (`run_*`) + Task 6 (`for_each_in_bbox`); `check_neighbors` exclusion documented. §6 testing → per-task parity tests + Task 7 gate. §7 non-goals: `ApiQueryArgs` untouched (no task); validation leniency preserved (resolve mirrors `init`). **Type consistency:** `ClusteringArgs::resolve → koji_core::ClusteringConfig`, `CalcRequest` tagged on `mode`, `LegacyArgs`/`LegacyResolved` names consistent across Tasks 4–7. **Placeholders:** resolve() bodies reference the spec §2 defaults table (the contract) rather than re-pasting; new type signatures + all tests/commands are concrete. **Ordering:** each task leaves the workspace green (model survives until Task 5; nothing imports it by then).
