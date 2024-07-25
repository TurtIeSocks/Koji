mod circle;
mod sec;
mod state;
mod utils;

use std::time::Instant;

use model::api::{single_vec::SingleVec, Precision};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};

use crate::rtree::{self, SortDedupe};

pub fn with_data(radius: Precision, points: &SingleVec, clusters: &SingleVec) -> SingleVec {
    let time = Instant::now();
    log::info!("centering clusters on their points");
    let tree = rtree::spawn(radius, points);
    let clusters: Vec<rtree::point::Point> = clusters
        .into_iter()
        .map(|c| rtree::point::Point::new(radius, 20, *c))
        .collect();

    let mut clusters = rtree::cluster_info(&tree, &clusters);
    clusters.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));

    let mut seen_points = std::collections::HashSet::new();

    for c in clusters.iter_mut() {
        let mut unique_points = vec![];
        for p in c.all.iter() {
            if seen_points.contains(p) {
                continue;
            }
            seen_points.insert(p);
            unique_points.push(*p);
        }
        unique_points.sort_dedupe();
        c.unique = unique_points;
    }

    let centered: Vec<_> = clusters
        .par_iter()
        .map(|c| {
            (
                sec::multi_attempt(
                    c.unique
                        .iter()
                        .map(|p| geo::Point::new(p.center[1], p.center[0])),
                    radius,
                    100,
                ),
                c.point,
            )
        })
        .collect();

    let mut radius_too_big = 0;
    let mut points_outside = 0;
    let mut successes = 0;
    let mut circle_fails = 0;
    let mut final_clusters = vec![];

    for (result, fallback) in centered.into_iter() {
        match result {
            sec::SmallestEnclosingCircle::RadiusTooBig => {
                radius_too_big += 1;
                final_clusters.push(fallback.center);
            }
            sec::SmallestEnclosingCircle::MissingPoints => {
                points_outside += 1;
                final_clusters.push(fallback.center);
            }
            sec::SmallestEnclosingCircle::Centered(center) => {
                successes += 1;
                final_clusters.push([center.y(), center.x()]);
            }
            sec::SmallestEnclosingCircle::None => {
                circle_fails += 1;
                final_clusters.push(fallback.center);
            }
        }
    }
    log::info!("Success: {successes} | Radius Too Big: {radius_too_big} | Missing Points: {points_outside} | Circle Fails: {circle_fails}");
    log::info!("centered clusters in {:.2}s", time.elapsed().as_secs_f32());
    final_clusters
}
