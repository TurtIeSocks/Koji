# Koji V2 — Phase 2: algorithms take config structs + extract koji-plugins (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run). Assumptions = async review checkpoint.
**Parent:** [architecture](2026-05-28-koji-v2-architecture-design.md) §4, §8. Roadmap P2. Investigator map informs the param→struct grouping + plugin coupling.

> **Verification caveat:** the calc path (`calculate.rs` → algorithms) needs a real MySQL connection + the `benchmark_mode`/`bypass_adaptive_partition` A/B to verify behavior; that can't run in the dev sandbox. P2 is build-verified + wiring cross-checked 1:1 against the pre-change flat values here; **the maintainer must run the A/B against a real DB** before trusting parity. Algorithm internals are unchanged — only the parameter plumbing.

## Two halves (independent green sub-commits)

### P2a — algorithm entry points take config structs
Change the three entry points from primitive lists to the koji-core config structs defined in P1d:
```rust
clustering::main(data_points: &SingleVec, cfg: &ClusteringConfig, collection: FeatureCollection, stats: &mut Stats) -> SingleVec
routing::main(data_points: &SingleVec, clusters: SingleVec, radius: f64, cfg: &RoutingConfig, stats: &mut Stats) -> SingleVec
bootstrap::main(area: FeatureCollection, cfg: &BootstrapConfig, routing: &RoutingConfig, stats: &mut Stats) -> Vec<Feature>
```
- Inside each, read `cfg.field` instead of the old positional params. **Algorithm logic + Stats mutations unchanged.**
- `plugin_args` stays a raw composite `String` on each config (model `init()` still appends `--radius/--min_points/--max_clusters` to clustering's, `--radius` to bootstrap's — for the plugin subprocess; algorithms pass it straight to `Plugin::new`, never parse it). The typed fields (`radius`, `min_points`, `max_clusters`) are **separate** struct fields that feed greedy/fastest/s2.
- `routing::main` keeps `radius` as a separate param (per §4) — it's needed by `sort_point_count` and is shared with clustering; `RoutingConfig` omits radius.
- **`ArgsUnwrapped` becomes composed:** replace the flat config-ish fields with `clustering: ClusteringConfig`, `routing: RoutingConfig`, `bootstrap: BootstrapConfig`, `output: OutputConfig`, `data_filter: DataFilter`, `dev: DevConfig`; keep the resolved inputs/results flat (`area`, `data_points`, `clusters`, `mode`, `parent`, `instance`). `init()` builds the structs. `DevArgsUnwrapped` removed (→ `koji_core::DevConfig`).
- **`calculate.rs`** (4 calc handlers) destructure the composed `ArgsUnwrapped` and pass `&cfg`; read `cfg.output.return_type`, `cfg.data_filter.{last_seen,tth}`, `cfg.dev.*`, etc. Cross-check every value against the pre-change field 1:1.

### P2b — extract `koji-plugins` (move only; protocol unchanged, §8)
- New crate `koji-plugins` (`→ koji-core`): move `algorithms/src/plugin.rs` (`Plugin`, `Folder`, `JoinFunction`, `ParseCoord`, `new`/`run`/`run_multi`) + `stringify_points` (from `algorithms/utils.rs:171`, plugin-only).
- **`create_cell_map`** (`algorithms/s2.rs:353`) is used by **both** the plugin runner and `clustering/partition.rs` → move it to **`koji-core`** (pure `SingleVec → HashMap<u64,SingleVec>` S2-grid grouping; adds the `s2` crate dep to koji-core). algorithms + koji-plugins both call `koji_core::create_cell_map`. (Avoids an algorithms↔koji-plugins cycle without changing the plugin protocol.)
- `algorithms` gains a `koji-plugins` dep; `clustering`/`routing`/`bootstrap` import `Plugin`/`Folder` from `koji_plugins`. `get_plugin_list` (utils.rs:147) stays in algorithms (used by `clustering_plugins()`/`bootstrap_plugins()`), or moves too — decide at exec (keep in algorithms if it has no plugin-runner coupling).

## Dep graph after P2
`koji-core` (+ `create_cell_map`, `s2` dep) ← koji-plugins ← algorithms ← api. Acyclic.

## Sub-commit plan
1. **P2b** first (pure structural move, build-verified): create koji-plugins, move plugin.rs + stringify_points, move create_cell_map→koji-core, rewire algorithms. Green + test parity.
2. **P2a**: change the 3 signatures to read configs; compose `ArgsUnwrapped`; rewire `calculate.rs`; remove redundant flat fields. Green + test parity (algorithm units) + wiring cross-check. Commit.

## Assumptions (DELEGATE CHECKPOINT)
1. **Geometry stays `SingleVec`** (not §4's `&[Point]`) — P1c kept the aliases to avoid hot-path churn; signatures use `&SingleVec`/`SingleVec`.
2. **`plugin_args` = composite string built in model `init()`**; typed fields separate on the config. Algorithms never parse it (only `Plugin::new` does, for `Custom` modes). No behavior change to plugin invocation.
3. **`create_cell_map` → koji-core** (+ `s2` dep on koji-core). Alternative (koji-plugins owns it) rejected — it's an S2 clustering util used by `partition.rs`, not plugin-specific; core is the shared home.
4. **Plugin protocol unchanged** (move-only) per §8; formalization (manifest/JSON protocol) is P7.
5. **`routing::main` keeps a separate `radius` param**; `RoutingConfig` omits radius (matches §4).
6. **Behavioral parity is build + 1:1-wiring-checked here, NOT runtime-verified** — maintainer runs the `benchmark_mode` A/B against a real DB. Algorithm internals untouched, so parity risk is confined to the param plumbing.
7. **`model` still builds the configs in `init()`** and stays the thin v1 crate (dissolves P4).
