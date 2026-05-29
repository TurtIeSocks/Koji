//! Composable algorithm/request configuration structs. `model`'s legacy `Args`
//! (and, later, the v2 request DTOs) map into these; the algorithms consume
//! them from P2 onward.

use crate::{
    CalculationMode, ClusterMode, GeoFormats, Precision, ReturnTypeArg, SortBy, SpawnpointTth,
};

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
    pub cluster_split_level: u64,
    pub calculation_mode: CalculationMode,
    pub s2: S2Config,
    pub center_clusters: bool,
    pub genetic_post_processing: bool,
    pub plugin_args: String,
}

#[derive(Debug, Clone)]
pub struct RoutingConfig {
    pub sort_by: SortBy,
    pub route_split_level: u64,
    pub plugin_args: String,
}

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub calculation_mode: CalculationMode,
    pub radius: Precision,
    pub s2: S2Config,
    pub plugin_args: String,
}

/// Raw area source + identity, pre-resolution to a `FeatureCollection`.
#[derive(Debug, Clone)]
pub struct AreaInput {
    pub area: Option<GeoFormats>,
    pub instance: String,
    pub geometry_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DataFilter {
    pub last_seen: u32,
    pub tth: SpawnpointTth,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub return_type: ReturnTypeArg,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub save_to_scanner_only: bool,
    pub simplify: bool,
}

/// Developer / experimental toggles + benchmark mode.
#[derive(Debug, Clone, Default)]
pub struct DevConfig {
    pub bypass_adaptive_partition: bool,
    pub benchmark_mode: bool,
}
