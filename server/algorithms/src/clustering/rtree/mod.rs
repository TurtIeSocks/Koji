mod point;

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use point::{Point, PointType};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
use s2::cellid::CellID;

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
            let mut tree = point::main(radius, values);
            run_clustering(&mut tree, radius, min_points, time)
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

    // let (mut new_clusters, new_tree) = run_clustering(tree, radius, min_points, time, 1);
    // println!("new tree size: {}", new_tree.size());
    // let (clusters_2, tree_2) = run_clustering(new_tree, radius, min_points, time, 2);
    // new_clusters.extend(clusters_2);
    // println!("new tree size: {}", tree_2.size());
    // let (clusters_3, tree_3) = run_clustering(tree_2, radius, min_points, time, 3);
    // new_clusters.extend(clusters_3);
    // println!("new tree size: {}", tree_3.size());

    stats.points_covered = stats.total_points - missing_count;
    stats.total_clusters = return_set.len();
    stats.cluster_time = time.elapsed().as_secs_f64();

    println!("total time: {}s", time.elapsed().as_secs_f64());

    return_set.into_iter().map(|p| p.center).collect()
}

fn run_clustering(
    tree: &mut RTree<Point>,
    radius: f64,
    min_points: usize,
    time: Instant,
) -> (HashSet<Point>, RTree<Point>) {
    println!("made tree: {}", time.elapsed().as_secs_f64());

    let mut initial_clusters = vec![];
    for point in tree.iter() {
        let neighbors = tree.locate_within_distance(point.center, radius * 2.);
        let mut lat = 0.0;
        let mut lon = 0.0;
        let mut points = HashSet::new();
        for neighbor in neighbors {
            lat += neighbor.center[0];
            lon += neighbor.center[1];
            points.insert(neighbor.cell_id);

            let midpoint = point.midpoint(&neighbor);
            initial_clusters.push(midpoint);

            // if iteration > 1 {
            // let quarter = point.midpoint(&midpoint);
            // initial_clusters.push(quarter);

            // let three_quarter = midpoint.midpoint(neighbor);
            // initial_clusters.push(three_quarter);

            // if iteration > 2 {
            // let eighth = point.midpoint(&quarter);
            // initial_clusters.push(eighth);

            // let three_eighths = quarter.midpoint(&midpoint);
            // initial_clusters.push(three_eighths);

            // let five_eighths = midpoint.midpoint(&three_quarter);
            // initial_clusters.push(five_eighths);

            // let seven_eighths = three_quarter.midpoint(&neighbor);
            // initial_clusters.push(seven_eighths);
            //     }
            // }
        }
        let count = points.len();
        log::info!("Count {}", count);

        let center = if count == 0 {
            point.center
        } else {
            lat /= count as Precision;
            lon /= count as Precision;
            [lat, lon]
        };
        initial_clusters.push(Point::new(radius, center, PointType::Cluster));
    }
    println!("looped: {}s", time.elapsed().as_secs_f64());

    println!("made second tree: {}", time.elapsed().as_secs_f64());

    let mut new_clusters = HashSet::<Point>::new();
    let mut block_clusters = HashSet::<&Point>::new();
    let mut block_points = HashSet::<&Point>::new();

    println!("Data {} Clusters {}", tree.size(), initial_clusters.len());

    let cluster_map: HashMap<CellID, Vec<&Point>> = initial_clusters
        .par_iter()
        .map(|cluster| {
            let points = tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            (cluster.cell_id, points)
        })
        .collect();
    println!("made map: {}", time.elapsed().as_secs_f64());

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
                            .get(&cluster.cell_id)
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
                if cluster.blocked || length == 0 {
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
        println!("Current: {} | {}", highest, new_clusters.len());
    }
    println!("second loop: {}", time.elapsed().as_secs_f64());

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
