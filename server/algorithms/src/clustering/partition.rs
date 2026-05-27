use std::collections::HashMap;
use std::collections::HashSet as StdHashSet;
use std::sync::Once;

use ::s2::cell::Cell;
use ::s2::cellid::CellID;
use ::s2::latlng::LatLng;
use model::api::Precision;
use model::api::cluster_mode::ClusterMode;
use model::api::point_array::PointArray;
use model::api::single_vec::SingleVec;
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
    density.min(BYTE * 6)
}

/// Worst-case candidate count from the S2 cell walk used by Better/Best modes.
///
/// `get_s2_clusters` descends from level 16 → level 22 per occupied ancestor,
/// so each distinct level-16 cell touched by the input contributes up to 4^6 = 4096
/// candidate leaves. Real output is usually smaller due to per-level point-tree
/// filtering, but this is the honest upper bound used by the partition budget.
pub(crate) fn s2_walk_cost(points: &[PointArray]) -> usize {
    distinct_l16_cells(points).saturating_mul(4096)
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

pub(crate) fn contains_latlng(cell: CellID, p: PointArray) -> bool {
    let derived = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(cell.level());
    derived == cell
}

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

pub(crate) fn adaptive_partition(
    points: &SingleVec,
    budget: usize,
    mode: &ClusterMode,
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
        frontier = next_frontier;
    }
    accepted
}

pub(crate) fn select_effective_mode(
    requested: ClusterMode,
    points: &[PointArray],
    budget: usize,
) -> ClusterMode {
    debug_assert!(
        matches!(requested, ClusterMode::Better | ClusterMode::Best),
        "select_effective_mode is only meaningful for Better/Best; got {:?}",
        requested,
    );
    let mut current = requested;
    loop {
        let est = estimate_cost(points, &current, budget);
        if matches!(current, ClusterMode::Fast) || est <= budget {
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
            current, est, budget, next,
        );
        current = next;
    }
}

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
    fn ownership_filter_drops_foreign_centers() {
        use crate::clustering::greedy::Greedy;

        // Two non-adjacent dense clusters; partition should produce >=2 chunks.
        // Each chunk's solve_chunk result must contain ONLY centers parenting to its cell.
        let mut pts = dense_cluster([10.0, 20.0], 100, 50.0, 1);
        pts.extend(dense_cluster([40.0, 80.0], 100, 50.0, 2));

        let mut greedy = Greedy::default();
        greedy.set_cluster_mode(ClusterMode::Better).set_radius(70.0);

        let chunks = adaptive_partition(&pts, usize::MAX, &ClusterMode::Better, 6, 18);
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
