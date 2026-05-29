use std::time::Instant;

use geojson::{Feature, FeatureCollection};
use koji_core::{
    BootstrapConfig, CalculationMode, FeatureCtx, RoutingConfig, ToFeature, ToSingleVec,
};
use koji_plugins::PluginKind;

use crate::{plugins, stats::Stats};

pub mod radius;
pub mod s2;

pub fn main(
    area: FeatureCollection,
    cfg: &BootstrapConfig,
    routing: &RoutingConfig,
    stats: &mut Stats,
) -> Vec<Feature> {
    let mut features = vec![];

    for feature in area.features {
        match &cfg.calculation_mode {
            CalculationMode::Radius => {
                let mut new_radius = radius::BootstrapRadius::new(&feature, cfg.radius);
                new_radius.sort(routing);

                *stats += &new_radius.stats;
                features.push(new_radius.feature());
            }
            CalculationMode::S2 => {
                let mut new_s2 = s2::BootstrapS2::new(&feature, cfg.s2.level, cfg.s2.size);
                new_s2.sort(routing);

                *stats += &new_s2.stats;
                features.push(new_s2.feature());
            }
            CalculationMode::Custom(plugin) => {
                let time = Instant::now();
                let points = feature.clone().to_single_vec();
                if let Some(sorted_clusters) = plugins::run_once(PluginKind::Bootstrap, plugin, points, &cfg.plugin_args) {
                    let mut plugin_stats = Stats::new(plugin.to_string(), 0);
                    plugin_stats.set_cluster_time(time);
                    plugin_stats.cluster_stats(0., &vec![], &sorted_clusters);
                    features.push(sorted_clusters.to_feature(&FeatureCtx::default()));
                    *stats += &plugin_stats;
                }
            }
        }
    }
    features
}

pub fn bootstrap_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Bootstrap)
}

pub fn all_bootstrap_options() -> Vec<String> {
    let mut options = bootstrap_plugins();
    options.push("radius".to_string());
    options.push("s2".to_string());
    options
}
