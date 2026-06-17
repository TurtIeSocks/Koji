# Args Restructure — Design

- **Date:** 2026-06-14
- **Branch:** `claude/v2` (local, unpushed — rides the API security rework)
- **Status:** Approved (design); pending implementation plan
- **Supersedes / completes:** [P1d config structs](2026-05-29-koji-v2-p1d-args-config-structs-design.md) — that spec's Task 1 (the 7 koji-core config structs + `ReturnTypeArg` move) landed; its Tasks 2–3 (`ArgsUnwrapped` *produces* the configs; handlers read `cfg.*`) never did. This spec finishes that seam and goes further: dissolves `model`, splits the wire DTO per-operation, and kills the worst primitive-arg chains.

## Problem

The "Args situation" is three tangled messes around the calc-request path:

1. **A half-finished migration.** `koji-core::config` holds 7 resolved config structs
   (`S2Config`, `ClusteringConfig`, `RoutingConfig`, `BootstrapConfig`, `DataFilter`,
   `OutputConfig`, `DevConfig`) and the algorithm entry points already consume them
   (`clustering::main(&ClusteringConfig)`, `routing::main(&RoutingConfig)`,
   `bootstrap::main(&BootstrapConfig)`). **But** `model::Args` → `ArgsUnwrapped` still
   resolves to **30 flat fields**, and every calc handler destructures ~20 of them and
   **hand-reassembles** the config structs inline at the call site
   (`koji-service/src/public/v2/calc.rs:140-214`). That hand-assembly is duplicated
   across the calc/jobs/v1 handlers.

2. **A junk-drawer crate.** `model` exists only to hold HTTP-request DTOs (`Args`,
   `ArgsUnwrapped`, `Auth`, `Search`, `DataPointsArg`, `GeoInput`, `get_return_type`).
   An audit confirms **only `koji-service` depends on it** (11 files); zero leaf crates
   touch it. It is a layer with no reason to be its own crate.

3. **Primitive-arg chains.** 20 functions carry 5+ positional params. The worst,
   `run_cluster_route` (`calc.rs:253`), takes **11** — and is half-migrated: it passes
   `&ClusteringConfig` *and* loose `radius`/`min_points`/`cluster_mode`/`calculation_mode`
   that already live inside that config.

## Goals (user-stated)

- Remove the need for the `model` crate entirely.
- Every child struct lives in the crate that uses it; the big service crates/bins
  bring those in as deps.
- Minimize the never-ending single-primitive function argument chains.
- Do the whole thing correctly and professionally — no lingering tech debt.

## Decisions (signed off)

1. **Scope: full (Option C).** Dissolve `model`, co-locate types, split the wire DTO
   per-operation, kill the worst arg-chains.
2. **v1 stays as a thin shim.** The legacy `public/v1/*` surface keeps its current
   (flat) wire for existing clients; its fat DTO moves into koji-service as a frozen
   compat type and adapts into the shared resolver. `model` dies regardless.
3. **Nested arg-groups, two-layer.** koji-service gets small `Option`-field wire
   structs (`ClusteringArgs`, `RoutingArgs`, …), each `resolve()`-ing into the matching
   **koji-core config**. koji-core configs **stay pure domain — no serde added.** v2 wire
   JSON becomes nested (`{ clustering: {…}, routing: {…} }`).
4. **Keep the existing endpoints; type their bodies.** No `/calc/cluster` route
   explosion. `/api/v2/calc` takes an internally-tagged `CalcRequest` (serde `tag = "mode"`)
   so serde drives operation dispatch (replacing the current `mode`-string match).
5. **Surgical arg-chain reach.** Service-layer chains + fold the 4-float bbox chains
   into the existing `KojiBbox`. Leave deep crucible/genetic/greedy loop-state helpers
   alone (churning tested hot code buys little).

## §1 — Type re-homing (dissolving `model`)

| `model` item | kind | new home | note |
|---|---|---|---|
| `Args` (fat calc/convert DTO) | wire | **dissolved** → v2 per-op requests (koji-service `requests/`); the flat remnant → koji-service `public/v1/legacy.rs` as frozen `LegacyArgs` | v2 stops using it |
| `ArgsUnwrapped`, `DevArgsUnwrapped` | resolved | **dissolved** → koji-core configs (exist) + per-op resolved structs | the 30-field flat bag dies |
| `DataPointsArg` | wire | koji-service `requests/` | data-point input enum |
| `GeoInput` + `to_koji` + `geometry_to_koji` | wire+logic | koji-service `requests/` | area input |
| `Auth` | wire | koji-service `private/` (used by `misc::login`) | |
| `Search` | wire | koji-service `private/` (used by `misc`, `admin`) | |
| `get_return_type` | pure fn | **koji-core** (beside `ReturnTypeArg`) | used by v1 + v2; pure string parsing |
| `validate_s2_cell` | pure fn | koji-service `requests/resolve.rs` | resolution helper |
| `resolve_data_points` | logic | koji-service `requests/resolve.rs` | `DataPointsArg` → `SingleVec` |
| crate `model` | — | **deleted** from workspace members + koji-service `Cargo.toml` | ✅ goal met |

## §2 — The v2 request / resolve layer

New module `crates/koji-service/src/requests/`:

```rust
// groups.rs — HTTP wire. Option fields, camelCase. One group per koji-core config.
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ClusteringArgs {
    radius: Option<Precision>, min_points: Option<usize>, max_clusters: Option<usize>,
    mode: Option<ClusterMode>, calculation_mode: Option<CalculationMode>,
    s2_level: Option<u8>, s2_size: Option<u8>, cluster_split_level: Option<u64>,
    center_clusters: Option<bool>, genetic_post_processing: Option<bool>,
    plugin_args: Option<String>,
}
impl ClusteringArgs {
    pub fn resolve(self) -> koji_core::ClusteringConfig { /* defaults below + plugin-arg build */ }
}
// + RoutingArgs, BootstrapArgs, DataFilterArgs, OutputArgs, DevArgs — same pattern.

// ops.rs — per-operation requests compose only the groups they use.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum CalcRequest {                       // POST /api/v2/calc
    Cluster(ClusterReq), Route(ClusterReq), Reroute(RerouteReq),
    Bootstrap(BootstrapReq), RouteStats(StatsReq),
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterReq {
    area: Option<GeoInput>, data_points: Option<DataPointsArg>,
    #[serde(default)] clustering: ClusteringArgs,
    #[serde(default)] routing: RoutingArgs,
    #[serde(default)] output: OutputArgs,
    #[serde(default)] dev: DevArgs,
}
impl ClusterReq { pub fn resolve(self) -> ClusterResolved { /* see resolve order below */ } }
```

`CalcRequest` is `Serialize` so it rides the job payload directly — the job handler
deserializes the **typed** request and `resolve()`s it, replacing the current
"re-parse a fat `Args` + `init()`" step (`calc.rs:106`). `requests/resolve.rs` holds the
shared default + resolution logic so v1 and v2 funnel through **one source of truth**.

The **geo family** (`/api/v2/geo/{convert,simplify,merge-points}`) gets its own lean
requests (`ConvertReq`, `SimplifyReq`, `MergePointsReq`) composing just `area` /
`data_points` + `OutputArgs` (+ a `simplify` flag) — no clustering/routing groups. The
`/api/v2/calc` `Cluster` and `Route` variants share `ClusterReq`; they differ only in the
`sort_by` default (`route` overrides `Unset` → `Custom("tsp")` — see the defaults table).

### Resolution defaults (parity-critical — replicate exactly from today's `init()`)

| field | default | source nuance |
|---|---|---|
| `radius` | `70.0` | |
| `min_points` | `1` | |
| `max_clusters` | `usize::MAX` | `None` → MAX; `Some(0)` → MAX; else value |
| `cluster_mode` | `Balanced` | |
| `calculation_mode` | `Radius` | |
| `s2_level` / `s2_size` | `15` / `9` | |
| `cluster_split_level`, `route_split_level` | `validate_s2_cell` | 0–20 else warn→0; `None`→0 |
| `center_clusters`, `genetic_post_processing` | `false` | |
| `dev.bypass_adaptive_partition`, `benchmark_mode` | `false` | |
| `last_seen` | `0` | |
| `save_to_db` / `save_to_golbat` / `save_to_golbat_only` / `simplify` | `false` | |
| `sort_by` | `Unset` | **`/calc` `route` mode** overrides `Unset` → `Custom("tsp")` (keep in handler) |
| `tth` | `All` | |
| `routing_args` | `""` | |
| `clustering_args` | `""` then append `" --radius {radius} --min_points {min_points} --max_clusters {max_clusters}"` | **moves into `ClusteringArgs::resolve`** |
| `bootstrapping_args` | `""` then append `" --radius {radius}"` | **moves into `BootstrapArgs::resolve`** |
| `instance` | `""` | |
| `mode` (`koji_core::Mode`) | `Mode::from_legacy(instance)` else `Unset` | |
| `return_type` | `get_return_type(rt, default)` | `default` follows inbound `area` container: `FeatureCollection`→FC, `Feature`→Feature, `Geometry`→Geometry, **no area**→`SingleArray` |
| `area` | `GeoInput` → `KojiGeometryCollection` → `FeatureCollection` | lenient: bad geom logs warn, yields empty FC |

**Resolve order** inside a per-op `resolve()`: normalize `area` first → derive the
`default_return_type` from its container shape → `OutputArgs::resolve(default_return_type)`.
The `area`/`default_return_type` coupling is why `OutputArgs::resolve` takes the default
as a param rather than hardcoding `SingleArray`.

## §3 — v1 shim (kept, wire frozen)

The fat flat type moves verbatim into `koji-service/src/public/v1/legacy.rs` as
`LegacyArgs` (frozen, documented compat). v1 handlers adapt: `LegacyArgs` (flat) →
the **same `requests/resolve.rs` helpers** → koji-core configs → the same compute cores.
v1 wire JSON is unchanged; existing clients (Poracle, map frontends) are unaffected.

## §4 — Arg-chain cleanups (surgical)

| target | today | after |
|---|---|---|
| `run_cluster_route` (`calc.rs:253`) | 11 params; `&ClusteringConfig` + 4 loose dups | take `&ClusterResolved`; drop the dups; Stats label reads the configs |
| `run_bootstrap` / `run_reroute` / `run_route_stats` (`calc.rs`) | 6 each | take their per-op resolved struct |
| `for_each_in_bbox` (`crucible/refine.rs:220`, 8) | 4 bbox floats | fold into `KojiBbox` |
| `check_neighbors` (`koji-core/s2/mod.rs:214`, 6) | lat/lon/level + state | fold geometry into `KojiBbox`/a small state ref |
| deep crucible/genetic/greedy helpers | 5–8 | **untouched** (surgical boundary) |

## §5 — Data flow

```
BEFORE: JSON → model::Args(30 opt) → init() → ArgsUnwrapped(30 flat) → [handler hand-builds configs] → algos
AFTER:  JSON → koji-service CalcRequest(tagged; nested groups) → resolve() → koji-core configs → algos
        v1:   JSON(flat) → LegacyArgs → requests/resolve.rs → koji-core configs → algos   (same sink)
```

## §6 — Testing

- **v2 wire parity:** each per-op request deserializes its nested JSON; `resolve()`
  yields configs byte-identical to today's `init()` for equivalent input — focus on
  the plugin-arg string-build, `max_clusters 0→MAX`, and the `area`→`default_return_type`
  coupling.
- **v1 golden parity:** `LegacyArgs` deserializes today's flat wire identically; v1
  endpoint outputs unchanged (existing goldens).
- **Algorithm parity:** the `run_*` + bbox-fold refactors are behavior-preserving —
  guarded by existing clustering/routing goldens.
- **Workspace gate** each sub-commit: `fmt` + `clippy` + `test` (host target — never
  `--target wasm32`, that needs nightly + `-Z build-std`).

## §7 — Non-goals / deferred

- **`ApiQueryArgs`** (the 25-field GET read-filter god-struct in
  `koji-core/src/query_args.rs`) is **out of scope here and queued as the NEXT refactor**
  once this lands. It is not in `model`, already lives in koji-core, and is a different
  concern (read/output filters, not calc inputs). Same nested-sub-struct treatment will
  apply (name-modifiers / feature-props / hierarchy / return-type groups). Tracked in
  memory `koji-apiqueryargs-next`.
- **Validation semantics unchanged.** Resolution stays lenient (warn + default);
  strict per-field rejection belongs to the separate API security rework — the typed
  structure makes it a clean future drop-in.
- **Deep algorithm internals** stay as-is (per the surgical boundary).
- **v1 wire** is frozen, not changed.

## §8 — Risks

- **v2 wire becomes nested.** Pre-release/local, so acceptable; update the OpenAPI doc +
  changelog and any in-repo v2 client.
- **Default parity.** The resolution defaults table above is the contract — any drift
  (esp. the plugin-arg string concatenation) silently changes algorithm behavior. Parity
  tests are the net.
- **Blast radius.** ~11 koji-service files + `model` deletion + 2 algorithm spots.
  Sequenced as green sub-commits.

## §9 — Task sequence (for the implementation plan)

1. **koji-core:** move `get_return_type` in (beside `ReturnTypeArg`); re-point. Green.
2. **koji-service `requests/` module:** arg-groups + `resolve()` → configs; `resolve.rs`
   shared defaults; per-op `*Req` + tagged `CalcRequest`; `GeoInput`/`DataPointsArg`
   move in. Unit + wire-parity tests. Green.
3. **Rewire v2 handlers** (calc/geo/jobs) onto the per-op requests; typed job payload;
   delete the inline config hand-assembly. Green; parity.
4. **v1 shim:** `LegacyArgs` into `public/v1/legacy.rs`; v1 handlers adapt via
   `resolve.rs`; `Auth`/`Search` move to `private/`. Green; v1 goldens.
5. **Delete `model`:** drop the crate, workspace member, and koji-service dep. Green.
6. **Arg-chain cleanups:** `run_cluster_route` + the `run_*` trio; bbox folds
   (`for_each_in_bbox`, `check_neighbors`) → `KojiBbox`. Green; algorithm parity.
7. **Verification gate** + OpenAPI/changelog note for the nested v2 wire.
