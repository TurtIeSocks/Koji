mod point;

use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use point::Point;
// use rand::rngs::mock::StepRng;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;
// use shuffle::{irs::Irs, shuffler::Shuffler};
use std::time::Instant;

use crate::s2::create_cell_map;

struct Comparer {
    cluster: HashSet<Point>,
    missed: usize,
    score: usize,
}

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
            setup(tree, radius, min_points, time)
        }));
    }
    for thread in handlers {
        match thread.join() {
            Ok((results, missing)) => {
                return_set.extend(results);
                missing_count += missing;
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

fn setup(
    tree: RTree<Point>,
    radius: f64,
    min_points: usize,
    time: Instant,
) -> (HashSet<Point>, usize) {
    println!("made tree: {}", time.elapsed().as_secs_f32());

    let points: Vec<&Point> = tree.iter().map(|p| p).collect();

    let mut stats = Stats::new();
    stats.total_points = points.len();

    let initial_clusters = points
        .par_iter()
        .map(|point| {
            let neighbors = tree.locate_within_distance(point.center, radius * 2.);
            get_clusters(point, neighbors.into_iter().collect(), 8)
        })
        .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

    println!(
        "generated potential clusters: {}s",
        time.elapsed().as_secs_f32()
    );

    println!("Data {} Clusters {}", tree.size(), initial_clusters.len());

    // let cluster_tree = RTree::bulk_load(initial_clusters.into_iter().collect());

    let initial_clusters = initial_clusters.into_iter().collect::<Vec<Point>>();

    let mut clusters_with_data: Vec<(&Point, Vec<&Point>)> = initial_clusters
        .par_iter()
        .map(|cluster| {
            let points = tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            (cluster, points)
        })
        .collect();
    println!(
        "added potential clusters: {}s",
        time.elapsed().as_secs_f32()
    );

    // let mut cluster_map = HashMap::<Point, Vec<&Point>>::new();
    // for (key, values) in clusters_with_data {
    //     cluster_map.insert(*key, values);
    // }
    println!("created cluster map: {}s", time.elapsed().as_secs_f32());

    // let mut rng = StepRng::new(2, 13);
    // let mut irs = Irs::default();
    // let mut comparison = Comparer {
    //     cluster: HashSet::new(),
    //     missed: 0,
    //     score: usize::MAX,
    // };
    // let mut tries = 0;

    // while tries < 50 {
    //     match irs.shuffle(&mut clusters_with_data, &mut rng) {
    //         Ok(_) => {}
    //         Err(e) => {
    //             log::warn!("Error while shuffling: {}", e);
    //         }
    //     }

    //     let (new_clusters, missed) =
    //         clustering(min_points, stats.total_points, &clusters_with_data);

    //     stats.total_clusters = new_clusters.len();
    //     stats.points_covered = stats.total_points - missed;
    //     let current_score = stats.get_score(min_points);
    //     if current_score < comparison.score {
    //         comparison.cluster = new_clusters;
    //         comparison.missed = missed;
    //         comparison.score = current_score;
    //     }
    //     tries += 1;
    // }
    // let (new_clusters, missed) = clustering(min_points, stats.total_points, &clusters_with_data);

    println!("while loop finished: {}s", time.elapsed().as_secs_f32());

    // let missed = tree.iter().count() - block_points.len();
    clustering(min_points, stats.total_points, &clusters_with_data)
    // (comparison.cluster, comparison.missed)
}

fn clustering(
    min_points: usize,
    total_points: usize,
    clusters_with_data: &Vec<(&Point, Vec<&Point>)>,
) -> (HashSet<Point>, usize) {
    let mut highest = 100;
    let mut new_clusters = HashSet::<Point>::new();
    let mut block_clusters = HashSet::<&Point>::new();
    let mut block_points = HashSet::<&Point>::new();

    while highest > min_points {
        let local_clusters = clusters_with_data
            .par_iter()
            .filter_map(|(cluster, values)| {
                if block_clusters.contains(*cluster) {
                    None
                } else {
                    Some((
                        *cluster,
                        values
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
                    new_clusters.insert(cluster.clone().to_owned());
                }
            }
        }
        highest = best;
        // println!("Current: {} | {}", highest, new_clusters.len());
    }

    (new_clusters, total_points - block_points.len())
}

// for point in tree.iter() {
//     let neighbors = tree.locate_within_distance(point.center, radius * 2.);
//     get_clusters(
//         point,
//         neighbors.into_iter().collect(),
//         8,
//         &mut initial_clusters,
//     );
//     initial_clusters.insert(*point);
// }
