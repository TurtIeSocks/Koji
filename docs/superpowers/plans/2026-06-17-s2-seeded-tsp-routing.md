# S2-seeded TSP routing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an opt-in `SortBy::Tsp` routing strategy: seed with the existing S2 Hilbert sort, then refine the closed tour with a clean-room 2-opt + Or-opt local search.

**Architecture:** A new `routing/two_opt.rs` exposes `optimize(order) -> SingleVec`. It precomputes each point's S2 unit vector (cheap chord distance, no per-edge trig) and a k-nearest-neighbor candidate list (`rstar`), then runs alternating 2-opt and Or-opt sweeps over the cyclic tour to convergence. `routing::main` gains one dispatch arm `SortBy::Tsp => two_opt::optimize(clusters.sort_s2())`.

**Tech Stack:** Rust; existing deps `s2` (`Point`/`LatLng`), `rstar` (`RTree`, `GeomWithData`), `koji_core` (`Precision`, `SingleVec`). No new deps. Pure → wasm-safe.

**Spec:** `docs/superpowers/specs/2026-06-17-s2-seeded-tsp-routing-design.md`

---

## File Structure

- Create: `crates/algorithms/src/routing/two_opt.rs` — the refiner (`optimize` + private passes + tests).
- Modify: `crates/algorithms/src/routing/sort_by.rs` — add the `Tsp` variant.
- Modify: `crates/algorithms/src/routing/mod.rs` — `mod two_opt;`, one dispatch arm, one `all_routing_options` entry, one integration test.

---

## Task 1: Scaffold the variant + wiring (identity refiner)

Establish the module, the `SortBy::Tsp` variant, and the dispatch — with `optimize` as the identity function — so the pipeline works end-to-end and the permutation/edge/integration tests pass before any local search exists.

**Files:**
- Create: `crates/algorithms/src/routing/two_opt.rs`
- Modify: `crates/algorithms/src/routing/sort_by.rs`, `crates/algorithms/src/routing/mod.rs`

- [ ] **Step 1: Create `two_opt.rs` with an identity `optimize` + tests**

```rust
//! S2-seeded local-search route refinement (2-opt + Or-opt over the closed
//! tour). Opt-in via `SortBy::Tsp`. Deterministic, dependency-light, wasm-safe.

use koji_core::{Precision, SingleVec};

/// Refine a closed-tour visiting order (each point `[lat, lon]`) with 2-opt +
/// Or-opt local search. Returns a permutation of the input that lowers the
/// cyclic tour length. Deterministic.
pub fn optimize(order: SingleVec) -> SingleVec {
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sum of unit-sphere chord lengths around the closed tour (incl. wrap).
    fn tour_len(order: &SingleVec) -> Precision {
        use s2::{latlng::LatLng, point::Point};
        let v: Vec<(f64, f64, f64)> = order
            .iter()
            .map(|p| {
                let pt = Point::from(LatLng::from_degrees(p[0], p[1])).0;
                (pt.x, pt.y, pt.z)
            })
            .collect();
        let chord = |a: (f64, f64, f64), b: (f64, f64, f64)| {
            ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
        };
        let n = v.len();
        if n < 2 {
            return 0.0;
        }
        (0..n).map(|i| chord(v[i], v[(i + 1) % n])).sum()
    }

    fn sorted_xy(order: &SingleVec) -> Vec<(i64, i64)> {
        let mut s: Vec<(i64, i64)> = order
            .iter()
            .map(|p| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64))
            .collect();
        s.sort();
        s
    }

    #[test]
    fn optimize_is_permutation() {
        let order: SingleVec = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        let out = optimize(order.clone());
        assert_eq!(sorted_xy(&out), sorted_xy(&order), "same multiset");
    }

    #[test]
    fn optimize_edge_cases_return_same_set() {
        for order in [
            SingleVec::new(),
            vec![[1.0, 2.0]],
            vec![[1.0, 2.0], [3.0, 4.0]],
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
        ] {
            let out = optimize(order.clone());
            assert_eq!(sorted_xy(&out), sorted_xy(&order));
        }
    }

    #[test]
    fn optimize_never_worsens() {
        let order: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let before = tour_len(&order);
        let after = tour_len(&optimize(order));
        assert!(after <= before + 1e-12, "tour must not get longer");
    }

    #[test]
    fn optimize_is_deterministic() {
        let order: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        assert_eq!(optimize(order.clone()), optimize(order));
    }
}
```

- [ ] **Step 2: Run the new module tests — they pass on identity**

Run: `cargo test -p algorithms --lib routing::two_opt`
Expected: PASS (4 tests). Identity is a valid permutation that never worsens.

- [ ] **Step 3: Add the `Tsp` variant in `sort_by.rs`**

In `crates/algorithms/src/routing/sort_by.rs`, add the variant immediately before `Custom(String)`:

```rust
    #[str("tsp", alias("optimized", "2opt"))]
    Tsp,
    #[str(default)]
    Custom(String),
```

- [ ] **Step 4: Wire dispatch + options in `mod.rs`**

In `crates/algorithms/src/routing/mod.rs`:

Add the module declaration near the other `mod` lines (after `pub mod sorting;`):
```rust
mod two_opt;
```

Add the dispatch arm inside the `match &cfg.sort_by` block, next to the other built-ins (e.g. after the `SortBy::Random` arm):
```rust
        SortBy::Tsp => two_opt::optimize(clusters.sort_s2()),
```

Add the option string in `all_routing_options`, after `options.push("s2".to_string());`:
```rust
    options.push("tsp".to_string());
```

- [ ] **Step 5: Add the integration test in `mod.rs`**

In the `#[cfg(test)] mod tests` of `crates/algorithms/src/routing/mod.rs`, add (mirrors `s2_preserves_all_clusters`):

```rust
    #[test]
    fn tsp_preserves_all_clusters_and_routes() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(&data, clusters.clone(), 70.0, &make_cfg(SortBy::Tsp), &mut stats);
        assert_eq!(out.len(), clusters.len());
        assert!(stats.total_distance > 0.0);
    }
```

Also extend the built-ins assertion in `all_routing_options_includes_built_ins`:
```rust
        assert!(opts.contains(&"tsp".to_string()));
```

- [ ] **Step 6: Build, test, lint**

Run (single batch):
- `cargo test -p algorithms --lib routing`
- `cargo clippy -p algorithms --lib`

Expected: all routing tests pass (including `tsp_preserves_all_clusters_and_routes`); clippy clean. The compiler will flag any other exhaustive `match` on `SortBy` (e.g. in koji-service / wasm bindings) — add a `SortBy::Tsp` arm wherever it errors, routing the same as `SortBy::S2Cell` (or mapping the string `"tsp"`), and re-run.

- [ ] **Step 7: Commit**

```bash
git add crates/algorithms/src/routing/
git commit -m "feat(routing): scaffold opt-in SortBy::Tsp (identity refiner)

New routing/two_opt.rs with an identity optimize(), plus SortBy::Tsp
variant, dispatch (sort_s2 then refine), and all_routing_options entry.
Local search lands in the next commits. Wired end-to-end with
permutation/edge/determinism/integration tests.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: 2-opt refinement

Replace the identity body with distance + candidate infrastructure and a 2-opt convergence loop.

**Files:**
- Modify: `crates/algorithms/src/routing/two_opt.rs`

- [ ] **Step 1: Add the failing 2-opt test**

Add to `mod tests` in `two_opt.rs`:

```rust
    #[test]
    fn two_opt_uncrosses_bowtie() {
        // Unit-square corners fed in a self-crossing order (two diagonals).
        // Optimal closed tour is the perimeter; 2-opt must uncross it.
        let crossed: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let out = optimize(crossed.clone());
        let perim = tour_len(&vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert!(tour_len(&out) < tour_len(&crossed) - 1e-9, "must shorten");
        assert!(
            (tour_len(&out) - perim).abs() < 1e-9,
            "must reach the perimeter optimum"
        );
    }

    #[test]
    fn two_opt_idempotent_at_local_optimum() {
        let crossed: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let once = optimize(crossed);
        let twice = optimize(once.clone());
        assert!((tour_len(&twice) - tour_len(&once)).abs() < 1e-12);
    }
```

- [ ] **Step 2: Run to verify the bowtie test fails**

Run: `cargo test -p algorithms --lib routing::two_opt::tests::two_opt_uncrosses_bowtie`
Expected: FAIL — identity returns the crossed tour, so the perimeter assertion fails.

- [ ] **Step 3: Implement distance, candidates, and the 2-opt loop**

Replace the entire `optimize` function (and add the imports + consts at the top of the file) in `two_opt.rs`:

```rust
use koji_core::{Precision, SingleVec};
use rstar::{primitives::GeomWithData, RTree};
use s2::{latlng::LatLng, point::Point};

const K: usize = 8; // candidate neighbors per node
const MAX_PASSES: usize = 60;
const EPS: Precision = 1e-9; // unit-sphere chord units

type IdxPt = GeomWithData<[Precision; 2], usize>;

/// Refine a closed-tour visiting order (each point `[lat, lon]`) with 2-opt +
/// Or-opt local search. Returns a permutation of the input that lowers the
/// cyclic tour length. Deterministic.
pub fn optimize(order: SingleVec) -> SingleVec {
    let n = order.len();
    if n < 4 {
        return order; // a 0–3 node cycle has nothing to improve
    }

    // 3D unit vectors → cheap chord distance (no per-edge trig).
    let verts: Vec<(f64, f64, f64)> = order
        .iter()
        .map(|p| {
            let v = Point::from(LatLng::from_degrees(p[0], p[1])).0;
            (v.x, v.y, v.z)
        })
        .collect();
    let dist = |i: usize, j: usize| -> Precision {
        let a = verts[i];
        let b = verts[j];
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
    };

    // k-nearest-neighbor candidate lists (rstar on raw lat/lon; Euclidean is
    // fine for *selecting* near neighbors). Sorted by (chord, index) for
    // determinism.
    let tree: RTree<IdxPt> = RTree::bulk_load(
        order
            .iter()
            .enumerate()
            .map(|(i, p)| GeomWithData::new([p[0], p[1]], i))
            .collect(),
    );
    let candidates: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let q = [order[i][0], order[i][1]];
            let mut c: Vec<usize> = tree
                .nearest_neighbor_iter(&q)
                .filter(|g| g.data != i)
                .take(K)
                .map(|g| g.data)
                .collect();
            c.sort_by(|&x, &y| {
                dist(i, x)
                    .partial_cmp(&dist(i, y))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(x.cmp(&y))
            });
            c
        })
        .collect();

    // Tour as a permutation of node ids (0..n); pos is its inverse.
    let mut tour: Vec<usize> = (0..n).collect();
    let mut pos: Vec<usize> = (0..n).collect();

    let mut pass = 0;
    loop {
        let mut improved = false;

        // ── 2-opt: for edge (a,b=succ a) and candidate c (d=succ c), try
        //    replacing (a,b)+(c,d) with (a,c)+(b,d) by reversing b..c.
        for a in 0..n {
            let pa = pos[a];
            let b = tour[(pa + 1) % n];
            for &c in &candidates[a] {
                if c == b {
                    continue;
                }
                let pc = pos[c];
                let d = tour[(pc + 1) % n];
                if d == a {
                    continue; // edges adjacent — degenerate
                }
                let gain = dist(a, b) + dist(c, d) - dist(a, c) - dist(b, d);
                if gain > EPS {
                    let (lo, hi) = if pa < pc { (pa, pc) } else { (pc, pa) };
                    tour[lo + 1..=hi].reverse();
                    for p in lo + 1..=hi {
                        pos[tour[p]] = p;
                    }
                    improved = true;
                    break; // first-improvement; advance to next a
                }
            }
        }

        pass += 1;
        if !improved || pass >= MAX_PASSES {
            break;
        }
    }

    tour.into_iter().map(|i| order[i]).collect()
}
```

- [ ] **Step 4: Run the 2-opt tests**

Run: `cargo test -p algorithms --lib routing::two_opt`
Expected: PASS — `two_opt_uncrosses_bowtie`, `two_opt_idempotent_at_local_optimum`, and all Task 1 tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/routing/two_opt.rs
git commit -m "feat(routing): 2-opt refinement for SortBy::Tsp

optimize() now builds S2 unit-vector chord distances + rstar k-NN
candidate lists, then runs first-improvement 2-opt over the closed tour
to convergence (capped). Uncrosses the seed and removes long edges incl.
the wrap. Deterministic.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 3: Or-opt refinement

Add Or-opt (relocate runs of 1–3 nodes) to the convergence loop.

**Files:**
- Modify: `crates/algorithms/src/routing/two_opt.rs`

- [ ] **Step 1: Add the failing Or-opt test**

Add to `mod tests`:

```rust
    #[test]
    fn reaches_hexagon_optimum() {
        // 6 points on a circle; the optimal closed tour is the convex
        // perimeter. Seed scrambled. Exercises 2-opt + Or-opt jointly.
        let mut circle: SingleVec = (0..6)
            .map(|k| {
                let t = std::f64::consts::TAU * (k as f64) / 6.0;
                [t.sin(), t.cos()] // [lat, lon] on a small circle near (0,0)
            })
            .collect();
        let perim = tour_len(&circle);
        // Scramble into a crossing order.
        circle.swap(1, 4);
        circle.swap(2, 5);
        let out = optimize(circle.clone());
        assert!(tour_len(&out) < tour_len(&circle) - 1e-9, "must shorten");
        assert!(
            (tour_len(&out) - perim).abs() < 1e-9,
            "must reach the convex perimeter optimum"
        );
    }
```

- [ ] **Step 2: Run to verify it fails (or is suboptimal) without Or-opt**

Run: `cargo test -p algorithms --lib routing::two_opt::tests::reaches_hexagon_optimum`
Expected: FAIL or not-yet-optimal on some scrambles with 2-opt alone. (If it already passes, Or-opt still adds the relocation capability the spec requires; proceed to add it.)

- [ ] **Step 3: Add the Or-opt pass into the loop**

In `optimize`, inside the `loop { ... }`, AFTER the 2-opt `for a in 0..n { ... }` block and BEFORE `pass += 1;`, insert the Or-opt block:

```rust
        // ── Or-opt: relocate a run of L∈{1,2,3} nodes next to a candidate.
        for l in 1..=3usize {
            if n < l + 2 {
                continue;
            }
            for s in 0..n {
                let p0 = pos[s];
                let seg: Vec<usize> = (0..l).map(|t| tour[(p0 + t) % n]).collect();
                let first = seg[0];
                let last = seg[l - 1];
                let prev = tour[(p0 + n - 1) % n];
                let nxt = tour[(p0 + l) % n];
                if seg.contains(&prev) || seg.contains(&nxt) {
                    continue; // wraps onto itself (tiny n)
                }
                // Saving from removing the segment and closing the gap.
                let removed = dist(prev, first) + dist(last, nxt) - dist(prev, nxt);
                if removed <= EPS {
                    continue; // insertion cost is ≥0, so no net gain possible
                }
                let mut applied = false;
                for &c in candidates[first].iter().chain(candidates[last].iter()) {
                    if seg.contains(&c) || c == prev {
                        continue;
                    }
                    let e = tour[(pos[c] + 1) % n];
                    if seg.contains(&e) {
                        continue;
                    }
                    let base = dist(c, e);
                    let add_f = dist(c, first) + dist(last, e) - base;
                    let add_r = dist(c, last) + dist(first, e) - base;
                    let (add, rev) = if add_r < add_f {
                        (add_r, true)
                    } else {
                        (add_f, false)
                    };
                    if removed - add > EPS {
                        let seg_set: std::collections::HashSet<usize> =
                            seg.iter().copied().collect();
                        let mut rest: Vec<usize> =
                            tour.iter().copied().filter(|x| !seg_set.contains(x)).collect();
                        let cpos = rest.iter().position(|&x| x == c).unwrap();
                        let mut ins = seg.clone();
                        if rev {
                            ins.reverse();
                        }
                        rest.splice(cpos + 1..cpos + 1, ins);
                        tour = rest;
                        for (p, &node) in tour.iter().enumerate() {
                            pos[node] = p;
                        }
                        improved = true;
                        applied = true;
                        break;
                    }
                }
                let _ = applied;
            }
        }
```

- [ ] **Step 4: Run the Or-opt tests**

Run: `cargo test -p algorithms --lib routing::two_opt`
Expected: PASS — `reaches_hexagon_optimum` plus all prior tests green.

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/routing/two_opt.rs
git commit -m "feat(routing): Or-opt relocation for SortBy::Tsp

Add Or-opt (relocate runs of 1–3 nodes, optionally reversed, next to a
candidate) into the convergence loop alongside 2-opt. Fixes nodes 2-opt
can't cheaply move. Completes the clean-room S2-seeded refiner.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 4: Verification gate

**Files:** none (verification only).

- [ ] **Step 1: Full suite + lint**

Run (single batch):
- `cargo test -p algorithms --lib`
- `cargo clippy -p algorithms --all-targets`

Expected: all pass; clippy clean. Capture the real exit code (don't pipe through `tail`/`grep` alone — a pipeline masks cargo's status).

- [ ] **Step 2: Confirm wasm exposure is intact**

Confirm the `SortBy::Tsp` arm in `routing::main` is NOT behind `#[cfg(feature = "native")]` (only the `Custom` plugin arm is). `two_opt` uses only `s2` + `rstar` + `koji_core`, all wasm-safe, so `tsp` routing works in wasm. No edit expected — just verify.

---

## Self-Review

**Spec coverage:**
- Pipeline (sort_s2 seed → optimize → rotate_to_best unchanged) → Task 1 Step 4 dispatch. ✓
- Chord distance via s2::Point, no R factor, sqrt kept → Task 2 Step 3 `dist`. ✓
- k-NN candidates via rstar, k=8, clamped, deterministic sort → Task 2 Step 3. ✓
- 2-opt (gain, reverse b..c, cyclic) → Task 2 Step 3. ✓
- Or-opt (L∈{1,2,3}, forward/reversed insertion, cyclic) → Task 3 Step 3. ✓
- Closed cycle (wrap via `% n` everywhere) → both passes. ✓
- Convergence + cap (≤60) → loop in Task 2/3. ✓
- Determinism (no RNG, fixed order, first-improvement, sorted candidates) → ✓; tested in Task 1.
- Wiring (variant, dispatch, options, exhaustive-match fixups) → Task 1 Steps 3–6. ✓
- Opt-in, no config fields → no config touched. ✓
- Testing: permutation, never-worsens, bowtie (2-opt), hexagon optimum (Or-opt+2-opt), idempotent, determinism, edge cases, integration → Tasks 1–3. ✓

**Deviation from spec (flagged):** the spec described *don't-look bits* as the convergence mechanism. The plan implements the equivalent simpler form — full alternating sweeps repeated until a sweep yields no improving move (capped at 60). Same local-optimum result; don't-look bits are a pure-performance optimization deferred until profiling shows the sweeps are a bottleneck (they are not at expected N, runtime is tens of ms). Or-opt's splice does a full O(n) `pos` rebuild per applied move — simple and correct; also a future perf target.

**Placeholder scan:** no TBD/TODO; all code complete. `let _ = applied;` silences an unused-binding lint where `applied` documents intent; harmless.

**Type consistency:** `optimize(SingleVec) -> SingleVec`; `Precision` = f64; `order[i]` = `[lat, lon]`; `LatLng::from_degrees(lat, lon)` = `from_degrees(order[i][0], order[i][1])`; `Point(pub Vector{x,y,z})` accessed via `.0` then `.x/.y/.z`; `GeomWithData::new([lat,lon], idx)` with `.data` = idx. Consistent across tasks.
