mod point;

use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use point::Point;
use rand::Rng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::time::Instant;

use crate::s2::create_cell_map;

struct Comparer<'a> {
    cluster: HashSet<&'a Point>,
    missed: usize,
    score: usize,
}

#[derive(Debug, Clone)]
struct Cluster<'a> {
    point: Point,
    points: Vec<&'a Point>,
}

impl Default for Cluster<'_> {
    fn default() -> Self {
        Self {
            point: Point::default(),
            points: vec![],
        }
    }
}

pub fn main(
    points: SingleVec,
    radius: f64,
    min_points: usize,
    cluster_split_level: u64,
    stats: &mut Stats,
) -> SingleVec {
    let time = Instant::now();

    log::info!(
        "[RTREE] starting algorithm with {} data points",
        points.len()
    );
    stats.total_points = points.len();

    let (return_set, missing_count) = if cluster_split_level == 1 {
        setup(points, radius, min_points, time)
    } else {
        let cell_maps = create_cell_map(&points, cluster_split_level);

        let mut handlers = vec![];
        for (key, values) in cell_maps.into_iter() {
            log::debug!("[RTREE] Total {}: {}", key, values.len());
            handlers.push(std::thread::spawn(move || {
                setup(values, radius, min_points, time)
            }));
        }
        log::info!("[RTREE] created {} threads", handlers.len());

        let mut return_set = HashSet::new();
        let mut missing_count = 0;
        for thread in handlers {
            match thread.join() {
                Ok((results, missing)) => {
                    return_set.extend(results);
                    missing_count += missing;
                }
                Err(e) => {
                    log::error!("[RTREE] error joining thread: {:?}", e)
                }
            }
        }
        (return_set, missing_count)
    };

    stats.points_covered = stats.total_points - missing_count;
    stats.total_clusters = return_set.len();
    stats.cluster_time = time.elapsed().as_secs_f64();

    log::info!("[RTREE] total time: {}s", time.elapsed().as_secs_f32());

    return_set.into_iter().map(|p| p.center).collect()
}

fn get_clusters(point: &Point, neighbors: Vec<&Point>, segments: usize) -> HashSet<Point> {
    let mut set = HashSet::<Point>::new();
    for neighbor in neighbors {
        for i in 0..=(segments - 1) {
            let ratio = i as Precision / segments as Precision;
            let new_point = point.interpolate(neighbor, ratio);
            set.insert(new_point);
        }
    }
    set.insert(point.to_owned());
    set
}

fn get_initial_clusters(points: &SingleVec, radius: f64, time: Instant) -> Vec<Point> {
    let double_tree = point::main(radius * 2., points);
    log::info!(
        "[RTREE] Generated second tree with double radius: {}",
        time.elapsed().as_secs_f32()
    );

    let tree_points: Vec<&Point> = double_tree.iter().map(|p| p).collect();

    let clusters = tree_points
        .par_iter()
        .map(|point| {
            let neighbors = double_tree.locate_all_at_point(&point.center);
            get_clusters(point, neighbors.into_iter().collect(), 8)
        })
        .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

    log::info!(
        "[RTREE] generated {} potential clusters: {}",
        clusters.len(),
        time.elapsed().as_secs_f32()
    );
    clusters.into_iter().collect::<Vec<Point>>()
}

fn setup(
    points: Vec<[f64; 2]>,
    radius: f64,
    min_points: usize,
    time: Instant,
) -> (HashSet<Point>, usize) {
    let tree = point::main(radius, &points);
    log::info!(
        "[RTREE] made primary tree: {}s",
        time.elapsed().as_secs_f32()
    );

    let initial_clusters = get_initial_clusters(&points, radius, time);

    let clusters_with_data: Vec<Cluster> = initial_clusters
        .par_iter()
        .map(|cluster| {
            let points = tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            Cluster {
                point: *cluster,
                points,
            }
        })
        .collect();
    log::info!(
        "[RTREE] added data to cluster structs: {}s",
        time.elapsed().as_secs_f32()
    );

    iter_clustering(min_points, points.len(), &clusters_with_data, time)
    // (comparison.cluster, comparison.missed)
}

fn clustering(
    min_points: usize,
    total_points: usize,
    clusters_with_data: &Vec<Cluster>,
    time: Instant,
) -> (HashSet<Point>, usize) {
    log::info!("Starting clustering: {}", time.elapsed().as_secs_f32());
    let mut new_clusters = HashSet::<&Point>::new();
    let mut blocked_clusters = HashSet::<&Point>::new();
    let mut blocked_points = HashSet::<&Point>::new();

    let mut highest = 100;
    while highest > min_points {
        let local_clusters = clusters_with_data
            .par_iter()
            .filter_map(|cluster| {
                if blocked_clusters.contains(&cluster.point) {
                    None
                } else {
                    Some((
                        &cluster.point,
                        cluster
                            .points
                            .iter()
                            .filter_map(|p| {
                                if blocked_points.contains(p) {
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
                if blocked_clusters.contains(*cluster) || length == 0 {
                    continue;
                }
                let mut count = 0;
                for point in points {
                    if !blocked_points.contains(*point) {
                        count += 1;
                    }
                }
                if count >= min_points {
                    for point in points {
                        blocked_points.insert(point);
                    }
                    blocked_clusters.insert(cluster);
                    new_clusters.insert(*cluster);
                }
            }
        }
        highest = best;
        // println!("Current: {} | {}", highest, new_clusters.len());
    }
    log::info!("Finished clustering: {}", time.elapsed().as_secs_f32());
    (
        new_clusters.into_iter().map(|p| *p).collect(),
        total_points - blocked_points.len(),
    )
}

fn iter_clustering(
    min_points: usize,
    total_points: usize,
    clusters_with_data: &Vec<Cluster>,
    time: Instant,
) -> (HashSet<Point>, usize) {
    log::info!("Starting clustering: {}", time.elapsed().as_secs_f32());

    let mut stats = Stats::new();
    stats.total_points = total_points;

    let mut comparison = Comparer {
        cluster: HashSet::new(),
        missed: 0,
        score: usize::MAX,
    };

    let mut rng = rand::thread_rng();
    let length = clusters_with_data.len();

    // let mut highest = 100;
    let mut new_clusters = HashSet::<&Point>::new();
    let mut blocked_clusters = HashSet::<usize>::new();
    let mut blocked_points = HashSet::<&Point>::new();
    let mut total_iterations = 0;

    while total_iterations <= 1_000_000 {
        // log::info!("Starting iteration {}", total_iterations);
        let mut fails = 0;
        while blocked_points.len() != total_points {
            // log::info!(
            //     "Looping: {}  | {} | {}",
            //     comparison.cluster.len(),
            //     blocked_points.len(),
            //     fails
            // );

            if fails > 100 {
                // log::info!("Breaking iteration {}", total_iterations);
                break;
            }
            let blocked_point_ref = blocked_points.len();

            let mut random_index = rng.gen_range(0..length);
            while blocked_clusters.contains(&random_index) {
                // log::info!(
                //     "Checking index: {} | {}",
                //     random_index,
                //     blocked_clusters.contains(&random_index)
                // );
                random_index = rng.gen_range(0..length)
            }
            // log::info!(
            //     "Found Index: {}  | {} | {}",
            //     random_index,
            //     blocked_points.len(),
            //     fails
            // );
            blocked_clusters.insert(random_index);

            let cluster = &clusters_with_data[random_index];
            let valid_points: Vec<&&Point> = cluster
                .points
                .iter()
                .filter(|p| !blocked_points.contains(*p))
                .collect();
            if valid_points.len() >= min_points {
                for point in valid_points.iter() {
                    blocked_points.insert(*point);
                }
                new_clusters.insert(&cluster.point);
            }
            if blocked_point_ref == blocked_points.len() {
                fails += 1;
                // break;
            }
            // log::info!("Loop finished: {}", time.elapsed().as_secs_f32());
        }
        let missed = total_points - blocked_points.len();
        stats.total_clusters = new_clusters.len();
        stats.points_covered = total_points - missed;
        let current_score = stats.get_score(min_points);

        if current_score < comparison.score
        // && if comparison.cluster.is_empty() {
        //     true
        // } else {
        //     comparison.cluster.len() >= stats.total_clusters
        // }
        {
            log::info!(
                "Old Score: {} | New Score: {}| Iteration {}",
                comparison.score,
                current_score,
                total_iterations,
            );
            log::info!(
                "Covered: {} | Clusters: {}",
                stats.points_covered,
                stats.total_clusters
            );
            comparison.cluster = new_clusters.clone();
            comparison.missed = missed;
            comparison.score = current_score;
        }
        fails = 0;
        new_clusters.clear();
        blocked_clusters.clear();
        blocked_points.clear();

        total_iterations += 1;
    }
    log::info!("Finished clustering: {}", time.elapsed().as_secs_f32());
    (
        comparison.cluster.into_iter().map(|p| *p).collect(),
        comparison.missed,
    )
}

/*
    while highest > min_points {
        let local_clusters = clusters_with_data
            .par_iter()
            .filter_map(|cluster| {
                if block_clusters.contains(&cluster.point) {
                    None
                } else {
                    Some((
                        &cluster.point,
                        cluster
                            .points
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
                if block_clusters.contains(*cluster) || length == 0 {
                    continue;
                }
                let mut count = 0;
                for point in points {
                    if !block_points.contains(*point) {
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

*/

// let mut rng = StepRng::new(2, 13);
// let mut irs = Irs::default();
// let mut comparison = Comparer {
//     cluster: HashSet::new(),
//     missed: 0,
//     score: usize::MAX,
// };
// let mut tries = 0;

// while tries < 10 {
//     println!("Starting {} of {}", tries, 10);
//     match irs.shuffle(&mut clusters_with_data, &mut rng) {
//         Ok(_) => {
//             log::info!("Shuffled!")
//         }
//         Err(e) => {
//             log::warn!("Error while shuffling: {}", e);
//             continue;
//         }
//     }

//     let (new_clusters, missed) =
//         clustering(min_points, stats.total_points, &clusters_with_data);

//     stats.total_clusters = new_clusters.len();
//     stats.points_covered = stats.total_points - missed;
//     let current_score = stats.get_score(min_points);
//     if current_score < comparison.score {
//         println!("Current Score: {}", current_score);
//         comparison.cluster = new_clusters;
//         comparison.missed = missed;
//         comparison.score = current_score;
//     }
//     tries += 1;
// }
// let (new_clusters, missed) = clustering(min_points, stats.total_points, &clusters_with_data);

// let missed = tree.iter().count() - block_points.len();
