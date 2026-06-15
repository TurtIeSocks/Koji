use std::vec;
use web_time::Instant;

#[cfg(feature = "native")]
use koji_plugins::{JoinFunction, PluginKind};

#[cfg(feature = "native")]
use crate::plugins;
use crate::stats::Stats;

use self::greedy::Greedy;

use super::*;

use geojson::FeatureCollection;
use koji_core::SingleVec;

mod calc_mode;
mod candidates;
mod cluster_mode;
mod config;
mod crucible;
mod fastest;
mod greedy;
mod partition;
mod s2;

pub use calc_mode::CalculationMode;
pub use cluster_mode::ClusterMode;
pub use config::{ClusteringConfig, S2Config};

/// Warm-start entry point for incremental re-clustering (`Crucible::run_seeded`).
pub use crucible::Crucible;

pub fn main(
    data_points: &SingleVec,
    cfg: &ClusteringConfig,
    collection: FeatureCollection,
    bypass_adaptive_partition: bool,
    stats: &mut Stats,
) -> SingleVec {
    if data_points.is_empty() {
        return vec![];
    }
    let time = Instant::now();
    let clusters = match cfg.calculation_mode {
        CalculationMode::S2 => collection
            .into_iter()
            .flat_map(|feature| {
                s2::cluster(
                    feature,
                    data_points,
                    cfg.s2.level,
                    cfg.s2.size,
                    cfg.min_points,
                )
            })
            .collect(),
        _ => match cfg.mode.clone() {
            ClusterMode::Fastest => fastest::main(data_points, cfg.radius, cfg.min_points),
            // Quality modes all route to the mode-less `crucible` algorithm; the
            // legacy greedy stays reachable for A/B via KOJI_LEGACY_GREEDY=1
            // (and still backs Honeycomb, which is a layout mode, not a
            // quality mode).
            ClusterMode::Balanced | ClusterMode::Fast | ClusterMode::Better | ClusterMode::Best
                if !legacy_greedy_requested() =>
            {
                log::info!(
                    "cluster_mode '{:?}' routes to the crucible algorithm (modes are deprecated; set KOJI_LEGACY_GREEDY=1 for the legacy greedy)",
                    cfg.mode
                );
                let crucible = crucible::Crucible {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: cfg.max_clusters,
                };
                crucible.run(data_points)
            }
            ClusterMode::Honeycomb
            | ClusterMode::Balanced
            | ClusterMode::Fast
            | ClusterMode::Better
            | ClusterMode::Best => {
                let mut greedy = Greedy::default();
                greedy
                    .set_cluster_mode(cfg.mode.clone())
                    .set_cluster_split_level(cfg.cluster_split_level)
                    .set_max_clusters(cfg.max_clusters)
                    .set_min_points(cfg.min_points)
                    .set_radius(cfg.radius)
                    .set_bypass_adaptive_partition(bypass_adaptive_partition);

                greedy.run(data_points)
            }
            #[cfg(feature = "native")]
            ClusterMode::Custom(plugin) => {
                match plugins::resolve(PluginKind::Clustering, &plugin, cfg.cluster_split_level) {
                    Some(plugin_manager) => {
                        match plugin_manager.run_multi::<JoinFunction>(
                            data_points,
                            &plugins::merged_args(
                                PluginKind::Clustering,
                                &plugin,
                                &cfg.plugin_args,
                            ),
                            None,
                        ) {
                            Ok(sorted_clusters) => sorted_clusters,
                            Err(e) => {
                                log::error!("Error while running plugin: {}", e);
                                vec![]
                            }
                        }
                    }
                    None => vec![],
                }
            }
            #[cfg(not(feature = "native"))]
            ClusterMode::Custom(_plugin) => {
                log::warn!(
                    "custom clustering plugins are unavailable in wasm; using the crucible algorithm"
                );
                let crucible = crucible::Crucible {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: cfg.max_clusters,
                };
                crucible.run(data_points)
            }
        },
    };
    let clusters = if cfg.center_clusters {
        sec::with_data(cfg.radius, data_points, &clusters)
    } else {
        clusters
    };
    // let clusters = if genetic_post_processing {
    //     let optimizer = genetic::GeneticClusterOptimizer::new(
    //         data_points.clone(),
    //         min_points,
    //         max_clusters,
    //         radius,
    //     );
    //     optimizer.optimize(clusters)
    // } else {
    //     clusters
    // };

    stats.set_cluster_time(time);
    stats.cluster_stats(cfg.radius, data_points, &clusters);
    stats.set_score();

    clusters
}

/// Dev escape hatch: route quality modes back to the legacy greedy for A/B.
fn legacy_greedy_requested() -> bool {
    std::env::var("KOJI_LEGACY_GREEDY").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

#[cfg(feature = "native")]
pub fn clustering_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Clustering)
}

#[cfg(not(feature = "native"))]
pub fn clustering_plugins() -> Vec<String> {
    vec![]
}

pub fn all_clustering_options() -> Vec<String> {
    let mut options = clustering_plugins();
    options.push("honeycomb".to_string());
    options.push("fastest".to_string());
    options.push("balanced".to_string());
    options.push("fast".to_string());
    options.push("better".to_string());
    options.push("best".to_string());
    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
    use crate::stats::Stats;
    use geojson::FeatureCollection;

    fn make_cfg(mode: ClusterMode) -> ClusteringConfig {
        ClusteringConfig {
            mode,
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX, // 0 truncates all clusters
            cluster_split_level: 0,
            calculation_mode: CalculationMode::Radius,
            s2: S2Config::default(),
            center_clusters: false,
            genetic_post_processing: false,
            plugin_args: String::new(),
        }
    }

    fn empty_collection() -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        }
    }

    // ── main: empty input always returns empty ─────────────────────────────────

    #[test]
    fn empty_data_returns_empty() {
        let empty: SingleVec = vec![];
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&empty, &make_cfg(ClusterMode::Fastest), empty_collection(), false, &mut stats);
        assert!(result.is_empty());
    }

    // ── main: Fastest mode with valid data produces non-empty output ───────────

    #[test]
    fn fastest_mode_basic_data() {
        let pts = vec![
            [40.0_f64, -74.0_f64],
            [40.0001, -74.0001],
            [40.0002, -74.0002],
        ];
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &make_cfg(ClusterMode::Fastest), empty_collection(), false, &mut stats);
        assert!(!result.is_empty(), "Fastest mode should produce clusters");
        // Output stays within valid lat/lon range.
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0);
            assert!(lon.abs() < 180.0);
        }
    }

    // ── main: quality modes (no legacy greedy env) route to crucible ──────────

    #[test]
    fn best_mode_produces_clusters() {
        unsafe { std::env::remove_var("KOJI_LEGACY_GREEDY") };
        // Dense grid of 100 points in a 0.05° × 0.05° area; radius 500 m ensures
        // many points fall within each disk → crucible finds valid clusters.
        let pts: Vec<[f64; 2]> = (0..100)
            .map(|i| [40.0 + (i / 10) as f64 * 0.005, -74.0 + (i % 10) as f64 * 0.005])
            .collect();
        let mut cfg = make_cfg(ClusterMode::Best);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), false, &mut stats);
        assert!(!result.is_empty(), "Best mode should produce clusters for dense 100-point grid");
    }

    #[test]
    fn better_mode_produces_clusters() {
        unsafe { std::env::remove_var("KOJI_LEGACY_GREEDY") };
        let pts: Vec<[f64; 2]> = (0..100)
            .map(|i| [40.0 + (i / 10) as f64 * 0.005, -74.0 + (i % 10) as f64 * 0.005])
            .collect();
        let mut cfg = make_cfg(ClusterMode::Better);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), false, &mut stats);
        assert!(!result.is_empty(), "Better mode should produce clusters for dense 100-point grid");
    }

    // ── main: stats populated after run ──────────────────────────────────────

    #[test]
    fn stats_cluster_time_set() {
        let pts = vec![[40.0, -74.0], [40.001, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        main(&pts, &make_cfg(ClusterMode::Fastest), empty_collection(), false, &mut stats);
        assert!(stats.cluster_time >= 0.0);
    }

    // ── legacy_greedy_requested ───────────────────────────────────────────────

    #[test]
    fn legacy_greedy_off_by_default() {
        unsafe { std::env::remove_var("KOJI_LEGACY_GREEDY") };
        assert!(!legacy_greedy_requested());
    }

    #[test]
    fn legacy_greedy_enabled_by_env() {
        unsafe { std::env::set_var("KOJI_LEGACY_GREEDY", "1") };
        assert!(legacy_greedy_requested());
        unsafe { std::env::remove_var("KOJI_LEGACY_GREEDY") };
    }

    #[test]
    fn legacy_greedy_enabled_by_true() {
        unsafe { std::env::set_var("KOJI_LEGACY_GREEDY", "true") };
        assert!(legacy_greedy_requested());
        unsafe { std::env::remove_var("KOJI_LEGACY_GREEDY") };
    }

    // ── all_clustering_options ────────────────────────────────────────────────

    #[test]
    fn all_clustering_options_contains_built_ins() {
        let opts = all_clustering_options();
        assert!(opts.contains(&"fastest".to_string()));
        assert!(opts.contains(&"best".to_string()));
        assert!(opts.contains(&"honeycomb".to_string()));
    }
}

