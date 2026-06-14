//! Wasm-boundary DTOs. tsify derives generate the TypeScript `.d.ts` interfaces;
//! `serde` drives (de)serialization across the JS boundary. These mirror a flat,
//! demo-friendly subset of `algorithms::clustering::ClusteringConfig`.

use serde::{Deserialize, Serialize};
use tsify::Tsify;

/// Clustering request. `points` is `[[lat, lng], ...]`.
#[derive(Tsify, Deserialize)]
#[tsify(from_wasm_abi)]
pub struct ClusterRequest {
    pub points: Vec<[f64; 2]>,
    pub radius: f64,
    pub min_points: usize,
    pub max_clusters: usize,
    /// "fastest" | "balanced" | "fast" | "better" | "best" | "honeycomb"
    #[serde(default = "default_mode")]
    pub cluster_mode: String,
    /// "radius" | "s2"
    #[serde(default = "default_calc")]
    pub calculation_mode: String,
    #[serde(default)]
    pub center_clusters: bool,
}

fn default_mode() -> String {
    "balanced".to_string()
}
fn default_calc() -> String {
    "radius".to_string()
}

/// Clustering result: cluster centers + a stats summary.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct ClusterResponse {
    pub clusters: Vec<[f64; 2]>,
    pub stats: StatsSummary,
}

/// Demo-facing subset of `algorithms::stats::Stats`.
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct StatsSummary {
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub cluster_time_ms: f64,
}
