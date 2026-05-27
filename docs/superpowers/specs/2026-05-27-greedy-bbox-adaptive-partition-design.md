# Greedy Clustering — Adaptive S2 Partition (Short-Term Patch)

**Date:** 2026-05-27
**Status:** Design
**Scope:** Short-term patch for `ClusterMode::Better` and `ClusterMode::Best` candidate-count explosion on huge bboxes. Long-term algorithm rework is out of scope; this patch is a contained, reversible safety net.

## Problem

When a user selects `Better` or `Best` and the input points are spread across a very large bbox, the greedy algorithm in [`server/algorithms/src/clustering/greedy.rs`](../../../server/algorithms/src/clustering/greedy.rs) generates tens to hundreds of millions of candidate cluster centers:

1. `get_s2_clusters` walks every level-16 S2 cell in the bbox down to level 22, producing up to `4096` leaves per occupied level-16 ancestor.
2. `Best` mode additionally calls `gen_clusters(BYTE * 6, points)` which produces `(BYTE * 6)² = 37,748,736` jittered grid candidates regardless of bbox size.
3. The existing `cluster_split_level` setter is documented internally as not recommended (overlaps with rayon parallelism, manual tuning required).

This causes memory pressure, slow runs, and occasionally OOM.

## Non-Goals

- Cluster quality improvement (coverage %, packing efficiency).
- Replacing the greedy algorithm.
- Adding committed real-world test datasets.
- Public API additions (no new caller-facing setters).

## Design

### Section 1 — Architecture Overview

Adaptive partitioner sits between `Greedy::run` and `Greedy::setup` for `Better` and `Best` modes only. All other modes (`Honeycomb`, `Balanced`, `Fast`, `Custom`) keep the existing path.

```
Greedy::run(points)
  ├─ if cluster_mode ∈ {Better, Best}:
  │     ├─ adaptive_partition(points, budget) → Vec<Chunk>
  │     ├─ par_iter chunks → solve_chunk(chunk) → HashSet<Point>
  │     ├─ filter centers by cell-of-center ownership (inside solve_chunk)
  │     └─ union → final SingleVec
  └─ else: existing path unchanged
```

The `cluster_split_level` field and setter are retained for source compatibility but the value is ignored. Non-zero values logged as deprecation warning.

A new module `server/algorithms/src/clustering/partition.rs` houses the partitioner, cost estimator, halo gatherer, ownership filter, and fallback ladder.

**Orchestration sketch** (private method on `Greedy`):

```rust
fn run_partitioned(&self, points: &SingleVec) -> HashSet<Point> {
    let config = PartitionConfig::load();  // OnceLock-cached env+sysinfo
    let all_points_tree: RTree<Point> = rtree::spawn(self.radius, points);
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

    log::info!("partition: {} chunks, downgrades: {:?}", stats.total_chunks, stats.downgrades);

    if self.min_points == 1 {
        // Cluster -> Point conversion already done in solve_chunk via .into();
        // re-use check_missing semantics (greedy.rs:409) against the original points.
        self.check_missing_points(solution, points)
    } else {
        solution
    }
}
```

`check_missing_points` is a small refactor of the existing `check_missing` (greedy.rs:408-441) to accept a `HashSet<Point>` directly instead of `Vec<Cluster>`. Same logic, different input type.

### Section 2 — Adaptive S2 Partition Algorithm

**Types:**

```rust
pub(crate) struct Chunk {
    pub cell: CellID,        // owns clusters whose centers parent to this cell at cell.level()
    pub owned: SingleVec,    // points whose level-`cell.level()` parent == cell
}

pub(crate) struct PartitionConfig {
    pub budget: usize,
    pub start_level: u64,
    pub max_level: u64,
}

#[derive(Default)]
pub(crate) struct PartitionStats {
    pub total_chunks: usize,
    pub downgrades: HashMap<(ClusterMode, ClusterMode), usize>,
}
```

Top-down BFS descent. Start at a coarse S2 level, subdivide any cell whose estimated cost exceeds the budget. Halt at `MAX_LEVEL` regardless.

```rust
fn adaptive_partition(
    points: &SingleVec,
    budget: usize,
    mode: ClusterMode,
    start_level: u64,
    max_level: u64,
) -> Vec<Chunk> {
    // create_cell_map (server/algorithms/src/s2.rs:353) returns HashMap<u64, SingleVec>
    // where the key is CellID.0 (the raw u64 id).
    let mut frontier: HashMap<u64, SingleVec> = create_cell_map(points, start_level);
    let mut accepted: Vec<Chunk> = vec![];

    loop {
        let mut next_frontier: HashMap<u64, SingleVec> = HashMap::new();
        for (cell_id_raw, cell_points) in frontier {
            let cell = CellID(cell_id_raw);
            let est = estimate_cost(&cell_points, mode, budget);
            if est <= budget || cell.level() >= max_level {
                accepted.push(Chunk { cell, owned: cell_points });
            } else {
                let children = create_cell_map(&cell_points, cell.level() + 1);
                next_frontier.extend(children);
            }
        }
        if next_frontier.is_empty() { break; }
        frontier = next_frontier;
    }

    accepted
}
```

**Defaults:**
- `START_LEVEL = 6` — ~165km cells at equator. One chunk for small bbox, partitioned for large.
- `MAX_LEVEL = 18` — ~80m cells. Below this, cell size approaches cluster radius (70m), further subdivision is degenerate.

Both env-overridable: `KOJI_PARTITION_START_LEVEL`, `KOJI_PARTITION_MAX_LEVEL`.

`gather_halo` (Section 4) is computed lazily inside `solve_chunk` against a shared `Arc<RTree<Point>>` built once over all points before partitioning.

### Section 3 — Cost Estimator

Estimates the actual candidate count a chunk will materialize under the requested mode, **accounting for grid density scaling**. Used by both the partitioner (Section 2) and the fallback ladder (Section 5) so both reason about the same number.

```rust
const BYTES_PER_CANDIDATE: usize = 256;

fn distinct_l16_cells(points: &[PointArray]) -> usize {
    points.iter()
        .map(|p| CellID::from(LatLng::from_degrees(p[0], p[1])).parent(16))
        .collect::<HashSet<_>>()
        .len()
}

fn scaled_grid_density(s2_cost: usize, budget: usize) -> usize {
    let remaining = budget.saturating_sub(s2_cost);
    let density = (remaining as f64).sqrt() as usize;
    density.min(BYTE * 6)
}

fn estimate_cost(points: &[PointArray], mode: ClusterMode, budget: usize) -> usize {
    let s2_cost = distinct_l16_cells(points) * 4096;  // 4^(22-16)
    let grid_cost = if mode == ClusterMode::Best {
        scaled_grid_density(s2_cost, budget).pow(2)
    } else {
        0
    };
    s2_cost + grid_cost
}
```

**Grid density scaling is applied inside `solve_chunk` for Best mode**:

```rust
let s2_cost_actual = distinct_l16_cells(&combined) * 4096;
let grid_density = scaled_grid_density(s2_cost_actual, budget);
```

If `s2_cost_actual >= budget`, `grid_density = 0` → grid contribution dropped, S2 walk only. This collapses Best to Better effectively without invoking the fallback ladder. The estimator returns the same scaled cost, so the ladder is only invoked when even the S2 walk alone exceeds budget.

**Budget computation:**

```rust
let sys = System::new_all();
let mem_per_thread = sys.available_memory() / rayon::current_num_threads() as u64;
let auto_budget = (mem_per_thread / BYTES_PER_CANDIDATE as u64) as usize;
let budget = std::env::var("KOJI_MAX_CANDIDATES_PER_CHUNK")
    .ok()
    .and_then(|s| s.parse().ok())
    .unwrap_or(auto_budget);
```

`BYTES_PER_CANDIDATE = 256` is empirical: `PointArray` (16 bytes) plus the per-cluster `Cluster<Point>` overhead with its associated point Vec.

Config cached in `OnceLock<PartitionConfig>` to avoid repeated env parsing. Effective values logged at first use.

### Section 4 — Halo + Ownership Semantics

A single `RTree<Point>` is built once over all input points before partitioning and shared (via `&RTree<Point>` borrow into the rayon `par_iter`) across all chunks. This avoids per-chunk rtree rebuild and lets each chunk efficiently query its halo region.

**Halo gather:**

```rust
fn gather_halo(
    cell: CellID,
    all_points_tree: &RTree<Point>,
    radius_meters: f64,
) -> Vec<Point> {
    // Cell bbox from S2: Cell::from(&cell).rect_bound() returns an S2 Rect,
    // which we project to [min_lat, min_lon, max_lat, max_lon].
    let cell_bbox = cell_bbox_lat_lon(cell);
    let radius_deg = meters_to_degrees(radius_meters, cell_bbox.center_lat());
    let expanded = cell_bbox.expand(radius_deg);
    all_points_tree
        .locate_in_envelope_intersecting(&expanded.envelope())
        .filter(|p| !contains_latlng(cell, p.center))
        .cloned()
        .collect()
}
```

**Helpers introduced by this patch** (live in `partition.rs`):

```rust
fn cell_bbox_lat_lon(cell: CellID) -> BBox;             // S2 Rect → [min/max lat/lon]
fn contains_latlng(cell: CellID, p: [f64; 2]) -> bool;  // CellID::from(LatLng).parent(cell.level()) == cell
```

`contains_latlng` is implemented by re-deriving the cell at the partition level from the lat/lon and comparing to the chunk's owning cell — this is the same deterministic rule used for ownership, so no separate tie-break is required.

**Per-chunk solve:**

```rust
fn solve_chunk(
    &self,
    chunk: &Chunk,
    all_points_tree: &RTree<Point>,
    budget: usize,
) -> (HashSet<Point>, Option<(ClusterMode, ClusterMode)>) {
    let halo = gather_halo(chunk.cell, all_points_tree, self.radius);
    let combined: Vec<PointArray> = chunk.owned.iter().cloned()
        .chain(halo.iter().map(|p| p.center))
        .collect();

    let effective_mode = select_effective_mode(self.cluster_mode, &combined, budget);
    let downgrade = (effective_mode != self.cluster_mode)
        .then(|| (self.cluster_mode, effective_mode));

    let point_tree: RTree<Point> = rtree::spawn(self.radius, &combined);
    let clusters = self.associate_clusters_for_chunk(
        &combined,
        &point_tree,
        effective_mode,
        budget,  // for grid density scaling, see Section 3
    );
    let mut solution: Vec<Cluster> = self.cluster(&clusters).into_iter().collect();
    self.update_unique(&mut solution);

    solution.retain(|cluster| contains_latlng(chunk.cell, cluster.point.center));
    let result: HashSet<Point> = solution.into_iter().map(Into::into).collect();
    (result, downgrade)
}
```

**Critical semantics:**
- Halo points participate as targets (can be covered) but cluster centers landing in halo (i.e. outside owned cell) are dropped via `retain`.
- `CellID::from(LatLng).parent(level)` is deterministic — a cluster center is owned by exactly one cell. No tie-break logic needed.
- Halo radius = cluster `radius` exactly. Sufficient because a cluster center MUST be inside its owned cell; the worst case is a center at the cell edge, which reaches `radius` into the neighbor.

**Halo cost:** for a level-6 cell (~27,225 km² area, ~660 km perimeter) with 70m radius, halo area ≈ 46.2 km² ≈ 0.17% of cell area. Negligible.

**Final merge:** plain `HashSet::extend` across all chunks. Ownership filter guarantees zero center duplicates.

### Section 5 — Stepwise Fallback Ladder

A chunk at `MAX_LEVEL` with `estimate_cost > budget` is irreducible by partitioning. Apply per-chunk mode downgrade:

```rust
fn select_effective_mode(
    requested: ClusterMode,
    points: &[PointArray],
    budget: usize,
) -> ClusterMode {
    let mut current = requested;
    loop {
        if estimate_cost(points, current, budget) <= budget || current == ClusterMode::Fast {
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

**Ladder ordering rationale:**
- `Best → Better` — drops the grid candidate source entirely. Note: because `estimate_cost` already accounts for grid density scaling (Section 3), this step only triggers when even the S2 walk alone exceeds budget — at which point `Better` is also over budget and the loop continues straight to `Balanced`. Kept in the ladder for log granularity and forward-compat.
- `Better → Balanced` — drops the S2 cell walk, falls back to a `BYTE = 1024` grid (~1M candidates).
- `Balanced → Fast` — further reduces grid density to `BYTE / 2 = 512` (~262K candidates).
- `Fast` is the floor; even pathological chunks complete.

`solve_chunk` is parameterized by `effective_mode`. The existing mode switch in `associate_clusters` (greedy.rs:168-181) is extracted into a new helper `associate_clusters_for_chunk(points, point_tree, effective_mode, budget)` that takes the resolved mode as a parameter rather than reading `self.cluster_mode`. The rest of `associate_clusters`'s body (filter by min_points, memory log, bucket by cluster size) is reused.

**Observability:**

Two layers:

1. **Per-downgrade `log::warn!`** inside `select_effective_mode` — already shown above. Grep-able in logs to count occurrences after the fact.
2. **Summary `log::info!` per `run` call** — `solve_chunk` returns both the cluster set and an `Option<(ClusterMode, ClusterMode)>` recording the original→effective mode (if any downgrade happened). The orchestrator aggregates these into `PartitionStats` after `par_iter`:

```rust
let results: Vec<(HashSet<Point>, Option<(ClusterMode, ClusterMode)>)> = chunks
    .par_iter()
    .map(|chunk| self.solve_chunk(chunk, &all_points_tree, config.budget))
    .collect();

for (_, dg) in &results {
    if let Some(pair) = dg { *stats.downgrades.entry(*pair).or_insert(0) += 1; }
}
log::info!("partition: {} chunks, downgrades: {:?}", stats.total_chunks, stats.downgrades);
```

No metrics-system dependency. Logs only.

### Section 6 — API & Config Changes

**Public API surface impact: zero.** No new setters, no removed setters, no signature changes.

**`Greedy::set_cluster_split_level`** — retained, but body adds:

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

**`Greedy::run`** — branch removed:

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

**Caller site (`server/algorithms/src/clustering/mod.rs:57-66`)** — unchanged. Deprecation warning fires from the setter when a non-zero value is passed.

**Env vars (new, all optional):**

| Variable | Default | Purpose |
| --- | --- | --- |
| `KOJI_MAX_CANDIDATES_PER_CHUNK` | auto from `sysinfo` | Override per-chunk budget |
| `KOJI_PARTITION_START_LEVEL` | `6` | Coarse S2 level to start descent |
| `KOJI_PARTITION_MAX_LEVEL` | `18` | Halt descent at this depth |

All cached in `OnceLock<PartitionConfig>`. Effective values logged at startup.

### Section 7 — Testing & Verification

**Test helpers** in `server/algorithms/src/clustering/partition.rs#[cfg(test)] mod tests`:

```rust
fn random_points_in_bbox(n: usize, bbox: [f64; 4], seed: u64) -> SingleVec;
fn dense_cluster(center: [f64; 2], n: usize, radius_m: f64, seed: u64) -> SingleVec;
fn sparse_grid(rows: usize, cols: usize, bbox: [f64; 4]) -> SingleVec;
```

All use `SmallRng::seed_from_u64` for determinism. No data files committed.

**Unit tests:**

1. `partition_small_bbox_single_chunk` — 1000 points in ~1km² bbox → expect 1 chunk at `START_LEVEL`.
2. `partition_huge_sparse_bbox_subdivides` — `sparse_grid(50, 50, [-45., -90., 45., 90.])` → chunk count > 1, bounded by occupied cells.
3. `partition_dense_cluster_caps_at_max_level` — `dense_cluster([0., 0.], 50_000, 100., 42)` → at least one chunk reaches `MAX_LEVEL`.
4. `fallback_ladder_best_to_fast` — synthetic input with budget overridden to 100: assert `select_effective_mode` descends through `Best → Better → Balanced → Fast`, one log per step.
5. `halo_includes_boundary_points` — 2 adjacent cells, points within `radius` across boundary: assert chunk A's combined set contains chunk B's near-boundary points.
6. `ownership_filter_drops_foreign_centers` — inject a `Cluster` whose `point.center` parents to a foreign cell at partition level: assert filtered out by `solve_chunk`.
7. `cost_estimator_loose_bound` — for `random_points_in_bbox(n, bbox)`, assert `estimate_cost ≤ 4× actual_candidate_vec_len` after running `associate_clusters`. Loose sanity check, not strict equality.

**Smoke / shape tests:**

8. `greedy_better_completes_huge_random_bbox` — `random_points_in_bbox(10_000, [-10., -10., 10., 10.], 42)`, `ClusterMode::Better` → completes (no panic), returns non-empty `SingleVec`.
9. `greedy_best_completes_huge_random_bbox` — same with `Best`.

**Regression:**

10. Existing tests in `server/algorithms/src/clustering/` continue to pass unchanged for normal bbox sizes (where partitioner stays at single chunk at `START_LEVEL`).

**Deliberately skipped:**
- Real-world dataset tests (per project preference for now).
- Cluster quality regression (% coverage delta). Out of scope.
- Perf wall-clock assertions (CI variance too noisy).

## Implementation Order

1. Create `partition.rs` skeleton with `PartitionConfig`, `Chunk` struct, env loader.
2. Implement `estimate_cost` + `distinct_l16_cells` + unit tests for cost math.
3. Implement `adaptive_partition` (Section 2) + unit tests for partition shape.
4. Implement `gather_halo` + `solve_chunk` with ownership filter + unit tests for halo/ownership.
5. Implement `select_effective_mode` (fallback ladder) + tests.
6. Wire `run_partitioned` into `Greedy::run` for Better/Best modes.
7. Add deprecation warning to `set_cluster_split_level`.
8. Run smoke tests, full existing test suite.

## Open Risks

- `BYTES_PER_CANDIDATE = 256` is empirical and may be wrong by ±2× on real workloads. Env override exists if so.
- S2 cell bbox at `START_LEVEL = 6` near the poles has weird aspect ratios. For now, accept — Better/Best near the poles is an unusual case.
- Rayon parallelism within chunks (existing `par_iter` in `associate_clusters` and `cluster`) plus rayon across chunks could over-subscribe threads. Acceptable for short-term; long-term may want a dedicated thread pool or chunk-level scheduling.

## Long-Term Considerations (Out of Scope)

Listed here only as context for the upcoming algorithm rework:

- Replace S2 walk + grid candidate generation with a unified spatial-index-driven candidate stream.
- Boundary cluster optimality (current halo+ownership is correct but not globally optimal; a center near a boundary may still be sub-optimal vs. one a neighbor cell would have placed).
- Quality metric / regression harness with synthetic ground-truth.
