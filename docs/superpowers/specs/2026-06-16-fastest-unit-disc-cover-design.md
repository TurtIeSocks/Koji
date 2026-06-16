# Fastest clustering → correct unit-disc cover (design)

**Date:** 2026-06-16
**Scope:** `crates/algorithms/src/clustering/fastest.rs` (the `ClusterMode::Fastest` path)
**Status:** approved design, pre-plan

## Problem

`fastest.rs` is the cheapest clustering tier — a grid heuristic that places radius-`r`
discs to cover input points, keeping discs with `>= min_points` members. It was written
years ago (early Rust) and, judged as a *unit-disc cover*, has correctness defects:

1. **Merge is gated on point count.** The merge test is `combined_members > min_points`,
   tangling disc geometry ("do two cells fit one disc") with the output threshold. Points
   spread across 3+ cells never aggregate into one disc, so genuinely-coverable clusters
   are under-covered or dropped.
2. **Pairwise-only merge.** Each cell merges with at most one neighbor (`continue 'cells`
   after the first hit), so a disc that could swallow several adjacent cells eats only one.
3. **Blind placement.** A multi-point cell emits the *grid-cell geometric center*, ignoring
   where the points actually sit. Corner points land at plane-distance ≈ 1 (the disc
   boundary); projection distortion can then push them *outside* `radius` in real
   (Haversine) space → uncovered. This is the `knife_edge` failure the benchmark penalizes.
4. **Nondeterministic.** Merge decisions depend on `HashMap` iteration order, so the same
   input yields different covers run to run.

(The prior cleanup commit `93ad30d` already removed ~55 lines of dead edge-check code,
a write-only `seen_map`, and stringly-typed storage, with output proven unchanged. This
design builds on that cleaned baseline and **does** change behavior.)

## Goals & non-goals

- **Primary — make it correct.** A genuine, deterministic, robustly-covering grid
  disc-cover. This is the "make it correct" phase after "make it work".
- **Secondary — make it good, cheaply.** Small quality tweaks that lower `score_v2`, kept
  only if they pay off.
- **Hard constraint — stay the fastest tier.** Runtime is the reason this mode exists. Any
  change whose benchmark `wall_s` reaches or exceeds `ClusterMode::Fast` is **walked back**.
  Stay O(n) expected, grid-only — no RTree, no candidate lattice, no greedy max-coverage
  selection (those are what `Fast`/`Balanced`/`Crucible` are for).
- **Metric — `score_v2`, not `mygod_score`.** Quality is judged by the benchmark's
  composite v2 score (cluster count + coverage + route estimate + `knife_edge` +
  `multi_covered`/`overlap_excess`), which rewards robust placement and low overlap.
- **Non-goal:** optimality, or approaching the quality of the higher tiers.
- **Unchanged:** the public entry point `pub fn main(input: &SingleVec, radius: f64,
  min_points: usize) -> Vec<[f64; 2]>`.

## Geometry (recap, unchanged)

`Plane::project` scales so the target `radius` maps to **1 unit** in the projected plane.
A cover disc therefore has **radius 1** in plane coordinates; a point is covered iff its
plane-distance to a center is ≤ 1. The grid uses cells of side `√2`, whose half-diagonal is
exactly 1 — so a radius-1 disc at a cell's center covers the entire cell. The benchmark
measures coverage as Haversine ≤ `radius` on the **reversed** (lat/lon) centers, so the
plane→real distortion is real but small for typical radii; a tunable `margin` absorbs it.

## Approach: region-growing grid cover

Chosen structure (option **A** of the three considered; **B** = fixed-up pairwise is the
fallback if A nears the `Fast` runtime line; **C** = no-merge is the quality floor):

```
project
  → bucket points into a √2 grid (HashMap<(i32,i32), Cell>)
  → region-grow: iterate occupied cells in SORTED order; each unclaimed cell seeds a
       group and absorbs its sorted unclaimed 8-neighbours while the group still fits
       one radius-1 disc (placement-specific fit test)
  → place one center per group (placement strategy)
  → drop groups with member count < min_points
  → dedup coincident centers
  → reverse (un-project to lat/lon)
```

Transitive absorption (not pairwise) yields fewer discs. Sorted iteration + sorted
neighbour order make the output a deterministic *set*, independent of `HashMap` seed.

### Pluggable placement (the experiment)

A private `enum Placement { Mec, Centroid, BboxCenter }` selects both the center
computation and its matching fit-test, so a single code path can benchmark all three. The
user asked to "try each and see how they score."

| Variant | Center | Group fits one disc iff | Notes |
|---|---|---|---|
| `Mec` | center of smallest enclosing circle of the group's points | MEC radius ≤ `1 − margin` | textbook UDC placement; minimal covering radius; lowest `knife_edge`; most code |
| `Centroid` | mean of the group's points | union-bbox diagonal ≤ `2 − margin` | cheap; centroid may sit at boundary for wide groups |
| `BboxCenter` | center of the group's union bbox | union-bbox diagonal ≤ `2 − margin` | closest to today; blind to point positions |

`margin` is a small constant (default near 0), raised only if the benchmark shows
`knife_edge`. After benchmarking, the **winning variant is hard-coded and the losers
deleted** — no dead variants ship.

### MEC implementation

`crates/algorithms/src/sec/` already implements a smallest-enclosing-circle, **but** in
Haversine space and via a **randomized** Welzl (`multi_attempt` shuffles points with an RNG
and retries). That is the wrong metric for the projected plane and its RNG would defeat
determinism, so it is **not reused**. The `Mec` variant gets a small **deterministic
Euclidean** smallest-enclosing-circle (Welzl, fixed input order, ~40 lines) operating on
plane `geo::Coord`s, local to the module. (Welzl is O(k) expected; groups are a few cells'
worth of points, so total cost stays O(n) expected.)

### `min_points` handling

Default: **drop** the points of sub-threshold groups (today's semantics — keeps the tier
honest; sparse points that can't form a valid disc are not covered). A cheap
**reassign-to-an-adjacent-kept-disc** pass will be measured; it ships only if it improves
`score_v2` coverage without crossing the `Fast` runtime line.

## Data structures

- `type CellKey = (i32, i32)` — grid coordinate (`floor(x/√2)`, `floor(y/√2)`).
- `struct Cell { /* members or member coords + count */ }` — per-cell accumulation. Region
  growing needs the **member coordinates** (MEC/centroid need actual points), so `Cell`
  carries the points in the cell (a small `Vec<Coord>`), unlike the count-only cleanup
  baseline. This is a deliberate reversal of the prior `usize`-count change, justified by
  the new placement logic actually consuming point positions.
- Sorted iteration via `BTreeMap` keyed by `CellKey`, or collect+sort keys from a `HashMap`
  — chosen for whichever is cleaner/faster; both give determinism.

## Determinism

Output is a deterministic set: cells iterated in sorted `CellKey` order, neighbours
absorbed in fixed compass order, centers de-duplicated by coordinate bits. The existing
`deterministic_same_input` test is strengthened from len-equality to **set-equality**.

## Testing

Unit tests (module-local, black-box on `main` where possible):
- **Coverage property:** for a constructed input, every retained disc's assigned points are
  within `radius` (in-plane ≤ 1, allowing `margin`) of its center.
- **Determinism:** same input twice → identical *set* of centers (not just same count).
- **Disc count ≤ input count**; empty input → empty; single point + `min_points=1` → 1.
- **MEC unit tests:** known small cases (1/2/3 points, collinear, cocircular).
- Preserve the existing `main`-level tests.

## Benchmark plan & the runtime gate

Run `clusterbench` (binary `algorithms/src/bin/clusterbench.rs`) with a fixed seed across:
- datasets: `uniform`, `blobs`, `urban`
- a couple of `--radius` values and `--min-points` values

Compare, for each cell of that matrix, **`score_v2` and `wall_s`** across:
`fastest` (today's baseline) · each `Placement` variant · `fast` (the ceiling).

**Hard gate:** any variant with `wall_s ≥ fast.wall_s` is walked back. Among variants that
stay clearly under `fast`, pick the best `score_v2`. Record the comparison in the PR/commit
message. If region-growing (A) itself nears `fast`, fall back to fixed-up pairwise (B).

## File organization

Expected to stay well under ~300 lines, so a single `fastest.rs` is retained. If the MEC
helper + region-growing pushes it past the ~500-line split threshold, extract a sibling
`fastest/` module (`fastest/mod.rs` + `fastest/mec.rs`).

## Risks

- **Projection distortion** could make a plane-valid cover miss in Haversine space →
  mitigated by `margin`, quantified by the benchmark's `knife_edge`.
- **MEC cost** could push runtime toward `Fast` → mitigated by the hard runtime gate and
  the `Centroid`/`BboxCenter` fallbacks.
- **Region-grow ordering** must be fully deterministic → enforced by sorted iteration and
  covered by the set-equality determinism test.

## Benchmark results (2026-06-16)

Matrix: `{uniform, blobs, urban} × radius {70, 150} × min_points {1, 3}`, n=10000, seed 42
(12 cells). `score_v2` priced with `λ_knife = 0.5`, `λ_overlap = 0.1`, `λ_route = 0`
(cluster count is identical across placements, so the route term is placement-insensitive).

**Key structural finding:** with `margin = 0` the three fit-tests are mathematically
equivalent — `bbox-diagonal ≤ 2 ⟺ MEC-radius ≤ 1 ⟺ "fits a radius-1 disc"` — so all three
placements produce the *same grouping and same cluster count* (Σclusters = 20465). They
differ only in where the center sits inside each group, making the experiment a pure
coverage + knife_edge + overlap contest.

| variant | Σclusters | Σuncovered | Σknife | Σexcess | Σmygod | Σscore_v2 | Σwall_s |
|---|---|---|---|---|---|---|---|
| **mec** ✅ | 20465 | **6843** | **1230** | 50666 | 44588 | **50270** | 0.15 |
| bbox | 20465 | 6853 | 1959 | 49652 | 44598 | 50542 | 0.13 |
| centroid | 20465 | 9121 | 1419 | 54308 | 46866 | 53004 | 0.13 |
| _baseline (pre-redesign)_ | 20591 | 10809 | 1693 | 46718 | 52715 | 0.13 |
| _fast (next tier, ceiling)_ | 16400 | 7393 | 12775 | 42035 | 36565 | 47156 | 1.07 |

**Decision: `Placement::Mec`, `margin = 0`.** MEC wins `score_v2`, driven by the lowest
`knife_edge` (1230 vs bbox's 1959) and best coverage; centroid genuinely under-covers
(9121 uncovered). Runtime gate passed by a wide margin — every placement runs at
`cluster_s ≤ 0.01`, **5–16× faster than `fast`** (0.03–0.16), so MEC's Welzl cost is free at
these scales. A positive `margin` was rejected: it would add clusters (cost ≥ `min_points`
each) to shave diffuse ~1% knife points priced at 0.5 — net-negative for `score_v2`.

**vs. pre-redesign baseline:** −37% uncovered points (10809 → 6843, the under-coverage bug
fixed), −4.6% mygod, −4.6% score_v2, lower knife_edge, slightly fewer clusters, now fully
deterministic — at ~unchanged runtime. `fast` remains better (fewer clusters) as expected;
Fastest's job is a *correct, much cheaper* cover, which it now is.
