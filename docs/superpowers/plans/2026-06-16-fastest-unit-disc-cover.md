# Fastest Unit-Disc-Cover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `ClusterMode::Fastest` grid heuristic with a correct, deterministic, region-growing unit-disc cover whose disc-placement strategy is benchmark-selected — without exceeding `ClusterMode::Fast`'s runtime.

**Architecture:** Project points → bucket into a √2 grid → region-grow groups in sorted (deterministic) order, absorbing neighbours while the group still fits one radius-1 disc → place one center per group (via a swappable `Placement` strategy) → drop sub-`min_points` groups → dedup → reverse. Three placement variants (`Mec`/`Centroid`/`BboxCenter`) are implemented behind a private enum, benchmarked, and the winner is hard-coded while the losers are deleted.

**Tech Stack:** Rust (edition 2024), `geo::Coord`, `std::collections::{BTreeMap, BTreeSet, HashMap}`. No new deps. Disc geometry lives in the projected plane (radius = 1 unit).

**Spec:** `docs/superpowers/specs/2026-06-16-fastest-unit-disc-cover-design.md`

**File:** all code changes are in `crates/algorithms/src/clustering/fastest.rs` (single file; tests in its `#[cfg(test)] mod tests`). The benchmark binary `crates/algorithms/src/bin/clusterbench.rs` is *run*, not modified.

---

## Conventions used throughout

- Plane coordinates are `geo::Coord { x: f64, y: f64 }`. A cover disc has **radius 1** in the plane.
- `const MARGIN: f64 = 0.0;` — distortion safety margin; starts at 0, tuned only if the benchmark shows `knife_edge`.
- Containment epsilon: `const EPS: f64 = 1e-7;` (local plane coords near a center are O(1), so absolute epsilon is fine).
- Cell side is `√2`; `CellKey = (floor(x/√2), floor(y/√2))`.

---

## Task 1: Deterministic Euclidean smallest-enclosing-circle (MEC)

**Files:**
- Modify: `crates/algorithms/src/clustering/fastest.rs` (add geometry helpers above `cluster`)
- Test: same file, `mod tests`

- [ ] **Step 1: Write the failing tests**

Add to `mod tests`:

```rust
// ── MEC: geometry ─────────────────────────────────────────────────────────

fn c(x: f64, y: f64) -> Coord {
    Coord { x, y }
}

fn covers(center: Coord, r: f64, pts: &[Coord]) -> bool {
    pts.iter().all(|p| {
        let dx = p.x - center.x;
        let dy = p.y - center.y;
        dx * dx + dy * dy <= r * r + 1e-6
    })
}

#[test]
fn mec_single_point_is_zero_radius_at_point() {
    let (center, r) = smallest_enclosing_circle(&[c(3.0, -2.0)]).unwrap();
    assert!((center.x - 3.0).abs() < 1e-9 && (center.y + 2.0).abs() < 1e-9);
    assert!(r < 1e-9, "single-point radius should be ~0, got {r}");
}

#[test]
fn mec_two_points_is_diameter_circle() {
    let (center, r) = smallest_enclosing_circle(&[c(0.0, 0.0), c(2.0, 0.0)]).unwrap();
    assert!((center.x - 1.0).abs() < 1e-9 && center.y.abs() < 1e-9);
    assert!((r - 1.0).abs() < 1e-9, "radius should be 1.0, got {r}");
}

#[test]
fn mec_encloses_all_points() {
    let pts = [c(0.0, 0.0), c(1.0, 0.0), c(0.0, 1.0), c(1.0, 1.0), c(0.5, 0.5)];
    let (center, r) = smallest_enclosing_circle(&pts).unwrap();
    assert!(covers(center, r, &pts), "MEC must enclose every point");
    // tightest enclosing circle of the unit square has r = sqrt(2)/2
    assert!((r - std::f64::consts::SQRT_2 / 2.0).abs() < 1e-6, "got r={r}");
}

#[test]
fn mec_collinear_points_use_extreme_pair() {
    let pts = [c(-3.0, 0.0), c(0.0, 0.0), c(5.0, 0.0)];
    let (center, r) = smallest_enclosing_circle(&pts).unwrap();
    assert!(covers(center, r, &pts), "collinear MEC must enclose all");
    assert!((r - 4.0).abs() < 1e-6, "radius should span -3..5 → 4, got {r}");
}

#[test]
fn mec_empty_is_none() {
    assert!(smallest_enclosing_circle(&[]).is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms clustering::fastest::tests::mec 2>&1 | tail -20`
Expected: FAIL — `cannot find function smallest_enclosing_circle`.

- [ ] **Step 3: Implement the helpers**

Add above `fn cluster(...)` in `fastest.rs`:

```rust
const EPS: f64 = 1e-7;

#[inline]
fn dist2(a: Coord, b: Coord) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    dx * dx + dy * dy
}

#[inline]
fn in_disc(center: Coord, r: f64, p: Coord) -> bool {
    dist2(center, p) <= r * r + EPS
}

/// Circle through two points: center at their midpoint, radius half their distance.
fn circle_two(a: Coord, b: Coord) -> (Coord, f64) {
    let center = Coord {
        x: (a.x + b.x) / 2.0,
        y: (a.y + b.y) / 2.0,
    };
    (center, dist2(a, b).sqrt() / 2.0)
}

/// Circumcircle of three points; falls back to the diameter circle of the farthest
/// pair when the points are (near-)collinear.
fn circle_three(a: Coord, b: Coord, c: Coord) -> (Coord, f64) {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        // Collinear: the smallest enclosing circle is the diameter of the two
        // extreme points (the third lies between them).
        let (ab, bc, ca) = (dist2(a, b), dist2(b, c), dist2(c, a));
        return if ab >= bc && ab >= ca {
            circle_two(a, b)
        } else if bc >= ca {
            circle_two(b, c)
        } else {
            circle_two(c, a)
        };
    }
    let a2 = a.x * a.x + a.y * a.y;
    let b2 = b.x * b.x + b.y * b.y;
    let c2 = c.x * c.x + c.y * c.y;
    let center = Coord {
        x: (a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d,
        y: (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d,
    };
    (center, dist2(center, a).sqrt())
}

/// Smallest enclosing circle `(center, radius)` of `points` in the Euclidean plane.
/// Deterministic incremental Welzl (input order, no shuffle): O(n) expected, fine for
/// the small per-group point sets here. Returns `None` for empty input.
fn smallest_enclosing_circle(points: &[Coord]) -> Option<(Coord, f64)> {
    let n = points.len();
    if n == 0 {
        return None;
    }
    let mut center = points[0];
    let mut r = 0.0_f64;
    for i in 1..n {
        if in_disc(center, r, points[i]) {
            continue;
        }
        center = points[i];
        r = 0.0;
        for j in 0..i {
            if in_disc(center, r, points[j]) {
                continue;
            }
            let (cc, rr) = circle_two(points[i], points[j]);
            center = cc;
            r = rr;
            for k in 0..j {
                if in_disc(center, r, points[k]) {
                    continue;
                }
                let (cc, rr) = circle_three(points[i], points[j], points[k]);
                center = cc;
                r = rr;
            }
        }
    }
    Some((center, r))
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms clustering::fastest::tests::mec 2>&1 | tail -20`
Expected: PASS (5 mec_* tests).

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/clustering/fastest.rs
git commit -m "feat(fastest): deterministic Euclidean smallest-enclosing-circle"
```

---

## Task 2: `Placement` strategy — fit test + center placement

**Files:**
- Modify: `crates/algorithms/src/clustering/fastest.rs`
- Test: same file

- [ ] **Step 1: Write the failing tests**

```rust
// ── Placement: fit + place ────────────────────────────────────────────────

#[test]
fn placement_mec_fits_within_unit_disc() {
    // Two points distance 2 apart → MEC radius exactly 1 → fits at margin 0.
    let pts = [c(0.0, 0.0), c(2.0, 0.0)];
    assert!(Placement::Mec.fits(&pts, 0.0));
    // distance 2.001 apart → MEC radius > 1 → does not fit.
    let pts2 = [c(0.0, 0.0), c(2.001, 0.0)];
    assert!(!Placement::Mec.fits(&pts2, 0.0));
}

#[test]
fn placement_mec_margin_rejects_boundary() {
    // radius exactly 1 fails once a positive margin is required.
    let pts = [c(0.0, 0.0), c(2.0, 0.0)];
    assert!(!Placement::Mec.fits(&pts, 0.01));
}

#[test]
fn placement_bbox_uses_diagonal_test() {
    // bbox diagonal of unit square = sqrt(2) ≈ 1.414 ≤ 2 → fits.
    let sq = [c(0.0, 0.0), c(1.0, 1.0)];
    assert!(Placement::BboxCenter.fits(&sq, 0.0));
    assert!(Placement::Centroid.fits(&sq, 0.0));
    // diagonal just over 2 → does not fit.
    let wide = [c(0.0, 0.0), c(1.5, 1.5)]; // diag = 2.121
    assert!(!Placement::BboxCenter.fits(&wide, 0.0));
}

#[test]
fn placement_centers_are_sane() {
    let pts = [c(0.0, 0.0), c(2.0, 0.0), c(0.0, 2.0), c(2.0, 2.0)];
    // bbox center of the 2x2 square is its middle.
    let b = Placement::BboxCenter.place(&pts);
    assert!((b.x - 1.0).abs() < 1e-9 && (b.y - 1.0).abs() < 1e-9);
    // centroid of the 4 corners is also the middle.
    let g = Placement::Centroid.place(&pts);
    assert!((g.x - 1.0).abs() < 1e-9 && (g.y - 1.0).abs() < 1e-9);
    // MEC center of the square is the middle too.
    let m = Placement::Mec.place(&pts);
    assert!((m.x - 1.0).abs() < 1e-6 && (m.y - 1.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms clustering::fastest::tests::placement 2>&1 | tail -20`
Expected: FAIL — `cannot find type Placement`.

- [ ] **Step 3: Implement `Placement`**

Add above `fn cluster(...)`:

```rust
/// Disc-center placement strategy. Each variant pairs a center computation with the
/// matching "does this group fit one radius-1 disc?" test. Benchmarked against each
/// other; the winner is hard-coded in the final version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Placement {
    /// Smallest-enclosing-circle center; fit ⇔ MEC radius ≤ 1 − margin. Textbook UDC.
    Mec,
    /// Mean of the points; fit ⇔ union-bbox diagonal ≤ 2 − margin.
    Centroid,
    /// Union-bbox center; fit ⇔ union-bbox diagonal ≤ 2 − margin.
    BboxCenter,
}

/// `(min_x, min_y, max_x, max_y)` of a non-empty point set.
fn bounds(points: &[Coord]) -> (f64, f64, f64, f64) {
    let mut min_x = points[0].x;
    let mut min_y = points[0].y;
    let mut max_x = points[0].x;
    let mut max_y = points[0].y;
    for p in &points[1..] {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    (min_x, min_y, max_x, max_y)
}

impl Placement {
    /// Does `points` fit inside a single radius-1 disc (allowing `margin` of safety)?
    fn fits(self, points: &[Coord], margin: f64) -> bool {
        if points.is_empty() {
            return true;
        }
        // Cheap necessary pre-check shared by all variants: a radius-1 cover implies the
        // bounding-box diagonal is ≤ 2.
        let (min_x, min_y, max_x, max_y) = bounds(points);
        let diag2 = (max_x - min_x).powi(2) + (max_y - min_y).powi(2);
        match self {
            Placement::Mec => {
                if diag2 > 4.0 {
                    return false;
                }
                match smallest_enclosing_circle(points) {
                    Some((_, r)) => r <= 1.0 - margin,
                    None => true,
                }
            }
            Placement::Centroid | Placement::BboxCenter => {
                let limit = 2.0 - margin;
                diag2 <= limit * limit
            }
        }
    }

    /// Place the disc center for `points` (assumes non-empty).
    fn place(self, points: &[Coord]) -> Coord {
        match self {
            Placement::Mec => smallest_enclosing_circle(points).unwrap().0,
            Placement::Centroid => {
                let n = points.len() as f64;
                let (sx, sy) = points
                    .iter()
                    .fold((0.0, 0.0), |(sx, sy), p| (sx + p.x, sy + p.y));
                Coord { x: sx / n, y: sy / n }
            }
            Placement::BboxCenter => {
                let (min_x, min_y, max_x, max_y) = bounds(points);
                Coord {
                    x: (min_x + max_x) / 2.0,
                    y: (min_y + max_y) / 2.0,
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms clustering::fastest::tests::placement 2>&1 | tail -20`
Expected: PASS (4 placement_* tests).

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/clustering/fastest.rs
git commit -m "feat(fastest): swappable Placement strategy (MEC/centroid/bbox)"
```

---

## Task 3: Region-growing cover — rewrite `cluster` and wire `main`

**Files:**
- Modify: `crates/algorithms/src/clustering/fastest.rs` (replace `cluster`; update `main`; update imports)
- Test: same file

This replaces the old grid+pairwise-merge `cluster`. The old `BoundingBox`/`Cell` structs and the old `cluster` body are removed; `midpoint` is removed (unused). The dedup-by-bits and `main` filter are kept.

- [ ] **Step 1: Write the failing tests**

```rust
// ── cluster: region-growing behavior ──────────────────────────────────────

#[test]
fn cluster_mec_covers_every_point_when_min_points_1() {
    // A handful of nearby points; with min_points=1 nothing is dropped, so every
    // projected point must be within radius 1 of some returned center.
    let pts: Vec<Coord> = (0..25)
        .map(|i| c((i % 5) as f64 * 0.3, (i / 5) as f64 * 0.3))
        .collect();
    let centers = cluster(pts.clone(), 1, Placement::Mec, 0.0);
    for p in &pts {
        let covered = centers
            .iter()
            .any(|(ctr, _)| dist2(*ctr, *p) <= 1.0 + 1e-6);
        assert!(covered, "point {p:?} not covered by any disc");
    }
}

#[test]
fn cluster_is_deterministic_as_a_set() {
    let pts: Vec<Coord> = (0..40)
        .map(|i| c((i % 7) as f64 * 0.5, (i / 7) as f64 * 0.5))
        .collect();
    let mut a: Vec<(u64, u64)> = cluster(pts.clone(), 2, Placement::Mec, 0.0)
        .into_iter()
        .map(|(ctr, _)| (ctr.x.to_bits(), ctr.y.to_bits()))
        .collect();
    let mut b: Vec<(u64, u64)> = cluster(pts.clone(), 2, Placement::Mec, 0.0)
        .into_iter()
        .map(|(ctr, _)| (ctr.x.to_bits(), ctr.y.to_bits()))
        .collect();
    a.sort();
    b.sort();
    assert_eq!(a, b, "cluster output must be a deterministic set");
}

#[test]
fn cluster_aggregates_neighbouring_cells() {
    // Three points strung along x within a single radius-1 disc but in distinct √2
    // grid cells (cell width ≈ 1.414): x = 0.0, 1.5, 3.0 span 3 cells, MEC radius 1.5
    // → does NOT fit one disc; x = 0.0, 0.9, 1.8 (span ≈ 1.8, MEC r=0.9) DOES, across
    // 2 cells. Use the fitting case and assert they collapse to a single center.
    let pts = vec![c(0.0, 0.0), c(0.9, 0.0), c(1.8, 0.0)];
    let centers = cluster(pts, 1, Placement::Mec, 0.0);
    assert_eq!(centers.len(), 1, "three near-collinear points fitting one disc → 1 center");
    assert_eq!(centers[0].1, 3, "the single disc should count all 3 members");
}

#[test]
fn cluster_drops_sub_min_points_groups() {
    // Two far-apart singletons, min_points=2 → each group has 1 member → all dropped.
    let pts = vec![c(0.0, 0.0), c(100.0, 100.0)];
    let centers = cluster(pts, 2, Placement::Mec, 0.0);
    assert!(centers.is_empty(), "groups below min_points must be dropped");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms clustering::fastest::tests::cluster 2>&1 | tail -20`
Expected: FAIL — `cluster` signature mismatch (old one takes 2 args / different body).

- [ ] **Step 3: Rewrite `cluster`, `main`, imports**

Replace the imports block at the top of the file with:

```rust
use geo::Coord;
use koji_core::SingleVec;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::project::Plane;
```

Replace `main` with (note the temporary env-selected placement — removed in Task 6):

```rust
const MARGIN: f64 = 0.0;

/// Temporary benchmark knob (removed once the winning placement is hard-coded):
/// `KOJI_FASTEST_PLACEMENT=mec|centroid|bbox`, default `mec`.
fn placement_from_env() -> Placement {
    match std::env::var("KOJI_FASTEST_PLACEMENT").as_deref() {
        Ok("centroid") => Placement::Centroid,
        Ok("bbox") => Placement::BboxCenter,
        _ => Placement::Mec,
    }
}

pub fn main(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let projected = plane.project();

    let output: SingleVec = cluster(projected, min_points, placement_from_env(), MARGIN)
        .into_iter()
        .filter_map(|(center, count)| (count >= min_points).then_some([center.x, center.y]))
        .collect();

    plane.reverse(output)
}
```

Replace the old `BoundingBox` struct, `Cell` struct, `midpoint`, and the old `cluster` body with:

```rust
/// Integer coordinate of a cell in the scaled projection grid.
type CellKey = (i32, i32);

/// The eight grid neighbours of a cell.
fn neighbours((v, h): CellKey) -> [CellKey; 8] {
    [
        (v - 1, h - 1),
        (v - 1, h),
        (v - 1, h + 1),
        (v, h - 1),
        (v, h + 1),
        (v + 1, h - 1),
        (v + 1, h),
        (v + 1, h + 1),
    ]
}

/// Bucket points into a √2 grid, region-grow each occupied cell (in deterministic sorted
/// order) by absorbing neighbours while the group still fits one radius-1 disc, then emit
/// one `(center, member_count)` per group. Coincident centers are de-duplicated.
fn cluster(
    points: Vec<Coord>,
    min_points: usize,
    placement: Placement,
    margin: f64,
) -> Vec<(Coord, usize)> {
    let sqrt2 = std::f64::consts::SQRT_2;

    // First pass: bucket. BTreeMap gives deterministic sorted iteration over cells.
    let mut cells: BTreeMap<CellKey, Vec<Coord>> = BTreeMap::new();
    for p in points {
        let key = ((p.x / sqrt2).floor() as i32, (p.y / sqrt2).floor() as i32);
        cells.entry(key).or_default().push(p);
    }

    let mut centers: HashMap<(u64, u64), (Coord, usize)> = HashMap::new();
    let mut claimed: BTreeSet<CellKey> = BTreeSet::new();

    // Second pass: region-grow from each unclaimed cell in sorted order.
    for (&seed, seed_points) in &cells {
        if claimed.contains(&seed) {
            continue;
        }
        claimed.insert(seed);

        let mut group_cells: BTreeSet<CellKey> = BTreeSet::from([seed]);
        let mut group_points: Vec<Coord> = seed_points.clone();

        loop {
            // Sorted set of unclaimed, occupied neighbours of the current group.
            let mut candidates: BTreeSet<CellKey> = BTreeSet::new();
            for &cell in &group_cells {
                for n in neighbours(cell) {
                    if !claimed.contains(&n) && cells.contains_key(&n) {
                        candidates.insert(n);
                    }
                }
            }

            // Absorb the first (sorted) candidate that keeps the group disc-coverable.
            let mut absorbed = None;
            for cand in &candidates {
                let mut trial = group_points.clone();
                trial.extend_from_slice(&cells[cand]);
                if placement.fits(&trial, margin) {
                    absorbed = Some(*cand);
                    break;
                }
            }

            match absorbed {
                Some(cand) => {
                    group_points.extend_from_slice(&cells[&cand]);
                    group_cells.insert(cand);
                    claimed.insert(cand);
                }
                None => break,
            }
        }

        let count = group_points.len();
        if count == 0 {
            continue;
        }
        let center = placement.place(&group_points);
        centers.insert((center.x.to_bits(), center.y.to_bits()), (center, count));
    }

    centers.into_values().collect()
}
```

- [ ] **Step 4: Run the new + existing tests**

Run: `cargo test -p algorithms clustering::fastest 2>&1 | tail -30`
Expected: PASS — the 4 new `cluster_*` tests, the Task 1/2 tests, and the pre-existing `main`-level tests (`empty_input_returns_empty`, `single_point_min1_returns_one_cluster`, `clusters_do_not_exceed_input_points`, `sparse_points_filtered_by_min_points`, `tight_cluster_yields_one_center`, `output_lat_lon_in_valid_range`, `deterministic_same_input`).

If `clusters_do_not_exceed_input_points` or `tight_cluster_yields_one_center` fail, debug the region-grow (they assert `len ≤ input` and `tight cluster → ≥1 center`, both of which the new algorithm satisfies).

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/clustering/fastest.rs
git commit -m "feat(fastest): region-growing unit-disc cover replacing pairwise merge"
```

---

## Task 4: Strengthen the determinism test

**Files:**
- Modify: `crates/algorithms/src/clustering/fastest.rs` (the existing `deterministic_same_input` test)

- [ ] **Step 1: Replace the weak len-only determinism assertion**

Replace the existing `deterministic_same_input` test body with set-equality (the cover is now genuinely deterministic, not just stable in count):

```rust
#[test]
fn deterministic_same_input() {
    let pts = vec![[40.0, -74.0], [40.001, -74.001], [40.5, -73.0]];
    let mut a = main(&pts, 70.0, 1);
    let mut b = main(&pts, 70.0, 1);
    a.sort_by(|p, q| p.partial_cmp(q).unwrap());
    b.sort_by(|p, q| p.partial_cmp(q).unwrap());
    assert_eq!(a, b, "Fastest must be deterministic (identical set of centers)");
}
```

- [ ] **Step 2: Run the full module test suite + clippy in parallel**

```bash
cargo test -p algorithms clustering::fastest 2>&1 | tail -15
cargo clippy -p algorithms --lib 2>&1 | grep -E "warning|error|fastest" ; echo "clippy done"
```
Expected: all tests PASS; clippy prints only `clippy done` (no warnings touching `fastest.rs`).

- [ ] **Step 3: Commit**

```bash
git add crates/algorithms/src/clustering/fastest.rs
git commit -m "test(fastest): assert deterministic center set, not just count"
```

---

## Task 5: Benchmark the three placement variants + the runtime gate

**Files:** none modified — this task *runs* `clusterbench` and records results.

The decision rule (from the spec): compare `score_v2` and `wall_s` for today's baseline,
each variant, and `fast`. **Any variant whose `wall_s` ≥ `fast`'s is disqualified.** Among
the rest, pick the best `score_v2`.

- [ ] **Step 1: Capture the pre-redesign Fastest baseline**

The pre-redesign Fastest is at commit `93ad30d`. Capture its numbers without disturbing the
working tree, using a scratch worktree:

```bash
git worktree add -d /tmp/fastest-baseline 93ad30d
( cd /tmp/fastest-baseline && \
  for ds in uniform blobs urban; do for r in 70 150; do for mp in 1 3; do \
    cargo run --release -q -p algorithms --bin clusterbench -- \
      --mode fastest --dataset $ds --n 10000 --radius $r --min-points $mp --seed 42 ; \
  done; done; done ) | tee /tmp/bench-baseline.txt
git worktree remove --force /tmp/fastest-baseline
```

- [ ] **Step 2: Benchmark `fast` (the runtime ceiling) on the current tree**

```bash
for ds in uniform blobs urban; do for r in 70 150; do for mp in 1 3; do \
  cargo run --release -q -p algorithms --bin clusterbench -- \
    --mode fast --dataset $ds --n 10000 --radius $r --min-points $mp --seed 42 ; \
done; done; done | tee /tmp/bench-fast.txt
```

- [ ] **Step 3: Benchmark each placement variant**

```bash
for placement in mec centroid bbox; do \
  for ds in uniform blobs urban; do for r in 70 150; do for mp in 1 3; do \
    KOJI_FASTEST_PLACEMENT=$placement cargo run --release -q -p algorithms --bin clusterbench -- \
      --mode fastest --dataset $ds --n 10000 --radius $r --min-points $mp --seed 42 ; \
  done; done; done; \
done | tee /tmp/bench-variants.txt
```

- [ ] **Step 4: Tabulate and decide**

For each dataset/radius/min_points cell, read the `score` (score_v2) and `wall_s` columns
from the RESULT lines (column order documented at `clusterbench.rs:419-442`). Produce a
short table: variant × cell → (score_v2, wall_s), plus baseline and `fast`.

Decision (record the reasoning in a scratch note `/tmp/fastest-decision.md`):
1. Disqualify any variant whose median `wall_s` ≥ `fast`'s median `wall_s`.
2. Among survivors, pick the best (lowest) median `score_v2`.
3. Sanity-check it also beats the pre-redesign baseline's `score_v2`. If no variant beats
   baseline, fall back to structure **B** (fixed-up pairwise) — STOP and report; do not
   ship a regression.
4. If `knife_edge` is non-trivial for the winner, note a `MARGIN` candidate (e.g. 0.02) to
   re-test in Task 6.

- [ ] **Step 5: Record the decision (no code commit; this is analysis)**

Append the chosen variant + the comparison table to the bottom of the spec file
`docs/superpowers/specs/2026-06-16-fastest-unit-disc-cover-design.md` under a new
`## Benchmark results (2026-06-16)` section, and commit:

```bash
git add docs/superpowers/specs/2026-06-16-fastest-unit-disc-cover-design.md
git commit -m "docs(fastest): record placement benchmark results and chosen variant"
```

---

## Task 6: Finalize — hard-code the winner, delete the losers

**Files:**
- Modify: `crates/algorithms/src/clustering/fastest.rs`

Let `WINNER` be the variant chosen in Task 5. This task removes the experiment scaffolding.

- [ ] **Step 1: Hard-code the winner and drop the env knob**

Replace `placement_from_env()` and its call in `main` so `cluster` is invoked with the
winning variant directly, e.g. if the winner is `Mec`:

```rust
pub fn main(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let projected = plane.project();

    let output: SingleVec = cluster(projected, min_points, Placement::Mec, MARGIN)
        .into_iter()
        .filter_map(|(center, count)| (count >= min_points).then_some([center.x, center.y]))
        .collect();

    plane.reverse(output)
}
```

Delete `fn placement_from_env()`.

- [ ] **Step 2: Delete the losing `Placement` variants and now-dead helpers**

- Remove the unused variants from `enum Placement` and their match arms in `fits`/`place`.
  - If only one variant remains, collapse `Placement` entirely: inline the winner's `fits`
    and `place` as free functions `fn fits(points, margin) -> bool` / `fn place(points) -> Coord`,
    and change `cluster`'s signature to drop the `placement` parameter (update its callers
    in tests accordingly).
- If the winner is **not** `Mec`, delete `smallest_enclosing_circle`, `circle_two`,
  `circle_three`, `in_disc`, and the `mec_*` tests (and `dist2`/`EPS` if nothing else uses
  them — note the `cluster_mec_covers_*` test uses `dist2`, so update/remove that test too).
- If the winner **is** `Mec`, delete the `bounds`-based bbox/centroid arms only.

- [ ] **Step 3: Apply the `MARGIN` decision**

If Task 5 flagged knife_edge, set `const MARGIN` to the chosen value (e.g. `0.02`) and
re-run the variant benchmark cell that showed it to confirm knife_edge drops without a
score regression. Otherwise leave `MARGIN = 0.0`.

- [ ] **Step 4: Verify — tests + clippy + targeted re-benchmark**

```bash
cargo test -p algorithms clustering::fastest 2>&1 | tail -15
cargo clippy -p algorithms --lib 2>&1 | grep -E "warning|error|fastest" ; echo "clippy done"
cargo run --release -q -p algorithms --bin clusterbench -- \
  --mode fastest --dataset urban --n 10000 --radius 70 --min-points 3 --seed 42
```
Expected: all tests PASS; clippy clean; the final RESULT line's `score`/`wall_s` match the
winner's Task-5 numbers (confirms no regression from deleting scaffolding).

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/clustering/fastest.rs
git commit -m "feat(fastest): finalize winning placement, remove experiment scaffolding"
```

---

## Self-review notes (author)

- **Spec coverage:** pipeline (T3), geometry/radius-1 (T1–T3), pluggable placement (T2, T5), MEC deterministic Euclidean + non-reuse of `sec` (T1), min_points drop (T3; reassign deliberately deferred — flagged as optional in spec, not in scope unless a follow-up shows it's needed), determinism set-equality (T4), testing (T1–T4), benchmark + runtime gate (T5), finalize/delete-losers (T6). The optional reassign pass and grid-shift "make it good" extras from the spec are intentionally NOT tasks here — they are post-benchmark stretch ideas to attempt only if the winner has runtime headroom under `fast`; add them as a follow-up plan if Task 5 shows slack.
- **Placeholder scan:** none — every code step has full code; the only deferred value is `WINNER`/`MARGIN`, which are *outputs of Task 5* by design, with explicit conditional handling in Task 6.
- **Type consistency:** `cluster(Vec<Coord>, usize, Placement, f64) -> Vec<(Coord, usize)>`, `Placement::{fits(&[Coord], f64)->bool, place(&[Coord])->Coord}`, `smallest_enclosing_circle(&[Coord]) -> Option<(Coord, f64)>`, `CellKey=(i32,i32)`, `neighbours(CellKey)->[CellKey;8]` — used consistently across tasks.
```
