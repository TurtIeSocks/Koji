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
    max_clusters: usize,
) -> SingleVec {
    stats.total_points = data_points.len();
    let time = Instant::now();
    let data_points = data_points.to_single_vec();

    let clusters = match cluster_mode {
        ClusterMode::Fastest => {
            let clusters = fast::cluster(&data_points, radius, min_points);
            clusters
        }
        _ => {
            let clusters = rtree::main(
                &data_points,
                radius,
                min_points,
                cluster_split_level,
                max_clusters,
            );
            clusters
        }
    };
    stats.set_cluster_time(time);

    stats.cluster_stats(radius, &data_points, &clusters);
    stats.set_score(min_points);

    clusters
}
