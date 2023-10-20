use std::time::Instant;

use crate::stats::Stats;

use super::*;

use geojson::FeatureCollection;
use model::{
    api::{
        args::{ClusterMode, SortBy},
        single_vec::SingleVec,
        ToSingleVec,
    },
    db::GenericData,
};

mod balanced;
mod bruteforce;
mod fast;
pub mod rtree;

pub fn main(
    data_points: Vec<GenericData>,
    cluster_mode: ClusterMode,
    radius: f64,
    min_points: usize,
    only_unique: bool,
    area: FeatureCollection,
    stats: &mut Stats,
    sort_by: SortBy,
    cluster_split_level: u64,
) -> SingleVec {
    stats.total_points = data_points.len();
    let time = Instant::now();
    let data_points = data_points.to_single_vec();

    let clusters = match cluster_mode {
        ClusterMode::BruteForce => {
            bruteforce::multi_thread(&data_points, radius, min_points, cluster_split_level)
        }
        ClusterMode::Balanced => {
            let mut clusters = vec![];
            for feature in area.into_iter() {
                let feature_clusters = balanced::cluster(
                    &data_points,
                    bootstrapping::as_vec(feature, radius, stats),
                    radius,
                    min_points,
                    only_unique,
                    &sort_by,
                );
                clusters.extend(feature_clusters);
            }
            clusters
        }
        ClusterMode::Fast => {
            let clusters = fast::cluster(&data_points, radius, min_points);
            clusters
        }
        ClusterMode::RTree => {
            let clusters = rtree::main(&data_points, radius, min_points, cluster_split_level);
            clusters
        }
    };
    stats.set_cluster_time(time);

    stats.cluster_stats(radius, &data_points, &clusters);
    stats.distance_stats(&clusters);
    stats.set_score(min_points);

    clusters
}
