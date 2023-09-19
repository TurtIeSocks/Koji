use super::*;

use geojson::FeatureCollection;
use model::{
    api::{
        args::{ClusterMode, SortBy},
        single_vec::SingleVec,
        stats::Stats,
        ToSingleVec,
    },
    db::GenericData,
};

mod balanced;
mod bruteforce;
mod fast;
mod rtree;

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
    match cluster_mode {
        ClusterMode::BruteForce => {
            bruteforce::multi_thread(&data_points, radius, min_points, cluster_split_level, stats)
        }
        ClusterMode::Balanced => {
            let mut clusters = vec![];
            for feature in area.into_iter() {
                let feature_clusters = balanced::cluster(
                    &data_points,
                    bootstrapping::as_vec(feature, radius, stats),
                    radius,
                    min_points,
                    stats,
                    only_unique,
                    &sort_by,
                );
                clusters.extend(feature_clusters);
            }
            clusters
        }
        ClusterMode::Fast => {
            let clusters = fast::cluster(data_points.to_single_vec(), radius, min_points, stats);
            clusters
        }
        ClusterMode::RTree => {
            let clusters = rtree::main(
                data_points.into_iter().map(|p| p.p).collect(),
                radius,
                min_points,
                cluster_split_level,
                stats,
            );
            clusters
        }
    }
}
