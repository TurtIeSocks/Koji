use std::{time::Instant, vec};

use koji_plugins::{JoinFunction, PluginKind};

use crate::{plugins, stats::Stats};

use self::greedy::Greedy;

use super::*;

use geojson::FeatureCollection;
use koji_core::{CalculationMode, ClusterMode, ClusteringConfig, SingleVec};

mod auto;
mod candidates;
mod fastest;
// mod genetic;
mod greedy;
mod partition;
mod s2;

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
            // Quality modes all route to the mode-less `auto` algorithm; the
            // legacy greedy stays reachable for A/B via KOJI_LEGACY_GREEDY=1
            // (and still backs Honeycomb, which is a layout mode, not a
            // quality mode).
            ClusterMode::Balanced | ClusterMode::Fast | ClusterMode::Better | ClusterMode::Best
                if !legacy_greedy_requested() =>
            {
                log::info!(
                    "cluster_mode '{:?}' routes to the auto algorithm (modes are deprecated; set KOJI_LEGACY_GREEDY=1 for the legacy greedy)",
                    cfg.mode
                );
                let auto = auto::Auto {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: cfg.max_clusters,
                };
                auto.run(data_points)
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
            ClusterMode::Custom(plugin) => {
                match plugins::resolve(PluginKind::Clustering, &plugin, cfg.cluster_split_level) {
                    Some(plugin_manager) => {
                        match plugin_manager.run_multi::<JoinFunction>(
                            data_points,
                            &plugins::args_to_value(&cfg.plugin_args),
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

pub fn clustering_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Clustering)
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
