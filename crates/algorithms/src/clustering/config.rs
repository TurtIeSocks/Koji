//! Clustering algorithm-parameter config. `S2Config` is shared with bootstrap;
//! it lives here and bootstrap imports it via `crate::clustering::S2Config`.

use koji_core::Precision;

use super::{CalculationMode, ClusterMode};

/// S2 grid parameters shared by clustering + bootstrapping.
#[derive(Debug, Clone, Copy, Default)]
pub struct S2Config {
    pub level: u8,
    pub size: u8,
}

#[derive(Debug, Clone)]
pub struct ClusteringConfig {
    pub mode: ClusterMode,
    pub radius: Precision,
    pub min_points: usize,
    pub max_clusters: usize,
    pub calculation_mode: CalculationMode,
    pub s2: S2Config,
    pub center_clusters: bool,
    pub genetic_post_processing: bool,
    pub plugin_args: String,
}
