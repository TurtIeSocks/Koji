use std::time::Instant;

use model::api::{single_vec::SingleVec, sort_by::SortBy};

use self::sorting::{SortGeohash, SortLatLng, SortPointCount, SortRandom, SortS2};
use crate::{
    plugin::{Folder, Plugin},
    stats::Stats,
    utils::{get_plugin_list, rotate_to_best},
};

mod join;
pub mod sorting;
// pub mod vrp;

pub fn main(
    data_points: &SingleVec,
    clusters: SingleVec,
    sort_by: &SortBy,
    route_split_level: u64,
    radius: f64,
    stats: &mut Stats,
    routing_args: &str,
) -> SingleVec {
    let route_time = Instant::now();
    let clusters = match sort_by {
        SortBy::PointCount => clusters.sort_point_count(&data_points, radius),
        SortBy::LatLon => clusters.sort_lat_lng(),
        SortBy::GeoHash => clusters.sort_geohash(),
        SortBy::S2Cell => clusters.sort_s2(),
        SortBy::Random => clusters.sort_random(),
        SortBy::Unset => clusters,
        SortBy::Custom(plugin) => {
            let clusters = clusters.sort_s2();
            match Plugin::new(plugin, Folder::Routing, route_split_level, routing_args) {
                Ok(plugin_manager) => match plugin_manager.run(&clusters, Some(join::join)) {
                    Ok(sorted_clusters) => sorted_clusters,
                    Err(e) => {
                        log::error!("Error while running plugin: {}", e);
                        clusters
                    }
                },
                Err(e) => {
                    log::error!("Plugin not found: {}", e);
                    clusters
                }
            }
        }
    };
    let clusters = rotate_to_best(clusters, stats);

    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}

pub fn routing_plugins() -> Vec<String> {
    get_plugin_list("algorithms/src/routing/plugins").unwrap_or(vec![])
}
