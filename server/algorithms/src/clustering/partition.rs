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
