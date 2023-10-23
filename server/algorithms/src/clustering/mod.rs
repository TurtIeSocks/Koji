use std::time::Instant;

use crate::stats::Stats;

use self::greedy::Greedy;

use super::*;

use model::api::{cluster_mode::ClusterMode, single_vec::SingleVec};

mod fastest;
mod greedy;

pub fn main(
    data_points: &SingleVec,
    cluster_mode: ClusterMode,
    radius: f64,
    min_points: usize,
    stats: &mut Stats,
    cluster_split_level: u64,
    max_clusters: usize,
) -> SingleVec {
    let time = Instant::now();
    let clusters = match cluster_mode {
        ClusterMode::Fastest => {
            let clusters = fastest::main(&data_points, radius, min_points);
            clusters
        }
        _ => {
            let mut greedy = Greedy::default();
            greedy
                .set_cluster_mode(cluster_mode)
                .set_cluster_split_level(cluster_split_level)
                .set_max_clusters(max_clusters)
                .set_min_points(min_points)
                .set_radius(radius);

            greedy.run(&data_points)
        }
    };

    stats.set_cluster_time(time);
    stats.cluster_stats(radius, &data_points, &clusters);
    stats.set_score(min_points);

    clusters
}
