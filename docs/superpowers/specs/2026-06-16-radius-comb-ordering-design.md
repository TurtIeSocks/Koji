# Radius bootstrap — comb visiting order

**Date:** 2026-06-16
**Status:** design approved, awaiting plan
**Scope:** sub-project A of the routing closing-leg work. Radius bootstrap only.

## Problem

The radius bootstrap (`CalculationMode::Radius`) tiles a geofence polygon with a
staggered hex lattice of circle centers, then a device visits them in a loop
(last point → back to first, forever). The complaint: that closing leg is
usually very long.

Root cause (confirmed by audit, 2026-06-16):

- `generate_circles` ([radius.rs:109](../../../crates/algorithms/src/bootstrap/radius.rs))
  emits a **boustrophedon snake** — rows top→bottom, sweep direction flipping
  each row. A snake ends at the corner opposite its start.
- The default `sort_by` is `Unset` ([routing/mod.rs:35](../../../crates/algorithms/src/routing/mod.rs)),
  a passthrough, so the generation order survives to output.
- The consumer treats the list as a **cycle**; `distance_stats` closes
  `last → first`. So the closing leg = the snake's two endpoints = the full
  polygon height, every loop.
- `rotate_to_best` ([utils.rs:110](../../../crates/algorithms/src/utils.rs)) only
  rotates the seam; it cannot shrink a cycle's longest edge.

The built-in `SortBy` strategies are all cheap single-axis sorts, not tours, so
none minimize the wrap edge. A long closing leg is therefore structural for the
radius bootstrap, not a tuning issue.

## Goal

Emit the radius bootstrap points in a **comb** (interleaved boustrophedon-loop)
order so the device's loop has no long edge — closing leg ≈ one row pitch
instead of the full polygon height — at zero extra routing cost.

## Non-goals (explicitly deferred)

- **S2 bootstrap** comb ordering. Tractable later (cells at a fixed level have a
  known row height), but out of scope here.
- **Cross-polygon optimization** for multipolygon geofences. Handed to
  sub-project B.
- **Sub-project B — S2-seeded LKH router.** The general routing technique for
  real (non-grid) route points: build a TSP instance (native GEO distance via
  `lkh-core`), seed it with the S2 Hilbert order as the initial tour, let
  Lin-Kernighan refine. Separate brainstorm/spec. Comb is the cheap exact path
  for the bootstrap **grid**; B is for everything else.
- No new `SortBy` variant, no new config field. (The heuristic row-binning
  `SortBy::Comb` once considered for arbitrary points is dropped — B supersedes
  it.)

## Design

### Invariant

Comb is a **pure reordering** of the points `generate_circles` already
produces. Same point set, same coverage — only the visiting sequence changes.
Orthogonal to the row-pitch coverage fix (commit 5c29434).

### Comb algorithm

Walk **down the even rows, then back up the odd rows**, orienting each row to
enter at the end nearest the previous row's exit:

- Index order: even rows ascending `0, 2, 4, …`, then odd rows descending
  `…, 5, 3, 1`.
- For each row, choose orientation (as-emitted or reversed) so its first point
  is the end nearer the previous row's last point — keeps every connector
  minimal.
- The `row0 / row1` wrap is oriented so the closing leg (`last → first`) is one
  row pitch: row 1 is adjacent to row 0, and row 1's exit is aligned to row 0's
  entry side.

Guarantees:

- Every hop between consecutive points ≤ **2 row pitches** (the even→even and
  odd→odd skips).
- Closing leg ≈ **1 row pitch** (`y_mod · radius · 2 = 1.5 · radius`).
- No long edge anywhere in the loop.

Edge cases: 0 rows → empty; 1 row → that row unchanged (closing leg ~0);
≥2 rows → the interleave above.

### Code placement

All inside [radius.rs](../../../crates/algorithms/src/bootstrap/radius.rs), no
routing changes:

1. **`generate_circles`** — the outer `while` loop already delimits one row per
   iteration. Collect `rows: Vec<Vec<Point>>` (push into a per-row `Vec`,
   `rows.push(row)` at the bottom of each iteration) instead of a flat snake
   `Vec`. Keep-condition, lattice math, south-step, and bearing flip are
   untouched.
2. **New private `fn comb_order(rows: Vec<Vec<Point>>) -> Vec<Point>`** — pure,
   testable. Implements the even-down/odd-up interleave with greedy per-row
   orientation and the row0/row1 wrap handling. Returns the flattened comb
   permutation.
3. **`generate_circles` returns `comb_order(rows)`**; `run` / `flatten_circles`
   consume it unchanged.

### Routing interaction

Comb becomes the new default radius emission order (replaces the snake — same
points, tighter loop). Default `sort_by = Unset` preserves it to output. An
explicit `s2`/`geohash`/`latlon`/etc. choice overrides the comb — the user's
opt-out, behavior unchanged.

### Multipolygon

`flatten_circles` splits a `MultiPolygon` and combs each sub-polygon
([radius.rs:95](../../../crates/algorithms/src/bootstrap/radius.rs),
order-preserving `flat_map`): output = `[comb(poly0), comb(poly1), …]`. Each
polygon's own loop is tight; inter-polygon hops are left as-is (cross-polygon
optimization belongs to B). Single-polygon geofences — the common case — are
fully solved.

## Testing

`comb_order` is a pure fn, so testing is cheap and direct:

1. **Permutation invariant** — `comb_order(rows)` output is a permutation of all
   input row points (same length, same multiset). The coverage guarantee.
2. **Worst-hop / closing-leg property** — on a tall rectangle: assert comb's
   closing leg `Haversine(last, first)` is small (≤ ~3 · row_pitch) and far
   shorter than the snake's (≈ full polygon height). Optionally assert max
   consecutive hop ≤ ~2 row pitches. Pins the Q3 win.
3. **Edge cases** — empty → empty; 1 row → same set; 2 and ≥3 rows → correct
   even-down/odd-up order with the wrap oriented to one pitch.
4. **Determinism** — no RNG; same input → same output.
5. **Existing radius tests** stay green (a permutation preserves non-empty /
   valid lat-lon / empty-on-no-geometry).

## References

- Bootstrap audit, 2026-06-16 (closing-leg root cause + fixes).
- Coverage row-pitch fix: commit 5c29434.
- S2 block-center fix: commit bac5176.
- `lkh-core` (sub-project B backend): `/Users/rin/GitHub/lkh-rs` — embeddable
  library, native GEO distance, deterministic with fixed seed, wasm-capable
  core, accepts an initial tour. Path-dep / `publish = false` / research-use
  license.
