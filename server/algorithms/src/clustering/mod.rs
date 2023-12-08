use std::{time::Instant, vec};

use crate::{
    plugin::{Folder, JoinFunction, Plugin},
    stats::Stats,
    utils,
};

use self::greedy::Greedy;

use super::*;

use geojson::FeatureCollection;
use model::api::{args::CalculationMode, cluster_mode::ClusterMode, single_vec::SingleVec};

mod fastest;
mod greedy;
mod s2;

pub fn main(
    data_points: &SingleVec,
    cluster_mode: ClusterMode,
    radius: f64,
    min_points: usize,
    stats: &mut Stats,
    cluster_split_level: u64,
    max_clusters: usize,
    calculation_mode: CalculationMode,
    s2_level: u8,
    s2_size: u8,
    collection: FeatureCollection,
    clustering_args: &str,
) -> SingleVec {
    if data_points.is_empty() {
        return vec![];
    }
    let time = Instant::now();
    let clusters = match calculation_mode {
        CalculationMode::S2 => collection
            .into_iter()
            .flat_map(|feature| s2::cluster(feature, data_points, s2_level, s2_size))
            .collect(),
        _ => match cluster_mode {
            ClusterMode::Fastest => {
                let clusters = fastest::main(&data_points, radius, min_points);
                clusters
            }
            ClusterMode::Balanced | ClusterMode::Fast | ClusterMode::Better | ClusterMode::Best => {
                let mut greedy = Greedy::default();
                greedy
                    .set_cluster_mode(cluster_mode)
                    .set_cluster_split_level(cluster_split_level)
                    .set_max_clusters(max_clusters)
                    .set_min_points(min_points)
                    .set_radius(radius);

                greedy.run(&data_points)
            }
            ClusterMode::Custom(plugin) => {
                match Plugin::new(
                    &plugin,
                    Folder::Clustering,
                    cluster_split_level,
                    clustering_args,
                ) {
                    Ok(plugin_manager) => {
                        match plugin_manager.run::<JoinFunction>(data_points, None) {
                            Ok(sorted_clusters) => sorted_clusters,
                            Err(e) => {
                                log::error!("Error while running plugin: {}", e);
                                vec![]
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Plugin not found: {}", e);
                        vec![]
                    }
                }
            }
        },
    };

    stats.set_cluster_time(time);
    stats.cluster_stats(radius, &data_points, &clusters);
    stats.set_score();

    clusters
}

pub fn clustering_plugins() -> Vec<String> {
    utils::get_plugin_list("algorithms/src/clustering/plugins").unwrap_or(vec![])
}
