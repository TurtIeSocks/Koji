use std::time::Instant;

use geojson::{Feature, FeatureCollection};
use model::api::{calc_mode::CalculationMode, sort_by::SortBy, Precision, ToFeature};

use crate::{
    plugin::{Folder, Plugin},
    stats::Stats,
    utils,
};

pub mod radius;
pub mod s2;

pub fn main(
    area: FeatureCollection,
    calculation_mode: CalculationMode,
    radius: Precision,
    sort_by: SortBy,
    s2_level: u8,
    s2_size: u8,
    route_split_level: u64,
    stats: &mut Stats,
    routing_args: &str,
    bootstrap_args: &str,
) -> Vec<Feature> {
    let mut features = vec![];

    for feature in area.features {
        match &calculation_mode {
            CalculationMode::Radius => {
                let mut new_radius = radius::BootstrapRadius::new(&feature, radius);
                new_radius.sort(&sort_by, route_split_level, routing_args);

                *stats += &new_radius.stats;
                features.push(new_radius.feature());
            }
            CalculationMode::S2 => {
                let mut new_s2 = s2::BootstrapS2::new(&feature, s2_level as u64, s2_size);
                new_s2.sort(&sort_by, route_split_level, routing_args);

                *stats += &new_s2.stats;
                features.push(new_s2.feature());
            }
            CalculationMode::Custom(plugin) => {
                match Plugin::new(plugin, Folder::Bootstrap, 0, bootstrap_args) {
                    Ok(plugin_manager) => {
                        let time = Instant::now();
                        match plugin_manager.run(feature.to_string()) {
                            Ok(sorted_clusters) => {
                                let mut plugin_stats = Stats::new(plugin.to_string(), 0);
                                plugin_stats.set_cluster_time(time);
                                plugin_stats.cluster_stats(0., &vec![], &sorted_clusters);
                                features.push(sorted_clusters.to_feature(None));
                                *stats += &plugin_stats;
                            }
                            Err(e) => {
                                log::error!("Error while running plugin: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Plugin not found: {}", e);
                    }
                }
            }
        }
    }
    features
}

pub fn bootstrap_plugins() -> Vec<String> {
    utils::get_plugin_list("algorithms/src/bootstrap/plugins").unwrap_or(vec![])
}
