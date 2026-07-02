# tsp-mt Integration — Design

**Date:** 2026-07-02
**Status:** Approved (participate-mode Q&A, all sections signed off)
**Scope:** Replace Koji's clean-room S2-seeded TSP router with the tsp-mt crates, add a standalone tsp-mt mode, and remove all or-tools/C++ machinery.

## Decisions (locked)

| Decision | Choice |
|---|---|
| Hybrid fate | **Both modes**: standalone tsp-mt (`SortBy::Tsp`) + hybrid S2-seed → tsp-mt refine (`SortBy::TspHybrid`) |
| Plugin system | **Keep** `koji-plugins` crate, `SortBy::Custom`, registry, subprocess protocol — for external users. Only the built-in or-tools plugin dies. |
| Default Route sort | **Standalone tsp-mt** (`Unset` → `SortBy::Tsp`; was `Custom("tsp")` = or-tools subprocess) |
| Dependency form | **crates.io** published versions (Approach A), not git/path deps |
| Exposed knobs | **None.** No wire-level solver options. `max_neighbors: 20`, `time_limit: None` (solver auto-derives 2–120 s from n), `seed: 12345`, `threads: 0` (all cores), rest tsp-ils defaults. |

## Background

- Old router: `crates/algorithms/src/routing/two_opt.rs` — clean-room S2-seeded 2-opt/Or-opt, opt-in via `SortBy::Tsp`. Superseded by tsp-mt (benchmarked +0.72 % gap vs BHH-optimal estimate at 100 k nodes / 30 s).
- Default Route calc still shells out to the **or-tools C++ plugin** (`resolve_cluster_route()` defaults `Unset` → `Custom("tsp")`, `crates/koji-service/src/public/v2/calc.rs:237`).
- tsp-mt (`/Users/rin/GitHub/tsp-mt`) ships two published Apache-2.0 crates with zero Koji coupling:
  - **tsp-ils 0.1.0** — generic const-D solver: `solve(&[[f64; D]], &SolverConfig) -> Solution { tour: Vec<u32>, length: f64 }`. Greedy edge construction + 2-opt/Or-opt with don't-look bits + ILS (double-bridge kicks, parallel segment rounds, multi-start, spike repair). Deterministic seeded PRNG, rayon-parallel, wasm-safe (web-time).
  - **tsp-geo 0.2.0** — `solve_order(&[GeoPoint], cfg) -> Result<Vec<u32>>`; lat/lng validated, embedded on Earth-radius unit sphere (chord ≈ great-circle). Optional `geo-types` interop feature.

## Part 1 — tsp-mt: `refine` API (new versions)

New entry points that accept an initial tour and skip greedy construction:

```rust
// tsp-ils
pub fn refine<const D: usize>(pts: &[[f64; D]], initial_tour: &[u32], cfg: &SolverConfig) -> Solution
```

- Seeds `TourState` from `initial_tour`, then runs the identical ILS pipeline (candidates, 2-opt/Or-opt, kicks, segment rounds, spike repair).
- Multi-start path (small n): walker 0 seeds from `initial_tour`, remaining walkers use greedy construction; best wins.
- Invalid tour (not a permutation of `0..n`): tsp-ils may panic/assert like any index bug; the geo layer guards it.

```rust
// tsp-geo
pub fn refine_order(points: &[GeoPoint], initial_tour: &[u32], cfg: &SolverConfig) -> Result<Vec<u32>>
```

- Same coord validation/embedding as `solve_order`; returns `Err(Error::InvalidInput)` on a bad initial tour (never panics).

**Publish:** tsp-ils 0.1.0 → **0.2.0**, tsp-geo 0.2.0 → **0.3.0** (additive; tsp-geo bumps its tsp-ils dep). Existing tag-triggered publish workflow, dependency order. Koji work depends on these being live on crates.io.

## Part 2 — Koji wiring

**Cargo** (`crates/algorithms`):

```toml
tsp-geo = { version = "0.3", default-features = false, features = ["std"] }
```

*(Amended during planning: the `geo-types` feature is unnecessary — Koji routing operates on `SingleVec = Vec<[Precision; 2]>` raw `[lat, lng]` arrays, so `GeoPoint::from_lat_lng` maps directly.)*

- `tsp-geo/parallel` added to the algorithms crate's existing `native` feature → server gets rayon, koji-wasm gets the single-threaded solver automatically.

**SortBy** (`crates/algorithms/src/routing/sort_by.rs`):

| Variant | Behavior | Aliases |
|---|---|---|
| `Tsp` | standalone tsp-mt: `tsp_geo::solve_order` | `"tsp"`, `"optimized"`, `"2opt"` (unchanged strings, better tours) |
| `TspHybrid` (new) | existing S2 cell order → `tsp_geo::refine_order` | `"tsphybrid"`, `"hybrid"` |
| `Custom(String)` | untouched plugin dispatch | — |

**Dispatch** (`crates/algorithms/src/routing/mod.rs`): cluster centers → `Vec<GeoPoint>` (via geo-types interop) → solve/refine → reorder clusters by returned indices. All `two_opt::optimize` call sites replaced.

**SolverConfig construction** (one place, not exposed on the wire):

```rust
SolverConfig { max_neighbors: 20, ..Default::default() }
// time_limit: None → auto 2–120 s by n; seed: 12345; threads: 0 (all cores)
```

**Default flip** (`crates/koji-service/src/public/v2/calc.rs:237`): Route `sort_by: Unset` → `SortBy::Tsp`.

**Options list** (`all_routing_options()`): gains the hybrid entry; web client route-calc panel picks it up if list-driven (verify during planning).

## Part 3 — Deletions

| Target | Action |
|---|---|
| `crates/algorithms/src/routing/two_opt.rs` | delete (replaced by tsp-mt) |
| `or-tools/` dir incl. `src/tsp/koji_tsp.cc` | delete — all C++ gone |
| Dockerfile or-tools build stage (~lines 17–46) + plugin binary copy | delete — image loses the C++ toolchain layer |
| `plugins/tsp/plugin.toml` | delete |
| docker-compose plugins volume mount | **keep** (plugin machinery lives for external users); update comment if it names or-tools |
| README/docs mentions of or-tools / built-in tsp plugin | sweep + update |
| `crates/algorithms/examples/tsp_route.rs` | rewrite: S2Cell vs Tsp vs TspHybrid comparison |

## Error handling

- `n < 2`: identity order, solver skipped.
- `tsp_geo::Error` (NaN / out-of-range coords, bad seed tour) → log + fall back to the S2-sorted order — this mirrors the existing plugin-failure path in `routing::main`, which returns `SingleVec` (not `Result`); a solver rejection degrades the sort rather than failing the request. No panics reachable from the server path. *(Amended during planning to match `routing::main`'s actual error convention.)*

## Testing

- **tsp-mt (pre-publish):** `refine()` unit tests — permutation validation, result length ≤ initial tour length, same-seed determinism.
- **Koji algorithms:** standalone tour length ≤ S2 baseline on fixtures; hybrid length ≤ its S2 seed; migrate existing tests touching `two_opt`. *(Amended: no same-seed determinism tests — the solver's wall-clock budget makes round counts machine-dependent, so identical output across runs isn't guaranteed even with the fixed seed.)*
- **wasm:** `cargo check` for wasm32 via the koji-wasm path (nightly + `-Z build-std`; plain cargo build is a known false-failure trap).
- Full test suite at phase end only.

## Out of scope

- Plugin-system collapse (deliberately kept).
- Any solver-knob wire exposure (revisit if a real need appears).
- route_split/route_join (already removed).
