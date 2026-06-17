# S2-seeded TSP routing (clean-room 2-opt + Or-opt)

**Date:** 2026-06-17
**Status:** design approved, awaiting plan
**Scope:** sub-project B of the routing work. A new opt-in routing strategy.

## Problem

Koji's built-in `SortBy` routing strategies are all cheap single-axis sorts
(lat/lon, geohash Z-order, S2 Hilbert, density, random) — none optimize the
*tour*. The device consumes the result as a cycle, so for real (non-grid) route
points the visiting order has crossings and a long closing leg. Sub-project A
(comb) fixes the bootstrap **grid** case; this fixes the **general** case.

Pokémon GO data is S2-based, and an S2 Hilbert sort already gives good spatial
locality — a strong starting tour. What's missing is a refinement pass that
optimizes the closed cycle (connecting the "clusters of clusters" and removing
the long return edge).

## Goal

Add an opt-in `SortBy::Tsp` that seeds with the S2 Hilbert order and refines it
with a small, clean-room 2-opt + Or-opt local search over the closed cycle —
deterministic, dependency-free, wasm-native.

## Non-goals

- **Not the full LKH-3.** No alpha-nearness candidate sets, MST lower bounds,
  sequential k-opt, or partitioning. Clean-room — we do not vendor or depend on
  `lkh-rs` (research-use license, `publish = false`). 2-opt + Or-opt from a
  space-filling-curve seed lands within a few percent of optimal for geographic
  point sets, which is ample for device routing.
- **Not the default.** Opt-in `SortBy` variant; existing default routing
  (`Unset` passthrough; bootstrap radius comb) is unchanged.
- No new config fields. `k`, the pass cap, and ε are internal constants.

## Design

### Pipeline

When `sort_by == Tsp`, `routing::main` runs:

1. **Seed** — `clusters.sort_s2()` (existing) → S2 Hilbert order.
2. **Refine** — `two_opt::optimize(seed)` → 2-opt + Or-opt local search over the
   closed cycle.
3. `utils::rotate_to_best` anchors the start as today (unchanged).

No new dependencies (`rstar` and `s2` are already deps; `geo` unused here). Pure
Rust → runs in wasm (built-in `SortBy` arms are not `cfg`-gated). Deterministic.

### The refiner — `routing/two_opt.rs`

Public surface is one function:

```rust
pub fn optimize(order: SingleVec) -> SingleVec
```

It operates on a cyclic tour of indices into the point list.

**Distance.** Precompute each point's `s2::Point` unit vector once (O(N); the
trig happens here). Hot-loop distance between two points is the 3D **chord
length** `(v_i - v_j).norm()` — no transcendentals, one `sqrt`. The
Earth-radius factor is dropped (2-opt/Or-opt gains compare sums of distances; a
constant factor cancels). Squared distance is NOT usable — the gain arithmetic
is a sum/difference of distances, which a non-linear transform breaks. Chord vs
true arc differ by ~θ²/24; at geofence scale (θ ≤ ~1e-4 rad) that is ~1e-9
relative, so gain *signs* match Haversine exactly.

**Candidates.** Build `RTree<[Precision; 2]>` from the points (rstar, raw
lat/lon — Euclidean is fine for *selecting* near neighbors). For each point,
precompute its `k = 8` nearest neighbors via `nearest_neighbor_iter`, clamped to
N−1. Moves are only proposed between a node and one of its k candidates →
~O(N·k) per pass instead of O(N²).

**2-opt.** For node `a` with successor `b`, and each candidate `c` (successor
`d`): `gain = d(a,b) + d(c,d) − d(a,c) − d(b,d)`. If `gain > ε`, reverse the
segment `b…c`. Removes crossings and long edges, including the wrap.

**Or-opt.** Relocate a run of `L ∈ {1, 2, 3}` consecutive nodes next to a
candidate neighbor when it lowers total cycle cost. Handles single nodes
stranded on the wrong row that 2-opt cannot cheaply move.

**Closed cycle.** The edge set includes `last → first`; the closing leg is
optimized like any other edge. This is the core win over the cheap sorts.

**Don't-look bits.** A queue of active nodes; a node is skipped until a move
touches it (its endpoints reactivate). Passes get cheaper as the tour settles;
an empty queue is the convergence signal.

**Termination.** Alternate 2-opt and Or-opt sweeps until a full sweep makes no
improvement (local optimum), with a safety cap of ≤ 60 passes so runtime is
always bounded.

**Determinism.** No RNG. Candidates sorted by (chord, index); nodes processed in
fixed index order; first-improving move applied (not best-of). Same input →
same output.

### Wiring

- `routing/sort_by.rs`: add `Tsp` variant before `Custom(String)`:
  ```rust
  #[str("tsp", alias("optimized", "2opt"))]
  Tsp,
  ```
- `routing/mod.rs`: `mod two_opt;`, dispatch arm
  `SortBy::Tsp => two_opt::optimize(clusters.sort_s2()),`, and
  `options.push("tsp".to_string());` in `all_routing_options`.
- The `routing::main` match is exhaustive; the compiler forces the new arm. Any
  other exhaustive `match` on `SortBy` (e.g. koji-service / wasm bindings) will
  error until handled — follow the compiler. No API schema change beyond the new
  string value flowing through `from_str_opt`.

## Testing

`optimize` is pure and deterministic:

1. **Permutation invariant** — output is a permutation of input (same multiset,
   same length).
2. **Never worsens** — cyclic tour length after `optimize` ≤ seed length.
3. **Uncrosses a known crossing (decisive)** — 4 unit-square corners in a
   self-crossing "bowtie" order → `optimize` returns the perimeter cycle
   (optimal), proving 2-opt fires.
4. **Or-opt relocation** — a layout where a stranded node must be relocated (not
   reversed) → length drops to the known optimum, exercising Or-opt.
5. **Convergence / idempotent** — `len(optimize(optimize(x))) == len(optimize(x))`.
6. **Determinism** — `optimize(x) == optimize(x)` across runs.
7. **Edge cases** — empty → empty; 1/2/3 points → same set.
8. **Integration** — `SortBy::Tsp` through `routing::main` preserves count and
   yields `distance_stats > 0` (mirrors existing per-sort tests).

## Assumptions

- Distance: `s2::Point` 3D chord, no Earth-radius factor; `sqrt` kept.
- Candidates: own `RTree<[Precision; 2]>`, `nearest_neighbor_iter`, k = 8
  (clamped to N−1).
- Acceptance: first-improving move, small positive ε.
- Or-opt segment lengths L ∈ {1, 2, 3}.
- Convergence: no improving move in a full sweep; cap ≤ 60 passes.
- `k`, cap, ε are internal constants — no config fields.
- Seed = existing `sort_s2`; `rotate_to_best` unchanged after.
- Opt-in only; default routing unchanged; runs in wasm.

## References

- Bootstrap routing work + sub-project A (comb): see
  `docs/superpowers/specs/2026-06-16-radius-comb-ordering-design.md` and the
  matching plan.
- `lkh-rs` at `/Users/rin/GitHub/lkh-rs` — evaluated and deliberately NOT used:
  full LKH-3 is overkill for the hybrid, and its research-use / `publish = false`
  license would entangle Koji. Clean-room 2-opt + Or-opt instead.
- Do NOT build on `route_split_level` / `cluster_split_level` / `join::join` —
  unfinished code slated for removal.
