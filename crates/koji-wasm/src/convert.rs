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
            // 0 means "unlimited" (matches the native convention in
            // model::api::args), otherwise the crucible algorithm's max-cluster
            // cap truncates the result to `.take(0)` → zero clusters.
            max_clusters: if self.max_clusters == 0 {
                usize::MAX
            } else {
                self.max_clusters
            },
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
                cluster_time_ms: stats.cluster_time * 1000.0,
            },
            clusters,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::ClusterRequest;

    fn make_request(cluster_mode: &str, calculation_mode: &str) -> ClusterRequest {
        ClusterRequest {
            points: vec![[35.0, 139.0]],
            radius: 100.0,
            min_points: 3,
            max_clusters: 10,
            cluster_mode: cluster_mode.to_string(),
            calculation_mode: calculation_mode.to_string(),
            center_clusters: true,
        }
    }

    #[test]
    fn valid_request_parses_into_core() {
        let req = make_request("fastest", "radius");
        let (points, cfg) = req.into_core().expect("should succeed");
        assert!(matches!(cfg.mode, ClusterMode::Fastest));
        assert!(matches!(cfg.calculation_mode, CalculationMode::Radius));
        assert_eq!(cfg.radius, 100.0);
        assert_eq!(cfg.min_points, 3);
        assert_eq!(cfg.max_clusters, 10);
        assert!(cfg.center_clusters);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0], [35.0, 139.0]);
    }

    #[test]
    fn unknown_calculation_mode_maps_to_custom() {
        // CalculationMode::Deserialize maps unknown strings to Custom(s), not an error.
        let req = make_request("balanced", "notamode");
        let (_points, cfg) = req
            .into_core()
            .expect("should succeed (no error for unknown mode)");
        assert!(matches!(cfg.calculation_mode, CalculationMode::Custom(_)));
        if let CalculationMode::Custom(s) = cfg.calculation_mode {
            assert_eq!(s, "notamode");
        }
    }

    #[test]
    fn unknown_cluster_mode_maps_to_custom() {
        // ClusterMode::Deserialize also maps unknown strings to Custom(s).
        let req = make_request("unknownmode", "s2");
        let (_points, cfg) = req.into_core().expect("should succeed");
        assert!(matches!(cfg.mode, ClusterMode::Custom(_)));
        assert!(matches!(cfg.calculation_mode, CalculationMode::S2));
    }

    #[test]
    fn defaults_applied_via_serde() {
        // Construct via serde_json to exercise #[serde(default)] fields.
        let req: ClusterRequest = serde_json::from_value(serde_json::json!({
            "points": [[1.0, 2.0]],
            "radius": 50.0,
            "min_points": 1,
            "max_clusters": 0
        }))
        .expect("deserialize");
        assert_eq!(req.cluster_mode, "balanced");
        assert_eq!(req.calculation_mode, "radius");
        assert!(!req.center_clusters);
        let (_points, cfg) = req.into_core().expect("into_core");
        assert!(matches!(cfg.mode, ClusterMode::Balanced));
        assert!(matches!(cfg.calculation_mode, CalculationMode::Radius));
        assert!(!cfg.center_clusters);
    }
}
