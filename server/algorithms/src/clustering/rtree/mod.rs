mod point;

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use point::Point;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

use crate::s2::create_cell_map;

pub fn main(
    points: SingleVec,
    radius: f64,
    min_points: usize,
    cluster_split_level: u64,
    stats: &mut Stats,
) -> SingleVec {
    let time = Instant::now();

    stats.total_points = points.len();

    let mut return_set = HashSet::new();
    let mut missing_count = 0;
    let cell_maps = create_cell_map(&points, cluster_split_level);

    let mut handlers = vec![];
    for (key, values) in cell_maps.into_iter() {
        log::debug!("Total {}: {}", key, values.len());
        handlers.push(std::thread::spawn(move || {
            let tree = point::main(radius, values);
            run_clustering(tree, radius, min_points, time)
        }));
    }
    for thread in handlers {
        match thread.join() {
            Ok((results, missing)) => {
                return_set.extend(results);
                missing_count += missing.size();
            }
            Err(e) => {
                log::error!("[S2] Error joining thread: {:?}", e)
            }
        }
    }
    stats.points_covered = stats.total_points - missing_count;
    stats.total_clusters = return_set.len();
    stats.cluster_time = time.elapsed().as_secs_f64();

    println!("total time: {}s", time.elapsed().as_secs_f32());

    return_set.into_iter().map(|p| p.center).collect()
}

fn get_clusters(point: &Point, neighbors: Vec<&Point>, segments: usize, set: &mut HashSet<Point>) {
    for neighbor in neighbors {
        for i in 0..=(segments - 1) {
            let ratio = i as Precision / segments as Precision;
            let new_point = point.interpolate(neighbor, ratio);
            set.insert(new_point);
        }
    }
}

fn run_clustering(
    tree: RTree<Point>,
    radius: f64,
    min_points: usize,
    time: Instant,
) -> (HashSet<Point>, RTree<Point>) {
    println!("made tree: {}", time.elapsed().as_secs_f32());

    let mut initial_clusters = HashSet::new();

    for point in tree.iter() {
        let neighbors = tree.locate_within_distance(point.center, radius * 2.);
        get_clusters(
            point,
            neighbors.into_iter().collect(),
            8,
            &mut initial_clusters,
        );
        initial_clusters.insert(*point);
    }
    println!("Data {} Clusters {}", tree.size(), initial_clusters.len());

    // let cluster_tree = RTree::bulk_load(initial_clusters.into_iter().collect());

    println!("looped: {}s", time.elapsed().as_secs_f32());

    println!("made second tree: {}", time.elapsed().as_secs_f32());

    let mut new_clusters = HashSet::<Point>::new();
    let mut block_clusters = HashSet::<&Point>::new();
    let mut block_points = HashSet::<&Point>::new();

    let cluster_map: HashMap<&Point, Vec<&Point>> = initial_clusters
        .par_iter()
        .map(|cluster| {
            let points = tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            (cluster, points)
        })
        .collect();
    println!("made map: {}", time.elapsed().as_secs_f32());

    let mut highest = 100;
    while highest > min_points {
        let local_clusters = initial_clusters
            .par_iter()
            .filter_map(|cluster| {
                if block_clusters.contains(cluster) {
                    None
                } else {
                    Some((
                        cluster,
                        cluster_map
                            .get(cluster)
                            .unwrap()
                            .iter()
                            .filter_map(|p| {
                                if block_points.contains(p) {
                                    None
                                } else {
                                    Some(*p)
                                }
                            })
                            .collect::<Vec<&Point>>(),
                    ))
                }
            })
            .collect::<Vec<(&Point, Vec<&Point>)>>();

        let mut best = 0;
        for (cluster, points) in local_clusters.iter() {
            let length = points.len() + 1;

            if length > best {
                best = length;
            }
            if length >= highest {
                if block_clusters.contains(cluster) || length == 0 {
                    continue;
                }
                let mut count = 0;
                for point in points {
                    if !block_points.contains(point) {
                        count += 1;
                    }
                }
                if count >= min_points {
                    for point in points {
                        block_points.insert(point);
                    }
                    block_clusters.insert(cluster);
                    new_clusters.insert(**cluster);
                }
            }
        }
        highest = best;
        // println!("Current: {} | {}", highest, new_clusters.len());
    }
    println!("second loop: {}", time.elapsed().as_secs_f32());

    (
        new_clusters,
        RTree::bulk_load(
            tree.iter()
                .filter_map(|point| {
                    if block_points.contains(point) {
                        None
                    } else {
                        Some(*point)
                    }
                })
                .collect::<Vec<Point>>(),
        ),
    )
}
