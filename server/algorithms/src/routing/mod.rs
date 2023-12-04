use std::time::Instant;

use model::api::{single_vec::SingleVec, sort_by::SortBy};

use crate::{
    stats::Stats,
    utils::{get_plugin_list, rotate_to_best},
};

use self::{
    plugin_manager::PluginManager,
    sorting::{SortGeohash, SortLatLng, SortPointCount, SortRandom, SortS2},
};

pub mod plugin_manager;
pub mod sorting;
// pub mod vrp;

pub fn main(
    data_points: &SingleVec,
    clusters: SingleVec,
    sort_by: &SortBy,
    route_split_level: u64,
    radius: f64,
    stats: &mut Stats,
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
            if let Ok(plugin_manager) =
                PluginManager::new(plugin, route_split_level, radius, &clusters)
            {
                if let Ok(sorted_clusters) = plugin_manager.run() {
                    sorted_clusters
                } else {
                    clusters
                }
            } else {
                clusters
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
