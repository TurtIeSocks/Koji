pub mod cluster;
pub mod point;

use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, Precision};
use point::Point;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
use std::time::Instant;

use crate::s2::create_cell_map;
use cluster::Cluster;

pub fn main(
    points: &SingleVec,
    radius: f64,
    min_points: usize,
    cluster_split_level: u64,
) -> SingleVec {
    let time = Instant::now();

    log::info!(
        "[RTREE] starting algorithm with {} data points",
        points.len()
    );

    let return_set = if cluster_split_level == 1 {
        setup(points, radius, min_points, time)
    } else {
        let cell_maps = create_cell_map(&points, cluster_split_level);

        let mut handlers = vec![];
        for (key, values) in cell_maps.into_iter() {
            log::debug!("[RTREE] Total {}: {}", key, values.len());
            handlers.push(std::thread::spawn(move || {
                setup(&values, radius, min_points, time)
            }));
        }
        log::info!("[RTREE] created {} threads", handlers.len());

        let mut return_set = HashSet::new();
        for thread in handlers {
            match thread.join() {
                Ok(results) => {
                    return_set.extend(results);
                }
                Err(e) => {
                    log::error!("[RTREE] error joining thread: {:?}", e)
                }
            }
        }
        return_set
    };

    log::info!("[RTREE] {}s | finished", time.elapsed().as_secs_f32());

    return_set.into_iter().map(|p| p.center).collect()
}

fn generate_clusters(point: &Point, neighbors: Vec<&Point>, segments: usize) -> HashSet<Point> {
    let mut set = HashSet::<Point>::new();
    for neighbor in neighbors {
        for i in 0..=(segments - 1) {
            let ratio = i as Precision / segments as Precision;
            let new_point = point.interpolate(neighbor, ratio, 0., 0.);
            set.insert(new_point);
            for wiggle in vec![0.00025, 0.0001] {
                let wiggle_lat: f64 = wiggle / 2.;
                let wiggle_lon = wiggle;
                let random_point = point.interpolate(neighbor, ratio, wiggle_lat, wiggle_lon);
                set.insert(random_point);
                let random_point = point.interpolate(neighbor, ratio, wiggle_lat, -wiggle_lon);
                set.insert(random_point);
                let random_point = point.interpolate(neighbor, ratio, -wiggle_lat, wiggle_lon);
                set.insert(random_point);
                let random_point = point.interpolate(neighbor, ratio, -wiggle_lat, -wiggle_lon);
                set.insert(random_point);
            }
        }
    }
    set.insert(point.to_owned());
    set
}

fn get_initial_clusters(tree: &RTree<Point>) -> Vec<Point> {
    let tree_points: Vec<&Point> = tree.iter().map(|p| p).collect();

    let clusters = tree_points
        .par_iter()
        .map(|point| {
            let neighbors = tree.locate_all_at_point(&point.center);
            generate_clusters(point, neighbors.into_iter().collect(), 8)
        })
        .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

    clusters.into_iter().collect::<Vec<Point>>()
}

fn setup(points: &SingleVec, radius: f64, min_points: usize, time: Instant) -> HashSet<Point> {
    let point_tree: RTree<Point> = point::main(radius, points);
    log::info!(
        "[RTREE] {}s | created point tree",
        time.elapsed().as_secs_f32()
    );

    let neighbor_tree: RTree<Point> = point::main(radius * 2., points);
    log::info!(
        "[RTREE] {}s | created neighbor tree",
        time.elapsed().as_secs_f32()
    );

    let initial_clusters = get_initial_clusters(&neighbor_tree);
    log::info!(
        "[RTREE] {}s | created possible clusters: {}",
        time.elapsed().as_secs_f32(),
        initial_clusters.len()
    );

    let clusters_with_data: Vec<Cluster> = initial_clusters
        .par_iter()
        .map(|cluster| {
            let mut points: Vec<&Point> = point_tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            if point_tree.contains(cluster) && points.is_empty() {
                points.push(cluster)
            }
            Cluster::new(cluster, points.into_iter(), vec![].into_iter())
        })
        .collect();
    log::info!(
        "[RTREE] {}s | associated points with clusters",
        time.elapsed().as_secs_f32()
    );
    log::info!(
        "[RTREE] {}s | starting initial solution",
        time.elapsed().as_secs_f32()
    );
    let solution = initial_solution(min_points, clusters_with_data);
    log::info!(
        "[RTREE] {}s | finished initial solution",
        time.elapsed().as_secs_f32()
    );
    log::info!("[RTREE] Initial solution size: {}", solution.len());
    log::info!(
        "[RTREE] {}s | starting deduping",
        time.elapsed().as_secs_f32()
    );
    let solution = dedupe(solution, min_points);
    log::info!(
        "[RTREE] {}s | finished deduping",
        time.elapsed().as_secs_f32()
    );
    log::info!("[RTREE] Deduped solution size: {}", solution.len());

    solution
}

fn initial_solution(min_points: usize, clusters_with_data: Vec<Cluster>) -> HashSet<Cluster> {
    let mut new_clusters = HashSet::<Cluster>::new();
    let mut blocked_points = HashSet::<&Point>::new();

    let mut highest = 100;
    while highest > min_points {
        let local_clusters = clusters_with_data
            .par_iter()
            .filter_map(|cluster| {
                if new_clusters.contains(cluster) {
                    None
                } else {
                    Some(Cluster::new(
                        cluster.point,
                        cluster.all.clone().into_iter(),
                        cluster.all.iter().filter_map(|p| {
                            if blocked_points.contains(p) {
                                None
                            } else {
                                Some(*p)
                            }
                        }),
                    ))
                }
            })
            .collect::<Vec<Cluster>>();

        let mut best = 0;
        for cluster in local_clusters.into_iter() {
            let length = cluster.points.len() + 1;
            if length > best {
                best = length;
            }
            if length >= highest {
                if new_clusters.contains(&cluster) || length == 0 {
                    continue;
                }
                let mut count = 0;
                for point in cluster.points.iter() {
                    if !blocked_points.contains(point) {
                        count += 1;
                        if count >= min_points {
                            break;
                        }
                    }
                }
                if count >= min_points {
                    for point in cluster.points.iter() {
                        blocked_points.insert(point);
                    }
                    new_clusters.insert(cluster);
                }
            }
        }
        highest = best;
    }
    new_clusters
}

fn dedupe(initial_solution: HashSet<Cluster>, min_points: usize) -> HashSet<Point> {
    // let mut point_map: HashMap<String, HashSet<String>> = HashMap::new();
    // let mut cluster_map: HashMap<String, HashSet<String>> = HashMap::new();

    // for cluster in initial_solution.iter() {
    //     cluster_map.insert(
    //         cluster.point._get_geohash(),
    //         cluster.points.iter().map(|p| p._get_geohash()).collect(),
    //     );
    //     for point in cluster.points.iter() {
    //         point_map
    //             .entry(point._get_geohash())
    //             .and_modify(|f| {
    //                 f.insert(cluster.point._get_geohash());
    //             })
    //             .or_insert_with(|| {
    //                 let mut set: HashSet<String> = HashSet::new();
    //                 set.insert(cluster.point._get_geohash());
    //                 set
    //             });
    //     }
    // }

    // debug_hashmap("point_map.txt", &point_map).unwrap();
    // debug_hashmap("cluster_map.txt", &cluster_map).unwrap();

    let mut seen_points: HashSet<&Point> = HashSet::new();
    let mut solution: HashSet<Point> = initial_solution
        .iter()
        .filter_map(|cluster| {
            let unique_points = cluster
                .points
                .iter()
                .collect::<Vec<&&Point>>()
                .par_iter()
                .filter(|p| {
                    initial_solution
                        .iter()
                        .find(|c| c.point != cluster.point && c.all.contains(**p))
                        .is_none()
                })
                .count();

            if unique_points == 0 {
                None
            } else {
                seen_points.extend(cluster.points.iter());
                Some(*cluster.point)
            }
        })
        .collect();

    if min_points == 1 {
        // let mut count = 0;
        for cluster in initial_solution {
            let valid = cluster
                .points
                .iter()
                .find(|p| !seen_points.contains(*p))
                .is_some();
            if valid {
                solution.insert(*cluster.point);
                seen_points.extend(cluster.points.iter());
                // count += 1;
            }
        }
        // log::info!("Extra clusters: {}", count);
    }

    solution
}
