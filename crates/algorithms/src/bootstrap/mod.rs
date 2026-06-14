#[cfg(feature = "native")]
use std::time::Instant;

use geojson::{Feature, FeatureCollection};
#[cfg(feature = "native")]
use koji_core::{
    BootstrapConfig, CalculationMode, KojiGeometry, KojiGeometryCollection, RoutingConfig,
};
#[cfg(feature = "native")]
use koji_plugins::PluginKind;

#[cfg(feature = "native")]
use crate::{plugins, stats::Stats};

pub mod radius;
pub mod s2;

#[cfg(feature = "native")]
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
                // Koji-native inbound: geojson Feature -> KojiGeometry (Phase 1
                // `TryFrom`) -> one-item collection -> Phase 1B inherent
                // `to_single_vec` (parity-pinned against the dying matrix's
                // `ToSingleVec for Feature`). An unconvertible geometry yields no
                // points, matching the matrix's empty unsupported arm.
                let points = match KojiGeometry::try_from(feature.clone()) {
                    Ok(kg) => KojiGeometryCollection::new(vec![kg]).to_single_vec(),
                    Err(_) => vec![],
                };
                if let Some(sorted_clusters) =
                    plugins::run_once(PluginKind::Bootstrap, plugin, points, &cfg.plugin_args)
                {
                    let mut plugin_stats = Stats::new(plugin.to_string(), 0);
                    plugin_stats.set_cluster_time(time);
                    plugin_stats.cluster_stats(0., &vec![], &sorted_clusters);
                    // Koji-native Polygon projection (matches the old matrix
                    // default-context SingleVec projection); no To* matrix.
                    features.push(koji_core::single_vec_to_polygon_feature(&sorted_clusters));
                    *stats += &plugin_stats;
                }
            }
        }
    }
    features
}

#[cfg(feature = "native")]
pub fn bootstrap_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Bootstrap)
}

#[cfg(feature = "native")]
pub fn all_bootstrap_options() -> Vec<String> {
    let mut options = bootstrap_plugins();
    options.push("radius".to_string());
    options.push("s2".to_string());
    options
}
