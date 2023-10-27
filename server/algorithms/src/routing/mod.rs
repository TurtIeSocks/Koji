use std::{collections::VecDeque, time::Instant};

use model::api::{args::SortBy, point_array::PointArray, single_vec::SingleVec};

use crate::stats::Stats;

pub mod basic;
pub mod tsp;
// pub mod vrp;

pub fn main(
    data_points: &SingleVec,
    clusters: SingleVec,
    sort_by: SortBy,
    route_split_level: u64,
    radius: f64,
    stats: &mut Stats,
) -> SingleVec {
    let route_time = Instant::now();
    let clusters = if sort_by == SortBy::TSP && !clusters.is_empty() {
        let tour = tsp::multi(&clusters, route_split_level);
        let mut final_clusters = VecDeque::<PointArray>::new();

        let mut rotate_count = 0;
        for (i, [lat, lon]) in tour.into_iter().enumerate() {
            if stats.best_clusters.len() > 0
                && lat == stats.best_clusters[0][0]
                && lon == stats.best_clusters[0][1]
            {
                rotate_count = i;
                log::debug!("Found Best! {}, {} - {}", lat, lon, i);
            }
            final_clusters.push_back([lat, lon]);
        }
        final_clusters.rotate_left(rotate_count);

        final_clusters.into()
    } else {
        basic::sort(&data_points, clusters, radius, sort_by)
    };
    stats.set_route_time(route_time);
    stats.distance_stats(&clusters);

    clusters
}
