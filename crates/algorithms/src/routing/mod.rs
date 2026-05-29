use std::time::Instant;

use koji_core::{RoutingConfig, SingleVec, SortBy};
use koji_plugins::PluginKind;

use self::sorting::{SortGeohash, SortLatLng, SortPointCount, SortRandom, SortS2};
use crate::{plugins, stats::Stats, utils};

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
        SortBy::PointCount => clusters.sort_point_count(&data_points, radius),
        SortBy::LatLon => clusters.sort_lat_lng(),
        SortBy::GeoHash => clusters.sort_geohash(),
        SortBy::S2Cell => clusters.sort_s2(),
        SortBy::Random => clusters.sort_random(),
        SortBy::Unset => clusters,
        SortBy::Custom(plugin) => {
            let clusters = clusters.sort_s2();
            match plugins::resolve(PluginKind::Routing, plugin, cfg.route_split_level) {
                Some(plugin_manager) => match plugin_manager.run_multi(
                    &clusters,
                    &plugins::args_to_value(&cfg.plugin_args),
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
    };
    let clusters = utils::rotate_to_best(clusters, stats);

    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}

pub fn routing_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Routing)
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
