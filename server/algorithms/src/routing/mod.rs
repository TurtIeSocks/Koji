use std::time::Instant;

use model::api::{args::SortBy, single_vec::SingleVec};

use crate::{stats::Stats, utils::rotate_to_best};

pub mod basic;
pub mod tsp;
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
    let clusters = if sort_by == &SortBy::TSP && !clusters.is_empty() {
        tsp::run(clusters, route_split_level)
    } else {
        basic::sort(&data_points, clusters, radius, sort_by)
    };
    let clusters = rotate_to_best(clusters, stats);

    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}
