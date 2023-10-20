use std::time::Instant;

use crate::stats::Stats;

use self::rtree::coverage;

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
    let time = Instant::now();
    stats.total_points = data_points.len();

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
    stats.cluster_time = time.elapsed().as_secs_f64();

    let mut cluster_checker = coverage::ClusterStats::new(&data_points, radius);
    cluster_checker.set_clusters(&clusters);
    let (best, best_clusters) = cluster_checker.get_best_clusters();

    stats.total_clusters = clusters.len();
    stats.points_covered = cluster_checker.check_full_coverage();
    stats.best_cluster_point_count = best;
    stats.best_clusters = best_clusters;
    stats.set_score(min_points);
    stats.distance(&clusters);

    clusters
}
