mod point;

use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use point::Point;
use rand::Rng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crate::s2::create_cell_map;

struct Comparer<'a> {
    clusters: HashSet<&'a Point>,
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

fn get_initial_clusters(tree: &RTree<Point>, time: Instant) -> Vec<Point> {
    // let tree = point::main(radius * 2., points);
    // log::info!(
    //     "[RTREE] Generated second tree with double radius: {}s",
    //     time.elapsed().as_secs_f32()
    // );

    let tree_points: Vec<&Point> = tree.iter().map(|p| p).collect();

    let clusters = tree_points
        .par_iter()
        .map(|point| {
            let neighbors = tree.locate_all_at_point(&point.center);
            get_clusters(point, neighbors.into_iter().collect(), 8)
        })
        .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

    log::info!(
        "[RTREE] generated {} potential clusters: {}s",
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

    let initial_clusters = get_initial_clusters(&tree, time);

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

    clustering(min_points, points.len(), &clusters_with_data, time)
    // (comparison.cluster, comparison.missed)
}

fn initial_solution(
    min_points: usize,
    clusters_with_data: &Vec<Cluster>,
    time: Instant,
) -> (HashSet<Point>, usize) {
    log::info!(
        "Starting initial solution: {}s",
        time.elapsed().as_secs_f32()
    );
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
    }
    log::info!(
        "Finished initial solution: {}s",
        time.elapsed().as_secs_f32()
    );
    (
        new_clusters.into_iter().map(|p| *p).collect(),
        blocked_points.len(),
    )
}

fn clustering(
    min_points: usize,
    total_points: usize,
    clusters_with_data: &Vec<Cluster>,
    time: Instant,
) -> (HashSet<Point>, usize) {
    log::info!("Starting clustering: {}s", time.elapsed().as_secs_f32());

    let (clusters, covered) = initial_solution(min_points, clusters_with_data, time);

    let comparison = Comparer {
        clusters: clusters.iter().collect(),
        missed: total_points - covered,
        score: clusters.len() * min_points + (total_points - covered),
    };
    let arc = Arc::new(Mutex::new(comparison));
    let length = clusters_with_data.len();

    thread::scope(|scope| {
        for i in 0..num_cpus::get() {
            let arc_clone = Arc::clone(&arc);
            scope.spawn(move || {
                let mut rng = rand::thread_rng();
                let mut stats = Stats::new();
                stats.total_points = total_points;

                let mut iteration = 0;

                while iteration <= 100_000 {
                    let mut fails = 0;
                    let mut new_clusters = HashSet::<&Point>::new();
                    let mut blocked_clusters = HashSet::<usize>::new();
                    let mut blocked_points = HashSet::<&Point>::new();

                    log::info!("Thread: {}, Iteration: {}", i, iteration);
                    while fails < 100 {
                        let random_index = rng.gen_range(0..length);
                        if blocked_clusters.contains(&random_index) {
                            continue;
                        }
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
                            continue;
                        }
                        fails += 1;
                    }
                    let missed = total_points - blocked_points.len();
                    stats.total_clusters = new_clusters.len();
                    stats.points_covered = total_points - missed;
                    let current_score = stats.get_score(min_points);

                    let mut comparison = arc_clone.lock().unwrap();
                    let is_better = if current_score == comparison.score {
                        new_clusters.len() < comparison.clusters.len()
                    } else {
                        current_score < comparison.score
                    };
                    if is_better {
                        log::info!(
                            "Old Score: {} | New Score: {}| Iteration {}",
                            comparison.score,
                            current_score,
                            iteration,
                        );
                        log::info!(
                            "Covered: {} | Clusters: {}",
                            stats.points_covered,
                            stats.total_clusters
                        );
                        comparison.clusters = new_clusters.clone();
                        comparison.missed = missed;
                        comparison.score = current_score;
                    }
                    fails = 0;
                    iteration += 1;
                }
            });
        }
    });

    let final_result = arc.lock().unwrap();

    log::info!("Finished clustering: {}s", time.elapsed().as_secs_f32());
    (
        final_result.clusters.iter().map(|p| **p).collect(),
        final_result.missed,
    )
}
