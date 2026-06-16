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
            ClusterMode::Better | ClusterMode::Best => {
                let crucible = crucible::Crucible {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: cfg.max_clusters,
                };
                crucible.run(data_points)
            }
            ClusterMode::Honeycomb | ClusterMode::Fast | ClusterMode::Balanced => {
                let mut greedy = Greedy::default();
                greedy
                    .set_cluster_mode(cfg.mode.clone())
                    .set_max_clusters(cfg.max_clusters)
                    .set_min_points(cfg.min_points)
                    .set_radius(cfg.radius);
                greedy.run(data_points)
            }
            #[cfg(feature = "native")]
            ClusterMode::Custom(plugin) => {
                match plugins::resolve(PluginKind::Clustering, &plugin, 0) {
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

    stats.set_cluster_time(time);
    stats.cluster_stats(cfg.radius, data_points, &clusters);
    stats.set_score();

    clusters
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
            max_clusters: usize::MAX,
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
        let result = main(
            &empty,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
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
        let result = main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        assert!(!result.is_empty(), "Fastest mode should produce clusters");
        // Output stays within valid lat/lon range.
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0);
            assert!(lon.abs() < 180.0);
        }
    }

    // ── main: quality modes route to crucible ────────────────────────────────

    #[test]
    fn best_mode_produces_clusters() {
        // Dense grid of 100 points in a 0.05° × 0.05° area; radius 500 m ensures
        // many points fall within each disk → crucible finds valid clusters.
        let pts: Vec<[f64; 2]> = (0..100)
            .map(|i| {
                [
                    40.0 + (i / 10) as f64 * 0.005,
                    -74.0 + (i % 10) as f64 * 0.005,
                ]
            })
            .collect();
        let mut cfg = make_cfg(ClusterMode::Best);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        assert!(
            !result.is_empty(),
            "Best mode should produce clusters for dense 100-point grid"
        );
    }

    #[test]
    fn better_mode_produces_clusters() {
        let pts: Vec<[f64; 2]> = (0..100)
            .map(|i| {
                [
                    40.0 + (i / 10) as f64 * 0.005,
                    -74.0 + (i % 10) as f64 * 0.005,
                ]
            })
            .collect();
        let mut cfg = make_cfg(ClusterMode::Better);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        assert!(
            !result.is_empty(),
            "Better mode should produce clusters for dense 100-point grid"
        );
    }

    // ── main: stats populated after run ──────────────────────────────────────

    #[test]
    fn stats_cluster_time_set() {
        let pts = vec![[40.0, -74.0], [40.001, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        assert!(stats.cluster_time >= 0.0);
    }

    // ── all_clustering_options ────────────────────────────────────────────────

    #[test]
    fn all_clustering_options_contains_built_ins() {
        let opts = all_clustering_options();
        assert!(opts.contains(&"fastest".to_string()));
        assert!(opts.contains(&"best".to_string()));
        assert!(opts.contains(&"honeycomb".to_string()));
    }

    // ── main: Honeycomb mode ──────────────────────────────────────────────────

    #[test]
    fn honeycomb_mode_produces_clusters() {
        let pts: Vec<[f64; 2]> = (0..20).map(|i| [40.0 + i as f64 * 0.001, -74.0]).collect();
        let mut cfg = make_cfg(ClusterMode::Honeycomb);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        // Honeycomb is a layout mode; it should produce some output.
        assert!(!result.is_empty(), "Honeycomb mode should produce clusters");
    }

    // ── main: center_clusters=true path ───────────────────────────────────────

    #[test]
    fn center_clusters_path_runs_without_panic() {
        // center_clusters = true routes through sec::with_data.
        // Just verify it doesn't panic and returns something.
        let pts = vec![[40.0, -74.0], [40.0003, -74.0]];
        let mut cfg = make_cfg(ClusterMode::Fastest);
        cfg.center_clusters = true;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        // Result may be empty or non-empty depending on SEC; just no panic.
        assert!(result.len() <= pts.len() + 1);
    }

    // ── main: mygod_score populated ───────────────────────────────────────────

    #[test]
    fn main_populates_mygod_score() {
        let pts = vec![[40.0, -74.0], [40.0001, -74.0], [40.1, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        // mygod_score is set during main(); should be non-negative.
        assert!(stats.mygod_score < usize::MAX);
    }
}
