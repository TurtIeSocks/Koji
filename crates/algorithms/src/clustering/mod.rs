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
// mod genetic;
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
