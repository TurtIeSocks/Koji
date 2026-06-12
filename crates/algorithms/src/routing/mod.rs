use web_time::Instant;

use koji_core::{RoutingConfig, SingleVec, SortBy};
#[cfg(feature = "native")]
use koji_plugins::PluginKind;

use self::sorting::{SortGeohash, SortLatLng, SortPointCount, SortRandom, SortS2};
#[cfg(feature = "native")]
use crate::plugins;
use crate::{stats::Stats, utils};

#[cfg(feature = "native")]
mod join;
pub mod sorting;
// pub mod vrp;

pub fn main(
    data_points: &SingleVec,
    clusters: SingleVec,
    radius: f64,
    cfg: &RoutingConfig,
    stats: &mut Stats,
) -> SingleVec {
    let route_time = Instant::now();
    let clusters = match &cfg.sort_by {
        SortBy::PointCount => clusters.sort_point_count(data_points, radius),
        SortBy::LatLon => clusters.sort_lat_lng(),
        SortBy::GeoHash => clusters.sort_geohash(),
        SortBy::S2Cell => clusters.sort_s2(),
        SortBy::Random => clusters.sort_random(),
        SortBy::Unset => clusters,
        #[cfg(feature = "native")]
        SortBy::Custom(plugin) => {
            let clusters = clusters.sort_s2();
            match plugins::resolve(PluginKind::Routing, plugin, cfg.route_split_level) {
                Some(plugin_manager) => match plugin_manager.run_multi(
                    &clusters,
                    &plugins::merged_args(PluginKind::Routing, plugin, &cfg.plugin_args),
                    Some(join::join),
                ) {
                    Ok(sorted_clusters) => sorted_clusters,
                    Err(e) => {
                        log::error!("Error while running plugin: {}", e);
                        clusters
                    }
                },
                None => clusters,
            }
        }
        #[cfg(not(feature = "native"))]
        SortBy::Custom(_plugin) => {
            log::warn!("custom routing plugins unavailable in wasm; using S2 sort");
            clusters.sort_s2()
        }
    };
    let clusters = utils::rotate_to_best(clusters, stats);

    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}

#[cfg(feature = "native")]
pub fn routing_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Routing)
}

#[cfg(not(feature = "native"))]
pub fn routing_plugins() -> Vec<String> {
    vec![]
}

pub fn all_routing_options() -> Vec<String> {
    let mut options = routing_plugins();
    options.push("point_count".to_string());
    options.push("latlon".to_string());
    options.push("geohash".to_string());
    options.push("s2".to_string());
    options.push("random".to_string());
    options
}
