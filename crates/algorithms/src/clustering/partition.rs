use std::collections::HashMap;
use std::collections::HashSet as StdHashSet;
use std::sync::Once;

use ::s2::cell::Cell;
use ::s2::cellid::CellID;
use ::s2::latlng::LatLng;
use koji_core::Precision;
use koji_core::ClusterMode;
use koji_core::PointArray;
use koji_core::SingleVec;
use rstar::{AABB, RTree};
use sysinfo::System;

use crate::clustering::candidates;
use crate::clustering::rtree::point::Point;
use crate::s2::create_cell_map;

/// Shared 1024 constant used as both a unit byte size (1 KiB) and a candidate
/// grid-density base in greedy.rs. Centralized here to keep all `1024` magic
/// numbers in clustering on a single source of truth.
pub(crate) const BYTE: usize = 1024;

/// Bytes assumed per candidate when converting memory budget → candidate count.
/// PointArray (16 bytes) + Cluster<Point> overhead. Empirical: ~864 bytes for a
/// cluster with 100-point Vec<&Point> tail; rounded up to 1024 for safety so
/// auto-budget under-allocates rather than over-allocates on dense workloads.
/// Independent from BYTE (same numeric value today but different semantics).
pub(crate) const BYTES_PER_CANDIDATE: usize = 1024;

pub(crate) const DEFAULT_START_LEVEL: u64 = 6;
pub(crate) const DEFAULT_MAX_LEVEL: u64 = 18;

/// Guards PartitionConfig::load logging so each process logs the resolved
/// config exactly once, regardless of how many partitioned runs happen.
static LOG_ONCE: Once = Once::new();

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
    /// Counts downgrades by (from_tag, to_tag) pair, keyed by stable ClusterMode tags.
    pub downgrades: HashMap<(&'static str, &'static str), usize>,
}

/// Read `key` from env, parse as T, fall back to `default` if missing/malformed.
fn load_env_or<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<T>().ok())
        .unwrap_or(default)
}

impl PartitionConfig {
    pub(crate) fn load() -> PartitionConfig {
        let sys = System::new_all();
        let threads = rayon::current_num_threads().max(1) as u64;
        let mem_per_thread = sys.available_memory() / threads;
        let auto_budget = (mem_per_thread / BYTES_PER_CANDIDATE as u64) as usize;

        let budget = load_env_or("KOJI_MAX_CANDIDATES_PER_CHUNK", auto_budget);
        let start_level = load_env_or("KOJI_PARTITION_START_LEVEL", DEFAULT_START_LEVEL);
        let max_level = load_env_or("KOJI_PARTITION_MAX_LEVEL", DEFAULT_MAX_LEVEL);

        LOG_ONCE.call_once(|| {
            log::info!(
                "PartitionConfig: budget={}, start_level={}, max_level={}",
                budget, start_level, max_level
            );
        });
        PartitionConfig { budget, start_level, max_level }
    }
}

/// Stable string tag for a ClusterMode variant — used as HashMap key so callers don't
/// depend on `Debug` formatting, which can change. Custom plugins collapse to "Custom".
pub(crate) fn mode_tag(mode: &ClusterMode) -> &'static str {
    match mode {
        ClusterMode::Honeycomb => "Honeycomb",
        ClusterMode::Fastest => "Fastest",
        ClusterMode::Fast => "Fast",
        ClusterMode::Balanced => "Balanced",
        ClusterMode::Better => "Better",
        ClusterMode::Best => "Best",
        ClusterMode::Custom(_) => "Custom",
    }
}

/// Count the number of distinct level-16 S2 cells touched by the input points.
/// Used as the leading factor in the S2 walk cost estimate (see `s2_walk_cost`).
pub(crate) fn distinct_l16_cells(points: &[PointArray]) -> usize {
    points
        .iter()
        .map(|p| CellID::from(LatLng::from_degrees(p[0], p[1])).parent(16))
        .collect::<StdHashSet<_>>()
        .len()
}

/// Pick a grid density for Best mode that fits the remaining candidate budget.
///
/// `s2_cost` is the prior commitment from `s2_walk_cost`. Returns 0 if the S2
/// walk already saturates the budget (effectively collapsing Best to Better for
/// this chunk). Capped at `BYTE * 6` so we never exceed the original `Best`
/// density even when the budget is huge.
pub(crate) fn scaled_grid_density(s2_cost: usize, budget: usize) -> usize {
    let remaining = budget.saturating_sub(s2_cost);
    let density = (remaining as f64).sqrt() as usize;
    density.min(BYTE * 6)
}

/// Worst-case candidate count from the S2 cell walk used by Better/Best modes.
///
/// `get_s2_clusters` descends from level 16 → level 22 per occupied ancestor, so each
/// L16 cell can contribute up to 4^6 = 4096 candidate leaves. The per-level point-tree
/// filter prunes much of this on medium-density inputs, but on dense inputs (urban,
/// pokemon-spawn-style data) the descent runs close to worst case. We use the worst-case
/// value as the partition-budget estimator so peak memory is bounded even on dense data;
/// for medium-density inputs this triggers more partitioning than strictly necessary
/// (a few % quality cost) but is the only way to prevent OOM on dense huge bboxes.
pub(crate) const S2_WALK_COST_PER_L16: usize = 4096;

pub(crate) fn s2_walk_cost(points: &[PointArray]) -> usize {
    distinct_l16_cells(points).saturating_mul(S2_WALK_COST_PER_L16)
}

pub(crate) fn estimate_cost(
    points: &[PointArray],
    mode: &ClusterMode,
    budget: usize,
) -> usize {
    let s2_cost = s2_walk_cost(points);
    let grid_cost = if matches!(mode, ClusterMode::Best) {
        scaled_grid_density(s2_cost, budget).pow(2)
    } else {
        0
    };
    s2_cost.saturating_add(grid_cost)
}

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
    let mut bb = LatLonBBox {
        min_lat: Precision::INFINITY,
        max_lat: Precision::NEG_INFINITY,
        min_lon: Precision::INFINITY,
        max_lon: Precision::NEG_INFINITY,
    };
    for i in 0..4 {
        let v = c.vertex(i);
        let lat = v.latitude().deg();
        let lon = v.longitude().deg();
        bb.min_lat = bb.min_lat.min(lat);
        bb.max_lat = bb.max_lat.max(lat);
        bb.min_lon = bb.min_lon.min(lon);
        bb.max_lon = bb.max_lon.max(lon);
    }
    bb
}

/// Ownership predicate: does `p` belong to `cell` at the cell's own level?
///
/// Used both during partition (implicitly via `create_cell_map`) and during the
/// post-solve ownership filter in `Greedy::solve_chunk`. Both sides apply the
/// same deterministic `LatLng → parent(level)` rule so a cluster center is
/// owned by exactly one chunk.
pub(crate) fn contains_latlng(cell: CellID, p: PointArray) -> bool {
    let derived = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(cell.level());
    derived == cell
}

/// Collect halo points: those that intersect `cell`'s bbox expanded by `radius_meters`
/// but do NOT belong to `cell`. Halo points participate in a chunk's solve as
/// coverage targets but never own cluster centers, so adjacent chunks can place
/// edge clusters fairly without producing duplicates.
pub(crate) fn gather_halo(
    cell: CellID,
    all_points_tree: &RTree<Point>,
    radius_meters: Precision,
) -> Vec<Point> {
    let bbox = cell_bbox_lat_lon(cell);
    let radius_deg = candidates::meters_to_degrees(radius_meters, bbox.center_lat());
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

/// Top-down BFS partitioner. Buckets points by S2 cell at `start_level`, then
/// for each bucket: accept the chunk if its estimated candidate cost ≤ `budget`
/// or if it has reached `max_level`; otherwise subdivide into the cell's children
/// and re-evaluate. Returns the accepted chunks; their union covers all input
/// points without duplication.
///
/// Termination: bounded loop depth = `max_level - start_level`. At `max_level`,
/// over-budget chunks are accepted with a warn log and handled later by
/// `select_effective_mode`'s downgrade ladder.
pub(crate) fn adaptive_partition(
    points: &SingleVec,
    budget: usize,
    mode: &ClusterMode,
    start_level: u64,
    max_level: u64,
) -> Vec<Chunk> {
    debug_assert!(
        start_level <= max_level,
        "start_level ({}) must be <= max_level ({})",
        start_level,
        max_level,
    );
    if points.is_empty() {
        return vec![];
    }
    let mut frontier: HashMap<u64, SingleVec> = create_cell_map(points, start_level);
    let mut accepted: Vec<Chunk> = Vec::with_capacity(frontier.len());
    let mut next_frontier: HashMap<u64, SingleVec> = HashMap::with_capacity(frontier.len() * 4);

    loop {
        next_frontier.clear();
        for (cell_id_raw, cell_points) in frontier.drain() {
            let cell = CellID(cell_id_raw);
            let est = estimate_cost(&cell_points, mode, budget);
            let at_max_level = cell.level() >= max_level;
            let within_budget = est <= budget;

            if within_budget || at_max_level {
                if at_max_level && !within_budget {
                    log::warn!(
                        "partition: accepting chunk at max_level={} with est={} > budget={} (irreducible)",
                        cell.level(), est, budget,
                    );
                }
                accepted.push(Chunk { cell, owned: cell_points });
            } else {
                let children = create_cell_map(&cell_points, cell.level() + 1);
                for (k, v) in children {
                    next_frontier.entry(k).or_default().extend(v);
                }
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        std::mem::swap(&mut frontier, &mut next_frontier);
    }
    accepted
}

/// Stepwise mode downgrade ladder: `Best → Better → Balanced → Fast`.
///
/// Returns the highest-quality mode whose estimated cost fits `budget`, or
/// `Fast` if even that exceeds budget (Fast is the floor; pathological chunks
/// always complete). Each downgrade is logged at WARN. Only meaningful when
/// called with Better or Best — checked by debug_assert.
pub(crate) fn select_effective_mode(
    mut requested: ClusterMode,
    points: &[PointArray],
    budget: usize,
) -> ClusterMode {
    debug_assert!(
        matches!(requested, ClusterMode::Better | ClusterMode::Best),
        "select_effective_mode is only meaningful for Better/Best; got {:?}",
        requested,
    );
    loop {
        let est = estimate_cost(points, &requested, budget);
        if matches!(requested, ClusterMode::Fast) || est <= budget {
            return requested;
        }
        let next = match requested {
            ClusterMode::Best => ClusterMode::Better,
            ClusterMode::Better => ClusterMode::Balanced,
            ClusterMode::Balanced => ClusterMode::Fast,
            _ => return requested,
        };
        log::warn!(
            "chunk over budget for {:?} (est={}, budget={}), downgrading to {:?}",
            requested, est, budget, next,
        );
        requested = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::Precision;
    use rand::{Rng, SeedableRng, rngs::SmallRng};

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
        assert_eq!(big, BYTE * 6);
        // moderate: sqrt(remaining) used
        let mid = scaled_grid_density(0, 1_000_000);
        assert_eq!(mid, 1000);
    }

    #[test]
    fn estimate_cost_better_excludes_grid() {
        let points = dense_cluster([0., 0.], 100, 50.0, 2);
        let est_better = estimate_cost(&points, &ClusterMode::Better, 10_000_000);
        let est_best = estimate_cost(&points, &ClusterMode::Best, 10_000_000);
        assert!(est_best >= est_better, "Best must be >= Better");
    }

    #[test]
    fn estimate_cost_best_scales_with_budget() {
        let points = vec![[0.0, 0.0]];
        let est_small = estimate_cost(&points, &ClusterMode::Best, 100);
        let est_large = estimate_cost(&points, &ClusterMode::Best, 100_000_000);
        assert!(est_small < est_large, "larger budget should allow larger grid");
    }

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

    #[test]
    fn partition_small_bbox_single_chunk() {
        // 1000 points in roughly 1km² → should fit in one chunk at start_level=6.
        let pts = random_points_in_bbox(1000, [37.78, -122.43, 37.79, -122.42], 42);
        let chunks = adaptive_partition(&pts, usize::MAX, &ClusterMode::Better, 6, 18);
        assert_eq!(chunks.len(), 1, "small bbox should be one chunk, got {}", chunks.len());
    }

    #[test]
    fn partition_subdivides_when_over_budget() {
        // Force subdivision with a tiny budget.
        let pts = sparse_grid(20, 20, [-30., -60., 30., 60.]);
        let chunks = adaptive_partition(&pts, 1, &ClusterMode::Better, 6, 18);
        assert!(chunks.len() > 1, "tiny budget should produce many chunks");
    }

    #[test]
    fn partition_halts_at_max_level() {
        // Dense cluster + extremely tight budget → at least one chunk at max_level=18.
        let pts = dense_cluster([0., 0.], 5_000, 50.0, 11);
        let chunks = adaptive_partition(&pts, 1, &ClusterMode::Best, 6, 18);
        assert!(
            chunks.iter().any(|c| c.cell.level() == 18),
            "expected at least one chunk to reach max_level"
        );
    }

    #[test]
    fn partition_preserves_all_points() {
        let pts = random_points_in_bbox(500, [0., 0., 5., 5.], 99);
        let chunks = adaptive_partition(&pts, 1_000_000, &ClusterMode::Better, 6, 18);
        let total: usize = chunks.iter().map(|c| c.owned.len()).sum();
        assert_eq!(total, pts.len(), "no points should be dropped during partition");
    }

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
            halo.iter().any(|p: &Point| (p.center[1] - outside[1]).abs() < 1e-6),
            "halo should include the just-outside point"
        );
        assert!(
            !halo.iter().any(|p: &Point| (p.center[1] - inside[1]).abs() < 1e-6),
            "halo should NOT include the inside point"
        );
    }

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

    /// Load a CSV file in `lat,lon\n...` format. Skips header row.
    /// Returns a SingleVec of points. Panics on parse error.
    fn load_csv(path: &str) -> SingleVec {
        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("failed to read {path}: {e}"));
        contents
            .lines()
            .skip(1) // header
            .filter_map(|line| {
                let mut parts = line.split(',');
                let lat = parts.next()?.trim().parse::<Precision>().ok()?;
                let lon = parts.next()?.trim().parse::<Precision>().ok()?;
                Some([lat, lon])
            })
            .collect()
    }

    fn run_bench(mode: ClusterMode, label: &str) {
        use crate::clustering::greedy::Greedy;
        use crate::rtree;
        use std::time::Instant;

        // CSV expected at the repo root, or one level up from the crate dir.
        let path = if std::path::Path::new("points-nh.csv").exists() {
            "points-nh.csv"
        } else if std::path::Path::new("../points-nh.csv").exists() {
            "../points-nh.csv"
        } else {
            eprintln!("bench_nh_{label}: skipped (points-nh.csv not found)");
            return;
        };

        let load_t = Instant::now();
        let pts = load_csv(path);
        eprintln!(
            "bench_nh_{label}: loaded {} points in {:.2}s",
            pts.len(),
            load_t.elapsed().as_secs_f32()
        );

        let mut greedy = Greedy::default();
        greedy
            .set_cluster_mode(mode)
            .set_radius(70.0)
            .set_min_points(5);

        let run_t = Instant::now();
        let result = greedy.run(&pts);
        let elapsed = run_t.elapsed();

        // Compute coverage: how many input points are within radius of at least one cluster center?
        let cluster_tree = rtree::spawn(70.0, &result);
        let covered: usize = pts
            .iter()
            .filter(|p| cluster_tree.locate_at_point(p).is_some())
            .count();
        let coverage_pct = covered as f64 * 100.0 / pts.len() as f64;
        // mygod_score = clusters * min_points + uncovered_points (lower = better).
        // Source of truth: stats.rs::Stats::get_score.
        let uncovered = pts.len() - covered;
        let mygod_score = result.len() * 5 + uncovered; // min_points=5

        eprintln!(
            "bench_nh_{label}: mode={} radius=70 min_points=5 -> {} clusters, {:.2}% coverage ({}/{}), mygod_score={} in {:.2}s",
            label,
            result.len(),
            coverage_pct,
            covered,
            pts.len(),
            mygod_score,
            elapsed.as_secs_f32()
        );
    }

    #[test]
    #[ignore]
    fn bench_nh_balanced() {
        run_bench(ClusterMode::Balanced, "balanced");
    }

    #[test]
    #[ignore]
    fn bench_nh_better() {
        run_bench(ClusterMode::Better, "better");
    }

    #[test]
    #[ignore]
    fn bench_nh_best() {
        run_bench(ClusterMode::Best, "best");
    }

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
}
