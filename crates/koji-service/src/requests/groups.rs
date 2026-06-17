//! HTTP wire arg-groups. `Option`-field, camelCase wire structs — one group per
//! koji-core config struct. Each `resolve()` applies the legacy `init()` defaults
//! (spec §2 table) and produces the matching pure-domain config.
//!
//! koji-core configs stay serde-free; these wire structs carry the serde derives
//! and funnel through the shared [`super::resolve`] helpers so the default + plugin-
//! arg-build logic is a single source of truth across v1 and v2.

use algorithms::bootstrap::BootstrapConfig;
use algorithms::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
use algorithms::routing::{RoutingConfig, SortBy};
use koji_core::{Precision, SpawnpointTth};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::config::{DataFilter, DevConfig, OutputConfig, ReturnTypeArg, get_return_type};
use super::resolve::{
    DEFAULT_MIN_POINTS, DEFAULT_RADIUS, DEFAULT_S2_LEVEL, DEFAULT_S2_SIZE, bootstrap_plugin_args,
    clustering_plugin_args, resolve_max_clusters,
};

/// Clustering wire args → [`ClusteringConfig`].
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct ClusteringArgs {
    #[schema(value_type = Option<Precision>)]
    pub radius: Option<Precision>,
    pub min_points: Option<usize>,
    pub max_clusters: Option<usize>,
    #[schema(value_type = Option<String>)]
    pub mode: Option<ClusterMode>,
    #[schema(value_type = Option<String>)]
    pub calculation_mode: Option<CalculationMode>,
    pub s2_level: Option<u8>,
    pub s2_size: Option<u8>,
    pub center_clusters: Option<bool>,
    pub genetic_post_processing: Option<bool>,
    pub plugin_args: Option<String>,
}

impl ClusteringArgs {
    pub fn resolve(self) -> ClusteringConfig {
        let radius = self.radius.unwrap_or(DEFAULT_RADIUS);
        let min_points = self.min_points.unwrap_or(DEFAULT_MIN_POINTS);
        let max_clusters = resolve_max_clusters(self.max_clusters);
        ClusteringConfig {
            mode: self.mode.unwrap_or(ClusterMode::Balanced),
            radius,
            min_points,
            max_clusters,
            calculation_mode: self.calculation_mode.unwrap_or(CalculationMode::Radius),
            s2: S2Config {
                level: self.s2_level.unwrap_or(DEFAULT_S2_LEVEL),
                size: self.s2_size.unwrap_or(DEFAULT_S2_SIZE),
            },
            center_clusters: self.center_clusters.unwrap_or(false),
            genetic_post_processing: self.genetic_post_processing.unwrap_or(false),
            plugin_args: clustering_plugin_args(self.plugin_args, radius, min_points, max_clusters),
        }
    }
}

/// Routing wire args → [`RoutingConfig`].
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct RoutingArgs {
    #[schema(value_type = Option<String>)]
    pub sort_by: Option<SortBy>,
    pub plugin_args: Option<String>,
}

impl RoutingArgs {
    pub fn resolve(self) -> RoutingConfig {
        RoutingConfig {
            sort_by: self.sort_by.unwrap_or(SortBy::Unset),
            plugin_args: self.plugin_args.unwrap_or_default(),
        }
    }
}

/// Bootstrap wire args → [`BootstrapConfig`].
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct BootstrapArgs {
    #[schema(value_type = Option<String>)]
    pub calculation_mode: Option<CalculationMode>,
    #[schema(value_type = Option<Precision>)]
    pub radius: Option<Precision>,
    pub s2_level: Option<u8>,
    pub s2_size: Option<u8>,
    pub plugin_args: Option<String>,
}

impl BootstrapArgs {
    pub fn resolve(self) -> BootstrapConfig {
        let radius = self.radius.unwrap_or(DEFAULT_RADIUS);
        BootstrapConfig {
            calculation_mode: self.calculation_mode.unwrap_or(CalculationMode::Radius),
            radius,
            s2: S2Config {
                level: self.s2_level.unwrap_or(DEFAULT_S2_LEVEL),
                size: self.s2_size.unwrap_or(DEFAULT_S2_SIZE),
            },
            plugin_args: bootstrap_plugin_args(self.plugin_args, radius),
        }
    }
}

/// Data-filter wire args → [`DataFilter`].
#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct DataFilterArgs {
    pub last_seen: Option<u32>,
    #[schema(value_type = Option<String>)]
    pub tth: Option<SpawnpointTth>,
}

impl DataFilterArgs {
    pub fn resolve(self) -> DataFilter {
        DataFilter {
            last_seen: self.last_seen.unwrap_or(0),
            tth: self.tth.unwrap_or(SpawnpointTth::All),
        }
    }
}

/// Output wire args → [`OutputConfig`]. `resolve` takes the per-op
/// `default_return_type` (derived from the inbound `area` container shape — see
/// the per-op request layer); when `return_type` is `Some`, the wire string is
/// parsed against that default via [`get_return_type`], else the default stands.
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct OutputArgs {
    pub return_type: Option<String>,
    pub save_to_db: Option<bool>,
    pub save_to_golbat: Option<bool>,
    pub save_to_golbat_only: Option<bool>,
    pub simplify: Option<bool>,
}

impl OutputArgs {
    pub fn resolve(self, default_return_type: ReturnTypeArg) -> OutputConfig {
        let return_type = if let Some(return_type) = self.return_type {
            get_return_type(return_type, &default_return_type)
        } else {
            default_return_type
        };
        OutputConfig {
            return_type,
            save_to_db: self.save_to_db.unwrap_or(false),
            save_to_golbat: self.save_to_golbat.unwrap_or(false),
            save_to_golbat_only: self.save_to_golbat_only.unwrap_or(false),
            simplify: self.simplify.unwrap_or(false),
        }
    }
}

/// Developer / experimental wire toggles → [`DevConfig`].
#[derive(Debug, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase", default)]
pub struct DevArgs {
    pub benchmark_mode: Option<bool>,
}

impl DevArgs {
    pub fn resolve(self) -> DevConfig {
        DevConfig {
            benchmark_mode: self.benchmark_mode.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ClusteringArgs ───────────────────────────────────────────────────────

    #[test]
    fn clustering_args_resolve_matches_legacy_defaults() {
        // All-None wire -> the documented defaults + the plugin-arg tail.
        let cfg = ClusteringArgs::default().resolve();
        assert_eq!(cfg.radius, 70.0);
        assert_eq!(cfg.min_points, 1);
        assert_eq!(cfg.max_clusters, usize::MAX);
        assert_eq!(cfg.mode, ClusterMode::Balanced);
        assert_eq!(cfg.calculation_mode, CalculationMode::Radius);
        assert_eq!(cfg.s2.level, 15);
        assert_eq!(cfg.s2.size, 9);
        assert_eq!(
            cfg.plugin_args,
            " --radius 70 --min_points 1 --max_clusters 18446744073709551615"
        );
    }

    #[test]
    fn clustering_args_wire_deserializes_camel_case() {
        let g: ClusteringArgs = serde_json::from_str(r#"{"radius":50,"minPoints":3}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.radius, 50.0);
        assert_eq!(cfg.min_points, 3);
    }

    #[test]
    fn clustering_args_max_clusters_zero_becomes_usize_max() {
        let g: ClusteringArgs = serde_json::from_str(r#"{"maxClusters":0}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.max_clusters, usize::MAX);
    }

    #[test]
    fn clustering_args_explicit_max_clusters_passes_through() {
        let g: ClusteringArgs = serde_json::from_str(r#"{"maxClusters":7}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.max_clusters, 7);
    }

    #[test]
    fn clustering_args_s2_overrides() {
        let g: ClusteringArgs = serde_json::from_str(r#"{"s2Level":12,"s2Size":5}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.s2.level, 12);
        assert_eq!(cfg.s2.size, 5);
    }

    #[test]
    fn clustering_args_center_clusters_and_genetic_default_false() {
        let cfg = ClusteringArgs::default().resolve();
        assert!(!cfg.center_clusters);
        assert!(!cfg.genetic_post_processing);
    }

    #[test]
    fn clustering_args_center_clusters_explicit_true() {
        let g: ClusteringArgs =
            serde_json::from_str(r#"{"centerClusters":true,"geneticPostProcessing":true}"#)
                .unwrap();
        let cfg = g.resolve();
        assert!(cfg.center_clusters);
        assert!(cfg.genetic_post_processing);
    }

    #[test]
    fn clustering_args_plugin_args_prepended() {
        let g: ClusteringArgs =
            serde_json::from_str(r#"{"pluginArgs":"--custom","radius":40,"minPoints":2}"#).unwrap();
        let cfg = g.resolve();
        assert!(cfg.plugin_args.starts_with("--custom"));
        assert!(cfg.plugin_args.contains("--radius 40"));
        assert!(cfg.plugin_args.contains("--min_points 2"));
    }

    // ── RoutingArgs ──────────────────────────────────────────────────────────

    #[test]
    fn routing_args_defaults_to_unset_sort_and_empty_plugin_args() {
        let cfg = RoutingArgs::default().resolve();
        assert_eq!(cfg.sort_by, SortBy::Unset);
        assert_eq!(cfg.plugin_args, "");
    }

    #[test]
    fn routing_args_plugin_args_set() {
        let g: RoutingArgs = serde_json::from_str(r#"{"pluginArgs":"--foo"}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.plugin_args, "--foo");
    }

    // ── BootstrapArgs ────────────────────────────────────────────────────────

    #[test]
    fn bootstrap_args_defaults() {
        let cfg = BootstrapArgs::default().resolve();
        assert_eq!(cfg.radius, 70.0);
        assert_eq!(cfg.s2.level, 15);
        assert_eq!(cfg.s2.size, 9);
        assert_eq!(cfg.calculation_mode, CalculationMode::Radius);
        assert!(cfg.plugin_args.contains("--radius 70"));
    }

    #[test]
    fn bootstrap_args_custom_radius_feeds_plugin_args() {
        let g: BootstrapArgs = serde_json::from_str(r#"{"radius":50}"#).unwrap();
        let cfg = g.resolve();
        assert_eq!(cfg.radius, 50.0);
        assert!(cfg.plugin_args.contains("--radius 50"));
    }

    #[test]
    fn bootstrap_args_plugin_args_prepended() {
        let g: BootstrapArgs = serde_json::from_str(r#"{"pluginArgs":"--extra"}"#).unwrap();
        let cfg = g.resolve();
        assert!(cfg.plugin_args.starts_with("--extra"));
    }

    // ── DataFilterArgs ───────────────────────────────────────────────────────

    #[test]
    fn data_filter_defaults_last_seen_zero_and_tth_all() {
        let df = DataFilterArgs::default().resolve();
        assert_eq!(df.last_seen, 0);
        assert!(matches!(df.tth, SpawnpointTth::All));
    }

    #[test]
    fn data_filter_last_seen_set() {
        let g: DataFilterArgs = serde_json::from_str(r#"{"lastSeen":3600}"#).unwrap();
        let df = g.resolve();
        assert_eq!(df.last_seen, 3600);
    }

    #[test]
    fn data_filter_tth_known() {
        let g: DataFilterArgs = serde_json::from_str(r#"{"tth":"Known"}"#).unwrap();
        let df = g.resolve();
        assert!(matches!(df.tth, SpawnpointTth::Known));
    }

    #[test]
    fn data_filter_tth_unknown() {
        let g: DataFilterArgs = serde_json::from_str(r#"{"tth":"Unknown"}"#).unwrap();
        let df = g.resolve();
        assert!(matches!(df.tth, SpawnpointTth::Unknown));
    }

    // ── OutputArgs ───────────────────────────────────────────────────────────

    #[test]
    fn output_args_defaults_all_false_and_uses_provided_default_return_type() {
        use crate::requests::config::ReturnTypeArg;
        let cfg = OutputArgs::default().resolve(ReturnTypeArg::Feature);
        assert_eq!(cfg.return_type, ReturnTypeArg::Feature);
        assert!(!cfg.save_to_db);
        assert!(!cfg.save_to_golbat);
        assert!(!cfg.save_to_golbat_only);
        assert!(!cfg.simplify);
    }

    #[test]
    fn output_args_explicit_return_type_overrides_default() {
        use crate::requests::config::ReturnTypeArg;
        let g: OutputArgs = serde_json::from_str(r#"{"returnType":"featureCollection"}"#).unwrap();
        let cfg = g.resolve(ReturnTypeArg::Feature);
        assert_eq!(cfg.return_type, ReturnTypeArg::FeatureCollection);
    }

    #[test]
    fn output_args_unknown_return_type_falls_back_to_default() {
        use crate::requests::config::ReturnTypeArg;
        let g: OutputArgs = serde_json::from_str(r#"{"returnType":"nonsense"}"#).unwrap();
        let cfg = g.resolve(ReturnTypeArg::SingleArray);
        assert_eq!(cfg.return_type, ReturnTypeArg::SingleArray);
    }

    #[test]
    fn output_args_save_flags_set() {
        let g: OutputArgs = serde_json::from_str(
            r#"{"saveToDb":true,"saveToGolbat":true,"saveToGolbatOnly":true,"simplify":true}"#,
        )
        .unwrap();
        use crate::requests::config::ReturnTypeArg;
        let cfg = g.resolve(ReturnTypeArg::SingleArray);
        assert!(cfg.save_to_db);
        assert!(cfg.save_to_golbat);
        assert!(cfg.save_to_golbat_only);
        assert!(cfg.simplify);
    }

    // ── DevArgs ──────────────────────────────────────────────────────────────

    #[test]
    fn dev_args_defaults_false() {
        let cfg = DevArgs::default().resolve();
        assert!(!cfg.benchmark_mode);
    }

    #[test]
    fn dev_args_benchmark_mode_true() {
        let g: DevArgs = serde_json::from_str(r#"{"benchmarkMode":true}"#).unwrap();
        let cfg = g.resolve();
        assert!(cfg.benchmark_mode);
    }
}
