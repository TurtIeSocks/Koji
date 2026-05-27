use std::collections::HashMap;
use std::sync::OnceLock;

use model::api::cluster_mode::ClusterMode;
use model::api::single_vec::SingleVec;
use ::s2::cellid::CellID;
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
