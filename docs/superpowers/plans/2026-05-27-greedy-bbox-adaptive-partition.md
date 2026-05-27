# Greedy Adaptive S2 Partition — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop `ClusterMode::Better` and `ClusterMode::Best` from generating tens to hundreds of millions of cluster candidates on huge bboxes by adding an adaptive S2-cell partitioner with halo, ownership filter, and stepwise mode-fallback ladder.

**Architecture:** A new `clustering/partition.rs` module wraps the existing `Greedy::setup` per-chunk. For Better/Best modes, `Greedy::run` calls `run_partitioned`, which adaptively subdivides the input bbox by S2 cell level (starting at level 6, descending to level 18 max) until each chunk's estimated candidate cost fits the budget. Each chunk solves greedy with a `radius`-wide halo overlap and keeps only clusters whose center falls in the owned cell. Cost estimator accounts for the existing grid-density scaling and falls back per chunk to Better → Balanced → Fast for irreducible cases.

**Tech Stack:** Rust 2024 edition, `s2 = "0.0.13"`, `rstar = "0.12.2"`, `rayon = "1.11.0"`, `hashbrown = "0.16.0"`, `sysinfo = "0.37.0"`, `rand = "0.9.2"`, `log = "0.4.28"`.

**Spec:** [docs/superpowers/specs/2026-05-27-greedy-bbox-adaptive-partition-design.md](../specs/2026-05-27-greedy-bbox-adaptive-partition-design.md)

---

## File Map

| Path | Action | Responsibility |
| --- | --- | --- |
| `server/algorithms/src/clustering/partition.rs` | Create | Types, config loader, estimator, partitioner, halo, ladder, orchestrator |
| `server/algorithms/src/clustering/mod.rs` | Modify | Add `mod partition;` |
| `server/algorithms/src/clustering/greedy.rs` | Modify | Branch `run` for Better/Best → `run_partitioned`; extract `associate_clusters_for_chunk`; deprecation warning on `set_cluster_split_level` |

All new tests live inline in `partition.rs` under `#[cfg(test)] mod tests`. The `algorithms` crate has no existing tests; this plan introduces the first ones.

**Conventions used below:**
- Run tests with `cargo test -p algorithms` from `server/` (workspace root for cargo).
- If rayon contention shows up in tests, add `--test-threads=1`.
- Commits use Conventional Commits (`feat:`, `refactor:`, `test:`, `chore:`).
- Each task is one bounded TDD cycle and ends with a commit.

**Pre-flight check — verify baseline compiles before starting:**

```bash
cd server && cargo check -p algorithms
```

If this fails on `rand` errors (`SmallRng` not in `rngs`, `rand::rng` private), the workspace baseline is broken and unrelated to this plan. Fix `server/algorithms/Cargo.toml` line 21:

```toml
# from
rand = { version = "0.9.2", default-features = false }
# to (minimum to unblock baseline)
rand = { version = "0.9.2", default-features = false, features = ["small_rng", "std", "std_rng", "thread_rng"] }
```

Or replace `rand::rng()` (sec/sec.rs:22) with the rand 0.9 equivalent if simpler. Resolve to a clean `cargo check` before proceeding. **This pre-flight is not part of this plan's commits** — fix on a separate branch or commit independently.

---

## Task 1: Bootstrap `partition.rs` module + types + config loader

**Files:**
- Create: `server/algorithms/src/clustering/partition.rs`
- Modify: `server/algorithms/src/clustering/mod.rs` (add `mod partition;` and re-export)

- [ ] **Step 1: Create `partition.rs` with bare types**

Create file `server/algorithms/src/clustering/partition.rs`:

```rust
use std::collections::HashMap;
use std::sync::OnceLock;

use model::api::cluster_mode::ClusterMode;
use model::api::single_vec::SingleVec;
use s2::cellid::CellID;
use sysinfo::System;

/// Bytes assumed per candidate when converting memory budget → candidate count.
/// PointArray (16 bytes) + Cluster<Point> overhead (avg Vec<&Point> tail).
pub(crate) const BYTES_PER_CANDIDATE: usize = 256;

pub(crate) const DEFAULT_START_LEVEL: u64 = 6;
pub(crate) const DEFAULT_MAX_LEVEL: u64 = 18;

#[derive(Debug, Clone)]
pub(crate) struct Chunk {
    pub cell: CellID,
    pub owned: SingleVec,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PartitionConfig {
    pub budget: usize,
    pub start_level: u64,
    pub max_level: u64,
}

#[derive(Debug, Default)]
pub(crate) struct PartitionStats {
    pub total_chunks: usize,
    pub downgrades: HashMap<(ClusterMode, ClusterMode), usize>,
}

static CONFIG: OnceLock<PartitionConfig> = OnceLock::new();

impl PartitionConfig {
    pub(crate) fn load() -> PartitionConfig {
        *CONFIG.get_or_init(|| {
            let sys = System::new_all();
            let threads = rayon::current_num_threads().max(1) as u64;
            let mem_per_thread = sys.available_memory() / threads;
            let auto_budget = (mem_per_thread / BYTES_PER_CANDIDATE as u64) as usize;

            let budget = std::env::var("KOJI_MAX_CANDIDATES_PER_CHUNK")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(auto_budget);
            let start_level = std::env::var("KOJI_PARTITION_START_LEVEL")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(DEFAULT_START_LEVEL);
            let max_level = std::env::var("KOJI_PARTITION_MAX_LEVEL")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(DEFAULT_MAX_LEVEL);

            log::info!(
                "PartitionConfig: budget={}, start_level={}, max_level={}",
                budget, start_level, max_level
            );
            PartitionConfig { budget, start_level, max_level }
        })
    }
}
```

- [ ] **Step 2: Add module to clustering/mod.rs**

Edit `server/algorithms/src/clustering/mod.rs` at line 16-20 (the `mod` declarations).

Replace:
```rust
mod candidates;
mod fastest;
// mod genetic;
mod greedy;
mod s2;
```

With:
```rust
mod candidates;
mod fastest;
// mod genetic;
mod greedy;
mod partition;
mod s2;
```

- [ ] **Step 3: Run cargo check to verify compile**

Run: `cargo check -p algorithms`
Expected: PASS with no errors. Warnings about unused types are OK at this stage.

- [ ] **Step 4: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs server/algorithms/src/clustering/mod.rs
git commit -m "feat(clustering): scaffold partition module + config types"
```

---

## Task 2: Test helpers (random_points_in_bbox, dense_cluster, sparse_grid)

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs` (append `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write failing test that uses the helpers**

Append to `server/algorithms/src/clustering/partition.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use model::api::Precision;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    use rand::Rng;

    pub(super) fn random_points_in_bbox(n: usize, bbox: [Precision; 4], seed: u64) -> SingleVec {
        let [min_lat, min_lon, max_lat, max_lon] = bbox;
        let mut rng = SmallRng::seed_from_u64(seed);
        (0..n)
            .map(|_| {
                let lat = rng.random_range(min_lat..max_lat);
                let lon = rng.random_range(min_lon..max_lon);
                [lat, lon]
            })
            .collect()
    }

    pub(super) fn dense_cluster(
        center: [Precision; 2],
        n: usize,
        radius_m: Precision,
        seed: u64,
    ) -> SingleVec {
        let mut rng = SmallRng::seed_from_u64(seed);
        // Approx 1 deg lat ≈ 111_000 m; ignore lon shrinkage for tiny test radii
        let radius_deg = radius_m / 111_000.0;
        (0..n)
            .map(|_| {
                let dlat = rng.random_range(-radius_deg..radius_deg);
                let dlon = rng.random_range(-radius_deg..radius_deg);
                [center[0] + dlat, center[1] + dlon]
            })
            .collect()
    }

    pub(super) fn sparse_grid(
        rows: usize,
        cols: usize,
        bbox: [Precision; 4],
    ) -> SingleVec {
        let [min_lat, min_lon, max_lat, max_lon] = bbox;
        let lat_step = (max_lat - min_lat) / rows as Precision;
        let lon_step = (max_lon - min_lon) / cols as Precision;
        (0..rows)
            .flat_map(|r| {
                (0..cols).map(move |c| {
                    [
                        min_lat + (r as Precision + 0.5) * lat_step,
                        min_lon + (c as Precision + 0.5) * lon_step,
                    ]
                })
            })
            .collect()
    }

    #[test]
    fn helpers_produce_expected_shapes() {
        let pts = random_points_in_bbox(100, [0., 0., 1., 1.], 42);
        assert_eq!(pts.len(), 100);
        for p in &pts {
            assert!(p[0] >= 0.0 && p[0] < 1.0);
            assert!(p[1] >= 0.0 && p[1] < 1.0);
        }

        let cluster = dense_cluster([10., 20.], 50, 100.0, 7);
        assert_eq!(cluster.len(), 50);

        let grid = sparse_grid(5, 5, [0., 0., 10., 10.]);
        assert_eq!(grid.len(), 25);
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test -p algorithms --lib clustering::partition::tests::helpers_produce_expected_shapes`
Expected: PASS.

If `Precision` is not importable from `model::api`, check `/Users/rin/GitHub/Koji/.claude/worktrees/condescending-blackburn-c2cc26/server/model/src/api.rs` for the actual path — it's likely `model::api::Precision` per usage in `candidates.rs:4`.

- [ ] **Step 3: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "test(partition): add seeded random/cluster/grid helpers"
```

---

## Task 3: Cost estimator (distinct_l16_cells, scaled_grid_density, estimate_cost)

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs`

- [ ] **Step 1: Write failing tests**

Inside the `#[cfg(test)] mod tests` block, add:

```rust
#[test]
fn distinct_l16_cells_dedupes() {
    let points = vec![[0.0, 0.0], [0.0, 0.0], [0.0, 0.0]];
    assert_eq!(distinct_l16_cells(&points), 1);

    let points = dense_cluster([0., 0.], 10, 1.0, 1);  // 10 points, ~1m radius
    let n = distinct_l16_cells(&points);
    assert!(n >= 1 && n <= 10, "expected dedup count between 1 and 10, got {}", n);
}

#[test]
fn scaled_grid_density_caps_correctly() {
    // s2_cost dominates -> density 0
    assert_eq!(scaled_grid_density(1_000_000, 500_000), 0);
    // tiny s2 cost, huge budget -> capped at BYTE * 6
    let big = scaled_grid_density(0, 100_000_000_000);
    assert_eq!(big, BYTE_TIMES_SIX);
    // moderate: sqrt(remaining) used
    let mid = scaled_grid_density(0, 1_000_000);
    assert_eq!(mid, 1000);
}

#[test]
fn estimate_cost_better_excludes_grid() {
    let points = dense_cluster([0., 0.], 100, 50.0, 2);
    let est_better = estimate_cost(&points, ClusterMode::Better, 10_000_000);
    let est_best = estimate_cost(&points, ClusterMode::Best, 10_000_000);
    assert!(est_best >= est_better, "Best must be >= Better");
}

#[test]
fn estimate_cost_best_scales_with_budget() {
    let points = vec![[0.0, 0.0]];
    let est_small = estimate_cost(&points, ClusterMode::Best, 100);
    let est_large = estimate_cost(&points, ClusterMode::Best, 100_000_000);
    assert!(est_small < est_large, "larger budget should allow larger grid");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms --lib clustering::partition::tests::distinct_l16_cells_dedupes`
Expected: FAIL (function not found).

- [ ] **Step 3: Implement the estimator**

Add to `server/algorithms/src/clustering/partition.rs` (above the test module):

```rust
use s2::latlng::LatLng;
use std::collections::HashSet as StdHashSet;
use model::api::Precision;
use model::api::point_array::PointArray;

/// Existing `BYTE` constant is local to greedy.rs (`const BYTE: usize = 1024`).
/// We mirror it here rather than expose it: this is the BYTE × 6 grid-density ceiling
/// used by Best mode in greedy::associate_clusters.
pub(crate) const BYTE_TIMES_SIX: usize = 1024 * 6;

pub(crate) fn distinct_l16_cells(points: &[PointArray]) -> usize {
    points
        .iter()
        .map(|p| CellID::from(LatLng::from_degrees(p[0], p[1])).parent(16))
        .collect::<StdHashSet<_>>()
        .len()
}

pub(crate) fn scaled_grid_density(s2_cost: usize, budget: usize) -> usize {
    let remaining = budget.saturating_sub(s2_cost);
    let density = (remaining as f64).sqrt() as usize;
    density.min(BYTE_TIMES_SIX)
}

pub(crate) fn estimate_cost(
    points: &[PointArray],
    mode: ClusterMode,
    budget: usize,
) -> usize {
    let s2_cost = distinct_l16_cells(points).saturating_mul(4096); // 4^(22-16)
    let grid_cost = if matches!(mode, ClusterMode::Best) {
        scaled_grid_density(s2_cost, budget).pow(2)
    } else {
        0
    };
    s2_cost.saturating_add(grid_cost)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms --lib clustering::partition::tests`
Expected: PASS for all four new tests + helpers test.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "feat(partition): cost estimator with grid-density scaling"
```

---

## Task 4: Cell helpers (cell_bbox_lat_lon, contains_latlng)

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs`

- [ ] **Step 1: Write failing tests**

Append to the test module:

```rust
#[test]
fn contains_latlng_owns_only_its_cell() {
    // Pick a stable lat/lon, get its level-10 parent.
    let center = LatLng::from_degrees(37.7749, -122.4194);  // SF
    let owning_cell = CellID::from(center).parent(10);

    // The lat/lon itself must be contained.
    assert!(contains_latlng(owning_cell, [37.7749, -122.4194]));

    // A point on the opposite side of the world is not contained.
    assert!(!contains_latlng(owning_cell, [-37.7749, 57.5806]));
}

#[test]
fn cell_bbox_lat_lon_is_finite_and_ordered() {
    let center = LatLng::from_degrees(0., 0.);
    let cell = CellID::from(center).parent(8);
    let bb = cell_bbox_lat_lon(cell);
    assert!(bb.min_lat.is_finite() && bb.max_lat.is_finite());
    assert!(bb.min_lon.is_finite() && bb.max_lon.is_finite());
    assert!(bb.min_lat <= bb.max_lat);
    assert!(bb.min_lon <= bb.max_lon);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms --lib clustering::partition::tests::contains_latlng_owns_only_its_cell`
Expected: FAIL (function not found).

- [ ] **Step 3: Implement helpers**

Add to `partition.rs` (above the test module):

```rust
use s2::cell::Cell;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LatLonBBox {
    pub min_lat: Precision,
    pub max_lat: Precision,
    pub min_lon: Precision,
    pub max_lon: Precision,
}

impl LatLonBBox {
    pub fn center_lat(&self) -> Precision {
        0.5 * (self.min_lat + self.max_lat)
    }

    pub fn expand(&self, radius_deg: Precision) -> LatLonBBox {
        LatLonBBox {
            min_lat: self.min_lat - radius_deg,
            max_lat: self.max_lat + radius_deg,
            min_lon: self.min_lon - radius_deg,
            max_lon: self.max_lon + radius_deg,
        }
    }
}

pub(crate) fn cell_bbox_lat_lon(cell: CellID) -> LatLonBBox {
    let c = Cell::from(&cell);
    // Use vertex extremes; avoids rect_bound() API churn between s2 versions.
    let mut min_lat = Precision::INFINITY;
    let mut max_lat = Precision::NEG_INFINITY;
    let mut min_lon = Precision::INFINITY;
    let mut max_lon = Precision::NEG_INFINITY;
    for i in 0..4 {
        let v = c.vertex(i);
        let ll = LatLng::from(&v);
        let lat = ll.lat.deg();
        let lon = ll.lng.deg();
        if lat < min_lat { min_lat = lat; }
        if lat > max_lat { max_lat = lat; }
        if lon < min_lon { min_lon = lon; }
        if lon > max_lon { max_lon = lon; }
    }
    LatLonBBox { min_lat, max_lat, min_lon, max_lon }
}

pub(crate) fn contains_latlng(cell: CellID, p: PointArray) -> bool {
    let derived = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(cell.level());
    derived == cell
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms --lib clustering::partition::tests`
Expected: PASS for all previous tests + 2 new ones.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "feat(partition): cell bbox + ownership helpers"
```

---

## Task 5: `adaptive_partition` function

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs`

- [ ] **Step 1: Write failing tests**

Append to the test module:

```rust
#[test]
fn partition_small_bbox_single_chunk() {
    // 1000 points in roughly 1km² → should fit in one chunk at start_level=6.
    let pts = random_points_in_bbox(1000, [37.78, -122.43, 37.79, -122.42], 42);
    let chunks = adaptive_partition(&pts, usize::MAX, ClusterMode::Better, 6, 18);
    assert_eq!(chunks.len(), 1, "small bbox should be one chunk, got {}", chunks.len());
}

#[test]
fn partition_subdivides_when_over_budget() {
    // Force subdivision with a tiny budget.
    let pts = sparse_grid(20, 20, [-30., -60., 30., 60.]);
    let chunks = adaptive_partition(&pts, 1, ClusterMode::Better, 6, 18);
    assert!(chunks.len() > 1, "tiny budget should produce many chunks");
}

#[test]
fn partition_halts_at_max_level() {
    // Dense cluster + extremely tight budget → at least one chunk at max_level=18.
    let pts = dense_cluster([0., 0.], 5_000, 50.0, 11);
    let chunks = adaptive_partition(&pts, 1, ClusterMode::Best, 6, 18);
    assert!(
        chunks.iter().any(|c| c.cell.level() == 18),
        "expected at least one chunk to reach max_level"
    );
}

#[test]
fn partition_preserves_all_points() {
    let pts = random_points_in_bbox(500, [0., 0., 5., 5.], 99);
    let chunks = adaptive_partition(&pts, 1_000_000, ClusterMode::Better, 6, 18);
    let total: usize = chunks.iter().map(|c| c.owned.len()).sum();
    assert_eq!(total, pts.len(), "no points should be dropped during partition");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms --lib clustering::partition::tests::partition_small_bbox_single_chunk`
Expected: FAIL (function not found).

- [ ] **Step 3: Implement `adaptive_partition`**

Add to `partition.rs` (above the test module):

```rust
use crate::s2::create_cell_map;

pub(crate) fn adaptive_partition(
    points: &SingleVec,
    budget: usize,
    mode: ClusterMode,
    start_level: u64,
    max_level: u64,
) -> Vec<Chunk> {
    if points.is_empty() {
        return vec![];
    }
    let mut frontier: HashMap<u64, SingleVec> = create_cell_map(points, start_level);
    let mut accepted: Vec<Chunk> = Vec::new();

    loop {
        let mut next_frontier: HashMap<u64, SingleVec> = HashMap::new();
        for (cell_id_raw, cell_points) in frontier.into_iter() {
            let cell = CellID(cell_id_raw);
            let est = estimate_cost(&cell_points, mode, budget);
            if est <= budget || cell.level() >= max_level {
                accepted.push(Chunk { cell, owned: cell_points });
            } else {
                let children = create_cell_map(&cell_points, cell.level() + 1);
                for (k, v) in children {
                    next_frontier.entry(k).or_insert_with(Vec::new).extend(v);
                }
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }
    accepted
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms --lib clustering::partition::tests`
Expected: All passing including 4 new partition tests.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "feat(partition): adaptive_partition with BFS S2-level descent"
```

---

## Task 6: `gather_halo` function

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs`

- [ ] **Step 1: Write failing test**

Append to the test module:

```rust
#[test]
fn gather_halo_includes_nearby_points_outside_cell() {
    use crate::clustering::rtree::point::Point;
    use crate::rtree;

    // Pick a level-12 cell; place one point inside, one just outside (within radius).
    let inside = [37.7749, -122.4194];
    let cell = CellID::from(LatLng::from_degrees(inside[0], inside[1])).parent(12);
    let bbox = cell_bbox_lat_lon(cell);

    // Point just past the east edge (10 meters past, well under default 70m radius).
    let outside = [bbox.max_lat - 0.00001, bbox.max_lon + 0.00005];

    let all_points: SingleVec = vec![inside, outside];
    let tree = rtree::spawn(70.0, &all_points);

    let halo = gather_halo(cell, &tree, 70.0);
    assert!(
        halo.iter().any(|p| (p.center[1] - outside[1]).abs() < 1e-6),
        "halo should include the just-outside point"
    );
    assert!(
        !halo.iter().any(|p| (p.center[1] - inside[1]).abs() < 1e-6),
        "halo should NOT include the inside point"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algorithms --lib clustering::partition::tests::gather_halo_includes_nearby_points_outside_cell`
Expected: FAIL (function not found).

- [ ] **Step 3: Implement `gather_halo`**

Add to `partition.rs`:

```rust
use crate::clustering::rtree::point::Point;
use crate::clustering::candidates;
use rstar::{RTree, AABB};

pub(crate) fn gather_halo(
    cell: CellID,
    all_points_tree: &RTree<Point>,
    radius_meters: Precision,
) -> Vec<Point> {
    let bbox = cell_bbox_lat_lon(cell);
    let radius_deg = candidates_meters_to_degrees(radius_meters, bbox.center_lat());
    let expanded = bbox.expand(radius_deg);

    let envelope = AABB::from_corners(
        [expanded.min_lat, expanded.min_lon],
        [expanded.max_lat, expanded.max_lon],
    );

    all_points_tree
        .locate_in_envelope_intersecting(&envelope)
        .filter(|p| !contains_latlng(cell, p.center))
        .cloned()
        .collect()
}

/// Thin re-export so we don't depend on `pub` of candidates::meters_to_degrees.
/// candidates::meters_to_degrees is already `pub fn` (candidates.rs:127), use it directly.
fn candidates_meters_to_degrees(meters: Precision, lat: Precision) -> Precision {
    candidates::meters_to_degrees(meters, lat)
}
```

Note: `Point` is `#[derive(Debug, Clone, Copy)]` (see `server/algorithms/src/rtree/point.rs:15`), so `.cloned()` works. No special handling needed.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p algorithms --lib clustering::partition::tests::gather_halo_includes_nearby_points_outside_cell`
Expected: PASS.

If rtree's `locate_in_envelope_intersecting` API differs in version 0.12, consult `rstar = "0.12.2"` docs — equivalent method may be `locate_in_envelope_intersecting_mut` (no-op for our read) or `iter_in_envelope`. Pick the closest match and adjust.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "feat(partition): gather_halo collects boundary points via rtree envelope"
```

---

## Task 7: `select_effective_mode` (fallback ladder)

**Files:**
- Modify: `server/algorithms/src/clustering/partition.rs`

- [ ] **Step 1: Write failing tests**

Append to the test module:

```rust
#[test]
fn fallback_ladder_keeps_mode_when_under_budget() {
    let pts = random_points_in_bbox(10, [0., 0., 0.001, 0.001], 1);
    let mode = select_effective_mode(ClusterMode::Best, &pts, usize::MAX);
    assert_eq!(mode, ClusterMode::Best);
}

#[test]
fn fallback_ladder_walks_down_to_fast() {
    // Force a chunk that is over budget even at Fast (impossible in practice,
    // but we set budget = 0 to verify ladder reaches floor).
    let pts = random_points_in_bbox(50, [-30., -60., 30., 60.], 7);
    let mode = select_effective_mode(ClusterMode::Best, &pts, 0);
    assert_eq!(mode, ClusterMode::Fast, "ladder must terminate at Fast");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p algorithms --lib clustering::partition::tests::fallback_ladder_keeps_mode_when_under_budget`
Expected: FAIL (function not found).

- [ ] **Step 3: Implement `select_effective_mode`**

Add to `partition.rs`:

```rust
pub(crate) fn select_effective_mode(
    requested: ClusterMode,
    points: &[PointArray],
    budget: usize,
) -> ClusterMode {
    let mut current = requested;
    loop {
        if matches!(current, ClusterMode::Fast)
            || estimate_cost(points, current, budget) <= budget
        {
            return current;
        }
        let next = match current {
            ClusterMode::Best => ClusterMode::Better,
            ClusterMode::Better => ClusterMode::Balanced,
            ClusterMode::Balanced => ClusterMode::Fast,
            _ => return current,
        };
        log::warn!(
            "chunk over budget for {:?} (est={}, budget={}), downgrading to {:?}",
            current,
            estimate_cost(points, current, budget),
            budget,
            next,
        );
        current = next;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p algorithms --lib clustering::partition::tests`
Expected: All passing.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "feat(partition): select_effective_mode stepwise fallback ladder"
```

---

## Task 8: Refactor `associate_clusters` → expose mode-parameterized variant

**Files:**
- Modify: `server/algorithms/src/clustering/greedy.rs:157-242`

- [ ] **Step 1: Read current implementation**

Read `server/algorithms/src/clustering/greedy.rs:157-242`. The current `associate_clusters` does mode-based candidate generation (lines 168-181) then filtering, sizing, and bucketing. The bucketing tail is mode-agnostic.

- [ ] **Step 2: Extract candidate generation into a helper that accepts mode + grid_density**

Replace the body of `associate_clusters` (greedy.rs:157-242). Change the signature **of the candidate-generation step** to accept `mode` and `grid_density`, leaving the rest of the body intact.

New helper at module scope (still inside `impl<'a> Greedy`):

```rust
fn generate_candidates_for_mode(
    &self,
    points: &'a SingleVec,
    point_tree: &'a RTree<Point>,
    mode: ClusterMode,
    grid_density_override: Option<usize>,
) -> SingleVec {
    const BYTE: usize = 1024;
    match mode {
        ClusterMode::Honeycomb => self.get_honeycomb_clusters(points),
        ClusterMode::Fast => self.gen_clusters(BYTE / 2, points),
        ClusterMode::Balanced => self.gen_clusters(BYTE, points),
        ClusterMode::Better | ClusterMode::Best => {
            let mut pcs = self.get_s2_clusters(points, point_tree);
            if matches!(mode, ClusterMode::Best) {
                let density = grid_density_override.unwrap_or(BYTE * 6);
                if density > 0 {
                    pcs.extend_from_slice(&self.gen_clusters(density, points));
                }
            }
            pcs
        }
        _ => vec![],
    }
}
```

Then update `associate_clusters` to use it:

Replace lines 168-181 (the `let clusters_with_data: Vec<Cluster> = match self.cluster_mode { ... } .into_par_iter() ...`) with:

```rust
let clusters_with_data: Vec<Cluster> = self
    .generate_candidates_for_mode(points, point_tree, self.cluster_mode, None)
    .into_par_iter()
    .filter_map(|cluster| {
        let iter = point_tree.locate_all_at_point(&cluster);
        let mut points = Vec::with_capacity(iter.size_hint().0);
        points.extend(iter);

        (points.len() >= self.min_points)
            .then(|| Cluster::new(Point::new(self.radius, 20, cluster), points, vec![]))
    })
    .collect();
```

The rest of `associate_clusters` (memory log, bucketing) is unchanged.

- [ ] **Step 3: Add an `associate_clusters_for_chunk` method**

Add inside `impl<'a> Greedy`, before `setup`:

```rust
fn associate_clusters_for_chunk(
    &'a self,
    points: &'a SingleVec,
    point_tree: &'a RTree<Point>,
    effective_mode: ClusterMode,
    budget: usize,
) -> Vec<Vec<Cluster<'a>>> {
    use crate::clustering::partition::{distinct_l16_cells, scaled_grid_density};

    let grid_density = if matches!(effective_mode, ClusterMode::Best) {
        let s2_cost = distinct_l16_cells(points).saturating_mul(4096);
        Some(scaled_grid_density(s2_cost, budget))
    } else {
        None
    };

    let raw_candidates = self.generate_candidates_for_mode(
        points,
        point_tree,
        effective_mode,
        grid_density,
    );

    let clusters_with_data: Vec<Cluster> = raw_candidates
        .into_par_iter()
        .filter_map(|cluster| {
            let iter = point_tree.locate_all_at_point(&cluster);
            let mut points = Vec::with_capacity(iter.size_hint().0);
            points.extend(iter);

            (points.len() >= self.min_points)
                .then(|| Cluster::new(Point::new(self.radius, 20, cluster), points, vec![]))
        })
        .collect();

    // Reuse the bucketing tail from associate_clusters. The cleanest way is to extract
    // it into a method `bucket_clusters_by_size`; for now inline:
    let max = clusters_with_data
        .iter()
        .map(|cluster| cluster.all.len())
        .max()
        .unwrap_or(100);
    let mut clustered_clusters = vec![vec![]; max + 1];
    for cluster in clusters_with_data.into_iter() {
        clustered_clusters[cluster.all.len()].push(cluster);
    }
    clustered_clusters
}
```

- [ ] **Step 4: Run cargo check to verify compile**

Run: `cargo check -p algorithms`
Expected: PASS.

- [ ] **Step 5: Verify existing tests + the partition tests still pass**

Run: `cargo test -p algorithms`
Expected: All passing.

- [ ] **Step 6: Commit**

```bash
git add server/algorithms/src/clustering/greedy.rs
git commit -m "refactor(greedy): extract mode-parameterized candidate generation"
```

---

## Task 9: `solve_chunk` (per-chunk orchestrator on Greedy)

**Files:**
- Modify: `server/algorithms/src/clustering/greedy.rs` (add method on `impl<'a> Greedy`)
- Modify: `server/algorithms/src/clustering/partition.rs` (only if helpers need exposure)

- [ ] **Step 1: Write failing test in partition.rs**

Append to the test module in `partition.rs`:

```rust
#[test]
fn ownership_filter_drops_foreign_centers() {
    use crate::clustering::greedy::Greedy;

    // Two non-adjacent dense clusters; partition should produce >=2 chunks.
    // Each chunk's solve_chunk result must contain ONLY centers parenting to its cell.
    let mut pts = dense_cluster([10.0, 20.0], 100, 50.0, 1);
    pts.extend(dense_cluster([40.0, 80.0], 100, 50.0, 2));

    let mut greedy = Greedy::default();
    greedy.set_cluster_mode(ClusterMode::Better).set_radius(70.0);

    let chunks = adaptive_partition(&pts, usize::MAX, ClusterMode::Better, 6, 18);
    assert!(chunks.len() >= 2, "two distant clusters should produce >=2 chunks");

    let tree = crate::rtree::spawn(70.0, &pts);
    for chunk in &chunks {
        let (solution, _downgrade) = greedy.solve_chunk(chunk, &tree, usize::MAX);
        for p in &solution {
            assert!(
                contains_latlng(chunk.cell, p.center),
                "cluster center {:?} must be inside owned cell {:?}",
                p.center, chunk.cell
            );
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p algorithms --lib clustering::partition::tests::ownership_filter_drops_foreign_centers`
Expected: FAIL (method `solve_chunk` not found).

- [ ] **Step 3: Implement `solve_chunk` on Greedy**

Add inside `impl<'a> Greedy` in `greedy.rs`, after `associate_clusters_for_chunk`:

```rust
pub(crate) fn solve_chunk(
    &'a self,
    chunk: &crate::clustering::partition::Chunk,
    all_points_tree: &RTree<Point>,
    budget: usize,
) -> (HashSet<Point>, Option<(ClusterMode, ClusterMode)>) {
    use crate::clustering::partition::{gather_halo, select_effective_mode, contains_latlng};

    let halo = gather_halo(chunk.cell, all_points_tree, self.radius);
    let combined: SingleVec = chunk
        .owned
        .iter()
        .cloned()
        .chain(halo.iter().map(|p| p.center))
        .collect();

    let effective_mode = select_effective_mode(self.cluster_mode, &combined, budget);
    let downgrade = (effective_mode != self.cluster_mode)
        .then(|| (self.cluster_mode, effective_mode));

    let point_tree: RTree<Point> = crate::rtree::spawn(self.radius, &combined);
    let clusters_with_data =
        self.associate_clusters_for_chunk(&combined, &point_tree, effective_mode, budget);

    let mut solution: Vec<Cluster> = self.cluster(&clusters_with_data).into_iter().collect();
    self.update_unique(&mut solution);

    solution.retain(|cluster| contains_latlng(chunk.cell, cluster.point.center));
    let result: HashSet<Point> = solution.into_iter().map(|c| c.into()).collect();
    (result, downgrade)
}
```

The visibility must be `pub(crate)` so the partition module's tests can reach it.

- [ ] **Step 4: Run all algorithm tests to verify pass**

Run: `cargo test -p algorithms`
Expected: All passing including the new ownership test.

- [ ] **Step 5: Commit**

```bash
git add server/algorithms/src/clustering/greedy.rs server/algorithms/src/clustering/partition.rs
git commit -m "feat(greedy): solve_chunk with halo + ownership filter + downgrade reporting"
```

---

## Task 10: `run_partitioned` + wire `Greedy::run` + setter deprecation

**Files:**
- Modify: `server/algorithms/src/clustering/greedy.rs`

- [ ] **Step 1: Write smoke tests for Greedy::run on Better/Best with huge bbox**

Append to the test module in `partition.rs`:

```rust
#[test]
fn greedy_better_completes_huge_random_bbox() {
    use crate::clustering::greedy::Greedy;
    let pts = random_points_in_bbox(10_000, [-10., -10., 10., 10.], 42);
    let mut greedy = Greedy::default();
    greedy.set_cluster_mode(ClusterMode::Better).set_radius(70.0);
    let result = greedy.run(&pts);
    assert!(!result.is_empty(), "Better mode should produce some clusters");
}

#[test]
fn greedy_best_completes_huge_random_bbox() {
    use crate::clustering::greedy::Greedy;
    let pts = random_points_in_bbox(5_000, [-5., -5., 5., 5.], 42);
    let mut greedy = Greedy::default();
    greedy.set_cluster_mode(ClusterMode::Best).set_radius(70.0);
    let result = greedy.run(&pts);
    assert!(!result.is_empty(), "Best mode should produce some clusters");
}
```

- [ ] **Step 2: Run tests to verify they fail (run_partitioned not yet wired)**

Run: `cargo test -p algorithms --lib clustering::partition::tests::greedy_better_completes_huge_random_bbox`
Expected: FAIL — `Greedy::run` for Better still calls the old single-shot `setup`. Tests likely time out or panic on these input sizes without the new partitioner. If they happen to pass with a small heap, that's OK — they will still serve as regression guards once `run_partitioned` is in place.

- [ ] **Step 3: Implement `run_partitioned` and wire `run`**

In `greedy.rs`, add inside `impl<'a> Greedy`:

```rust
fn run_partitioned(&'a self, points: &SingleVec) -> HashSet<Point> {
    use crate::clustering::partition::{adaptive_partition, PartitionConfig, PartitionStats};
    use rayon::prelude::*;

    let config = PartitionConfig::load();
    let all_points_tree: RTree<Point> = crate::rtree::spawn(self.radius, points);
    let chunks = adaptive_partition(
        points,
        config.budget,
        self.cluster_mode,
        config.start_level,
        config.max_level,
    );

    let mut stats = PartitionStats::default();
    stats.total_chunks = chunks.len();

    let chunk_results: Vec<(HashSet<Point>, Option<(ClusterMode, ClusterMode)>)> = chunks
        .par_iter()
        .map(|chunk| self.solve_chunk(chunk, &all_points_tree, config.budget))
        .collect();

    let mut solution: HashSet<Point> = HashSet::new();
    for (chunk_solution, downgrade) in chunk_results {
        solution.extend(chunk_solution);
        if let Some(pair) = downgrade {
            *stats.downgrades.entry(pair).or_insert(0) += 1;
        }
    }

    log::info!(
        "partition: {} chunks, downgrades: {:?}",
        stats.total_chunks,
        stats.downgrades,
    );

    if self.min_points == 1 {
        // Reuse check_missing's logic. The current check_missing takes Vec<Cluster>;
        // convert solution to a Vec of Cluster wrappers to call it, OR copy the
        // missing-point recovery logic inline. We inline since the conversion is awkward.
        let seen_cell_ids: HashSet<CellID> = solution
            .iter()
            .map(|p| p.cell_id)
            .collect();
        if seen_cell_ids.len() != points.len() {
            let missing: Vec<Point> = points
                .into_par_iter()
                .filter_map(|p| {
                    let cell_id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                    if seen_cell_ids.contains(&cell_id) {
                        None
                    } else {
                        Some(Point::new(self.radius, 20, *p))
                    }
                })
                .collect();
            solution.extend(missing);
        }
        log::info!("final solution size: {}", solution.len());
    }
    solution
}
```

Update `run` (greedy.rs:69-105). Replace its current body with:

```rust
pub fn run(&'a self, points: &SingleVec) -> SingleVec {
    let time = Instant::now();
    log::info!("starting algorithm with {} data points", points.len());

    let return_set = match self.cluster_mode {
        ClusterMode::Better | ClusterMode::Best => self.run_partitioned(points),
        _ => self.setup(points),
    };

    log::info!("finished in {:.2}s", time.elapsed().as_secs_f32());
    return_set.into_iter().map(|p| p.center).collect()
}
```

- [ ] **Step 4: Add deprecation warning to setter**

Replace `set_cluster_split_level` (greedy.rs:64-67) with:

```rust
pub fn set_cluster_split_level(&mut self, cluster_split_level: u64) -> &mut Self {
    if cluster_split_level != 0 {
        log::warn!(
            "cluster_split_level is deprecated and will be ignored. \
             Adaptive S2 partitioning is now automatic for Better/Best modes."
        );
    }
    self.cluster_split_level = cluster_split_level;
    self
}
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test -p algorithms`
Expected: All partition tests + smoke tests pass.

- [ ] **Step 6: Run clippy to catch obvious issues**

Run: `cargo clippy -p algorithms -- -D warnings`
Expected: No warnings. Fix any inline.

- [ ] **Step 7: Commit**

```bash
git add server/algorithms/src/clustering/greedy.rs server/algorithms/src/clustering/partition.rs
git commit -m "feat(greedy): adaptive partition for Better/Best modes + deprecate cluster_split_level"
```

---

## Task 11: Manual smoke against larger synthetic input + log inspection

**Files:**
- No file changes; this is a manual verification task.

- [ ] **Step 1: Add an `#[ignore]`-d benchmark-shaped test**

Append to the test module in `partition.rs`:

```rust
#[test]
#[ignore]  // run with: cargo test -p algorithms -- --ignored
fn manual_smoke_huge_bbox_better_mode() {
    use crate::clustering::greedy::Greedy;
    use std::time::Instant;

    let pts = random_points_in_bbox(50_000, [-45., -90., 45., 90.], 12345);
    let mut greedy = Greedy::default();
    greedy.set_cluster_mode(ClusterMode::Better).set_radius(70.0);

    let t = Instant::now();
    let result = greedy.run(&pts);
    let elapsed = t.elapsed();

    eprintln!(
        "manual_smoke: {} input pts → {} clusters in {:.2}s",
        pts.len(),
        result.len(),
        elapsed.as_secs_f32()
    );
    assert!(!result.is_empty());
}
```

- [ ] **Step 2: Run with `--ignored` and check log output**

Run:
```bash
RUST_LOG=info cargo test -p algorithms --lib clustering::partition::tests::manual_smoke_huge_bbox_better_mode -- --ignored --nocapture
```

Expected output includes:
- `PartitionConfig: budget=..., start_level=6, max_level=18`
- `partition: N chunks, downgrades: {...}` where N > 1
- Total elapsed under a reasonable bound (no specific assertion; this is for human inspection).

If the run OOMs or hangs, lower the test point count or set `KOJI_MAX_CANDIDATES_PER_CHUNK=100000` before running.

- [ ] **Step 3: Commit the ignored test**

```bash
git add server/algorithms/src/clustering/partition.rs
git commit -m "test(partition): add ignored manual smoke for huge bbox"
```

---

## Final Verification Checklist

- [ ] `cargo test -p algorithms` — all tests pass
- [ ] `cargo clippy -p algorithms -- -D warnings` — clean
- [ ] `cargo check --workspace` — no compile errors in dependents
- [ ] `cargo test -p algorithms -- --ignored --nocapture` — manual smoke passes, partition log shows expected shape
- [ ] `grep -n "cluster_split_level" server/algorithms/src/clustering/` — no removed-but-not-replaced references
- [ ] Final review of `git log --oneline main..HEAD` — clean linear history, conventional commits

## Open Risks & Notes for Implementer

1. **`Point` is `Clone + Copy`** — confirmed at `server/algorithms/src/rtree/point.rs:15` (`#[derive(Debug, Clone, Copy)]`). `.cloned()` calls in `gather_halo` are safe.

2. **`rstar` API drift** — `locate_in_envelope_intersecting` is the assumed method on `RTree`. Confirm against rstar 0.12.2 docs in `~/.cargo/registry/src/.../rstar-0.12.2/src/rtree.rs` if the call fails to compile.

3. **`create_cell_map` re-entry** — used in `adaptive_partition` to subdivide. Verify in `server/algorithms/src/s2.rs:353` that the level argument is `u64` and it dedupes points correctly when called recursively. It currently uses `parent(split_level)` which is safe for any level ≤ source level.

4. **Test threading** — rayon par_iter within tests can cause oversubscription. If tests flake, run with `--test-threads=1`.

5. **The `Greedy::cluster` method (greedy.rs:262-388)** is reused unchanged by `solve_chunk`. It already iterates `clusters_with_data` and produces a `HashSet<Cluster>`. No changes needed there.

6. **`Point::cell_id`** — referenced in `run_partitioned`'s missing-point recovery. Confirmed via `check_missing` in greedy.rs:411-415 that `Point` has a `cell_id` field. If it doesn't compile, use `from_array_to_cell_id(p.center, 20)` (s2.rs:344) instead.
