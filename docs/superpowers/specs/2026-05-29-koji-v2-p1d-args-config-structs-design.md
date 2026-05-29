# Koji V2 — Phase 1d: break `Args` into config structs (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run; "roadmap split" scope confirmed by user). Assumptions section is the async review checkpoint.
**Parent:** [architecture](2026-05-28-koji-v2-architecture-design.md) §4. Roadmap P1d.

## Goal

Replace the 36-field `Args` super-struct's flat `ArgsUnwrapped` with composable **config structs in koji-core**, drop the 6 deprecated fields, and have `ArgsUnwrapped` *produce* the config structs. Algorithm entry points keep their primitive signatures this phase; `calculate.rs` unpacks `cfg.field` for them — **P2** flips `clustering`/`routing`/`bootstrap` to take the structs (+ extracts koji-plugins, perf-verified). (Scope choice: "roadmap split", confirmed.)

## Drop these 6 deprecated fields
`devices`, `fast`, `generations`, `only_unique`, `route_chunk_size`, `routing_time`. (`fast` currently only feeds the `cluster_mode` default — preserve that: default `cluster_mode` to `Balanced` directly.)

## Config structs (koji-core, new `config` module)

Derived from the three algorithm entry points' primitives (`clustering::main` 15, `routing::main` 7, `bootstrap::main` 10):

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct S2Config { pub level: u8, pub size: u8 }            // s2_level, s2_size

pub struct ClusteringConfig {
    pub mode: ClusterMode,
    pub radius: Precision,
    pub min_points: usize,
    pub max_clusters: usize,
    pub cluster_split_level: u64,
    pub calculation_mode: CalculationMode,
    pub s2: S2Config,
    pub center_clusters: bool,
    pub genetic_post_processing: bool,
    pub plugin_args: String,        // clustering_args
}

pub struct RoutingConfig {
    pub sort_by: SortBy,
    pub route_split_level: u64,
    pub plugin_args: String,        // routing_args
}                                    // radius passed separately to route() per §4

pub struct BootstrapConfig {
    pub calculation_mode: CalculationMode,
    pub radius: Precision,
    pub s2: S2Config,
    pub plugin_args: String,        // bootstrapping_args
}                                    // bootstrap(area, &BootstrapConfig, &RoutingConfig, stats)

pub struct AreaInput {
    pub area: Option<GeoFormats>,
    pub instance: String,
    pub geometry_type: Option<String>,
}                                    // resolves to FeatureCollection + mode(FenceType) + default ReturnType

pub struct DataFilter { pub last_seen: u32, pub tth: SpawnpointTth }

pub struct OutputConfig {
    pub return_type: ReturnTypeArg,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub save_to_scanner_only: bool,
    pub simplify: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DevConfig { pub bypass_adaptive_partition: bool, pub benchmark_mode: bool }
```

`ArgsUnwrapped` keeps the *resolved inputs/results* that aren't config — `area: FeatureCollection`, `data_points: SingleVec`, `clusters: SingleVec`, `mode: FenceType`, `parent: Option<UnknownId>` — plus the config structs above (composition), and gains accessors so `calculate.rs` reads `unwrapped.clustering`, `unwrapped.routing`, etc.

## Crate placement
- **Config structs + `S2Config`** → `koji-core::config` (pure domain; no http/db).
- **`Args` / `ArgsUnwrapped` / `ReturnTypeArg` / `Auth` / `Search` / `DataPointsArg` / `get_return_type`** stay in **`model::api::args`** (the v1 calc-request DTOs). `model` persists as the thin request-DTO crate until **P4**, where the v1 shim (koji-service) absorbs it — matching §4 "the legacy `Args` maps into the same config structs". (So `model` is NOT dissolved in P1d; the no-`model` end-state lands at P4.)

`ReturnTypeArg` is referenced by `OutputConfig` (in koji-core) → therefore **`ReturnTypeArg` also moves to koji-core** (small Deserialize enum; `OutputConfig` needs it). `get_return_type` + `Args`/`ArgsUnwrapped` stay in model and use `koji_core::ReturnTypeArg`.

## Consumer impact (this phase)
- `model::api::args::init()` builds the config structs (defaults applied; deprecated dropped).
- `calculate.rs` destructures `ArgsUnwrapped`, then passes `cfg.clustering.radius`, `cfg.clustering.mode`, … to the still-primitive `clustering::main` / `routing::main` / `bootstrap::main`. (Transitional unpack; removed in P2.)
- `algorithms` unchanged this phase.

## Sub-commit plan (each green; tests parity 16+3 / 4+1)
1. **koji-core `config` module** — add `S2Config` + the 8 config structs + move `ReturnTypeArg` into koji-core (re-point `model`/`api`). Green.
2. **`ArgsUnwrapped` produces configs + drop deprecated** — restructure `init()`; drop the 6 fields from `Args`/`ArgsUnwrapped`; `ArgsUnwrapped` composes the config structs. Green.
3. **`calculate.rs` reads configs** — unpack `cfg.*` at the primitive algorithm calls; spot-check the 4 calc handlers. Green; full test parity; commit.

## Assumptions (DELEGATE CHECKPOINT)
1. **`radius` lives in `ClusteringConfig` + `BootstrapConfig`; `RoutingConfig` omits it** (passed separately to `route()` per §4). Minor duplication, matches the architecture's signatures.
2. **`calculation_mode` + `S2Config` are embedded** in both `ClusteringConfig` and `BootstrapConfig` (both algorithms need them) rather than a separate top-level pass.
3. **`benchmark_mode` joins `DevConfig`** (it's a dev/stats toggle), alongside `bypass_adaptive_partition` (migrated from the existing `DevArgs`).
4. **`ReturnTypeArg` moves to koji-core** (required because `OutputConfig` references it); `Auth`/`Search`/`Args`/`ArgsUnwrapped` stay in `model`.
5. **`model` is NOT dissolved in P1d** — it stays as the thin v1 calc-request crate until P4's v1 shim. (Revises the earlier roadmap RESUME note that said "dissolve model in P1d".)
6. **Algorithm signatures unchanged** this phase (P2 flips them); `calculate.rs` does a transitional `cfg.field` unpack.
7. **`parent`, `clusters`, `data_points`, `mode`, `area`** remain flat on `ArgsUnwrapped` (resolved inputs/results, not config).
