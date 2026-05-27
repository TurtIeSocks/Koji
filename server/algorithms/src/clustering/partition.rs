use std::collections::HashMap;
use std::collections::HashSet as StdHashSet;
use std::sync::OnceLock;

use model::api::cluster_mode::ClusterMode;
use model::api::point_array::PointArray;
use model::api::single_vec::SingleVec;
use ::s2::cellid::CellID;
use ::s2::latlng::LatLng;
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
}
