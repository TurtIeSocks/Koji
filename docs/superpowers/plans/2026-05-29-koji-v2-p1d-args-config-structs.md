# Koji V2 — Phase 1d: break `Args` into config structs

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Steps use checkbox (`- [ ]`).

**Goal:** Define 8 composable config structs in koji-core, drop the 6 deprecated `Args` fields, and have `ArgsUnwrapped` produce the structs. Algorithm signatures stay primitive (P2 flips them); `calculate.rs` unpacks `cfg.field` transitionally.

**Architecture:** Compiler-driven; each task a green sub-commit (`cargo build --workspace` + `cargo test --workspace` parity 16+3 / 4+1). Design + field assignments + assumptions: `docs/superpowers/specs/2026-05-29-koji-v2-p1d-args-config-structs-design.md`.

**Tech Stack:** Rust 2024, koji-core, model (thin Args remainder).

---

### Task 1: koji-core `config` module + move `ReturnTypeArg`

**Files:** `crates/koji-core/src/config.rs` (new), `crates/koji-core/src/lib.rs`, `crates/koji-core/src/return_type.rs` (new), `crates/model/src/api/args.rs` (drop ReturnTypeArg def + re-point).

- [ ] **Step 1:** Create `crates/koji-core/src/return_type.rs` — move the `ReturnTypeArg` enum verbatim from `model/src/api/args.rs` (derives `Debug, Serialize, Deserialize, Clone`; 14 variants AltText/Text/SingleArray/MultiArray/SingleStruct/MultiStruct/Geometry/GeometryVec/Feature/FeatureVec/FeatureCollection/PoracleSingle/Poracle/Sql).
- [ ] **Step 2:** Create `crates/koji-core/src/config.rs` with `S2Config` + the 8 config structs exactly as in the spec's struct block (use `crate::{CalculationMode, ClusterMode, GeoFormats, Precision, SortBy, SpawnpointTth}` + `crate::ReturnTypeArg` for OutputConfig). Derive `Debug, Clone` (+ `Copy, Default` on `S2Config`; `Default` on `DevConfig`).
- [ ] **Step 3:** `lib.rs`: add `mod return_type; mod config;` + `pub use return_type::ReturnTypeArg;` + `pub use config::{AreaInput, BootstrapConfig, ClusteringConfig, DataFilter, DevConfig, OutputConfig, RoutingConfig, S2Config};`.
- [ ] **Step 4:** In `model/src/api/args.rs` delete the `ReturnTypeArg` enum; add `ReturnTypeArg` to the `use koji_core::{…}` import. In `api` crate, `model::api::args::ReturnTypeArg` / `crate::model::api::args::ReturnTypeArg` → `koji_core::ReturnTypeArg` (build → fix; ~geofence.rs, route.rs, response.rs, calculate.rs).
- [ ] **Step 5:** `cargo build --workspace` green; `cargo test --workspace` parity. Commit `feat(core): add config structs (Clustering/Routing/Bootstrap/S2/Area/DataFilter/Output/Dev) + move ReturnTypeArg`.

---

### Task 2: `ArgsUnwrapped` produces config structs + drop deprecated

**Files:** `crates/model/src/api/args.rs`.

- [ ] **Step 1:** Drop the 6 deprecated fields from `Args` (`devices`, `fast`, `generations`, `only_unique`, `route_chunk_size`, `routing_time`) and their destructure in `init()`. Preserve the `cluster_mode` default as `ClusterMode::Balanced` (was the `fast` fallback).
- [ ] **Step 2:** Restructure `ArgsUnwrapped`: keep resolved inputs/results flat (`area: FeatureCollection`, `data_points`, `clusters`, `mode: FenceType`, `parent`, `instance`); replace the flat config-ish fields with composed structs `clustering: ClusteringConfig`, `routing: RoutingConfig`, `bootstrap: BootstrapConfig`, `data_filter: DataFilter`, `output: OutputConfig`, `dev: DevConfig`. Drop the now-unused `ArgsUnwrapped` fields that fold into configs (cluster_mode, cluster_split_level, max_clusters, min_points, radius, s2_level, s2_size, calculation_mode, sort_by, route_split_level, *_args, center_clusters, genetic_post_processing, return_type, save_*, simplify, last_seen, tth, benchmark_mode).
- [ ] **Step 2b:** Update `init()` to build the config structs from the unwrapped/defaulted values (same defaults as today: radius 70.0, s2_level 15, s2_size 9, min_points 1, cluster_mode Balanced, calculation_mode Radius, max_clusters 0→usize::MAX, etc.). Remove `DevArgsUnwrapped` in favor of `koji_core::DevConfig` (carry `bypass_adaptive_partition` + `benchmark_mode`).
- [ ] **Step 3:** `cargo build -p model` (api will break — fixed in Task 3). Commit deferred to Task 3 (model+api land together green).

---

### Task 3: `calculate.rs` reads configs (transitional unpack)

**Files:** `crates/api/src/public/v1/calculate.rs`.

- [ ] **Step 1:** In each of the 4+ calc handlers, update the `ArgsUnwrapped { … }` destructure to the new shape, then pass config fields to the primitive algorithm calls: e.g. `clustering::main(&data_points, cfg.clustering.mode, cfg.clustering.radius, cfg.clustering.min_points, &mut stats, cfg.clustering.cluster_split_level, cfg.clustering.max_clusters, cfg.clustering.calculation_mode, cfg.clustering.s2.level, cfg.clustering.s2.size, area, &cfg.clustering.plugin_args, cfg.clustering.center_clusters, cfg.clustering.genetic_post_processing, cfg.dev.bypass_adaptive_partition)`. Same unpack for `routing::main` (radius from `cfg.clustering.radius` or `cfg.bootstrap.radius` as today) + `bootstrap::main`. Build → fix → repeat.
- [ ] **Step 2:** `cargo build --workspace` green; `cargo test --workspace` parity 16+3 / 4+1; `rustfmt --edition 2024` the changed koji-core/model/api files.
- [ ] **Step 3:** Commit `refactor(model,api): ArgsUnwrapped yields config structs; drop 6 deprecated fields`.
- [ ] **Step 4:** Update roadmap RESUME + P1d entry → done; NEXT = P2. Commit `docs(v2): mark P1d done; set NEXT to P2`.

---

## Self-Review
**Spec coverage:** ✅ 8 config structs (Task 1); ✅ ReturnTypeArg→core (Task 1); ✅ drop deprecated (Task 2); ✅ ArgsUnwrapped composes configs (Task 2); ✅ transitional unpack (Task 3); ✅ model stays thin. **Placeholders:** struct fields from the spec; algorithm-call unpack uses the verified primitive signatures (clustering 15-arg, routing 7-arg, bootstrap 10-arg). **Risk:** the calculate.rs handlers each destructure ArgsUnwrapped differently — fix per-handler via the compiler; behavior unchanged (same values, regrouped). Tests parity is the net.
