//! DTO ↔ koji-core conversions. This is the single validation boundary between
//! untyped JS input and the typed algorithm.

use koji_core::{CalculationMode, ClusterMode, ClusteringConfig, S2Config, SingleVec};
use wasm_bindgen::JsError;

use crate::dto::{ClusterRequest, ClusterResponse, StatsSummary};

/// Parse a string into a koji-core enum via its existing `Deserialize` impl.
fn parse_mode<T: serde::de::DeserializeOwned>(s: &str, what: &str) -> Result<T, JsError> {
    serde_json::from_value::<T>(serde_json::Value::String(s.to_string()))
        .map_err(|e| JsError::new(&format!("invalid {what} '{s}': {e}")))
}

impl ClusterRequest {
    /// Split into the point list + the koji-core config the algorithm consumes.
    pub fn into_core(self) -> Result<(SingleVec, ClusteringConfig), JsError> {
        let mode: ClusterMode = parse_mode(&self.cluster_mode, "cluster_mode")?;
        let calculation_mode: CalculationMode =
            parse_mode(&self.calculation_mode, "calculation_mode")?;
        let cfg = ClusteringConfig {
            mode,
            radius: self.radius,
            min_points: self.min_points,
            max_clusters: self.max_clusters,
            cluster_split_level: 0,
            calculation_mode,
            s2: S2Config::default(),
            center_clusters: self.center_clusters,
            genetic_post_processing: false,
            plugin_args: String::new(),
        };
        Ok((self.points, cfg))
    }
}

impl ClusterResponse {
    /// Build the response from the cluster centers + the populated `Stats`.
    pub fn from_parts(clusters: SingleVec, stats: &algorithms::stats::Stats) -> Self {
        ClusterResponse {
            stats: StatsSummary {
                total_points: stats.total_points,
                points_covered: stats.points_covered,
                total_clusters: stats.total_clusters,
                cluster_time_ms: stats.cluster_time as f64 * 1000.0,
            },
            clusters,
        }
    }
}
