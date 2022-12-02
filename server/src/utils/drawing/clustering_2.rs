// use super::*;

use geo::Coordinate;
use geohash::encode;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    models::{api::Stats, scanner::GenericData, BBox, SingleVec},
    utils::drawing::helpers::*,
};

trait ClusterCoords {
    fn midpoint(&self, other: &Self) -> Coordinate;
}

impl ClusterCoords for Coordinate {
    fn midpoint(&self, other: &Coordinate) -> Coordinate {
        Coordinate {
            x: (self.x + other.x) / 2.,
            y: (self.y + other.y) / 2.,
        }
    }
}

#[derive(Debug, Clone)]
struct CircleInfo {
    coord: Coordinate,
    bbox: BBox,
    points: HashSet<String>,
}

#[derive(Debug, Clone)]
struct PointInfo {
    coord: Coordinate,
    circles: HashSet<String>,
    points: i16,
}

const PRECISION: usize = 9;
const APPROX_PRECISION: usize = PRECISION - 3;

// fn dev_log(circle_map: &HashMap<String, CircleInfo>, hash: &str) {
//     if let Some(info) = circle_map.get(hash) {
//         println!("{} Points: {}, {:?}", hash, info.points.len(), info.points);
//     }
// }

pub fn brute_force(
    points: &Vec<GenericData>,
    honeycomb: SingleVec,
    radius: f64,
    min_points: usize,
    _generations: usize,
    stats: &mut Stats,
) -> SingleVec {
    let time = Instant::now();
    // unfortunately, due to the borrower, we have to maintain this separately from the point_map
    let mut point_seen_map: HashSet<String> = HashSet::new();

    // Return value is a HashMap to ensure no duplicates are sent
    // TODO: Make into a SingleArray once the algorithm is solid
    let mut final_cluster_map: HashMap<String, CircleInfo> = HashMap::new();

    let (mut point_map, mut circle_map) = create_maps(points, honeycomb, radius);

    merge_circles(&mut circle_map, &mut point_map, radius);

    filter_unique_points(
        &mut point_map,
        &mut circle_map,
        &mut point_seen_map,
        min_points,
        &mut final_cluster_map,
    );

    cleanup_leftovers(
        &point_map,
        &mut point_seen_map,
        &mut final_cluster_map,
        radius,
        min_points,
    );

    stats.cluster_time = time.elapsed().as_secs_f32();
    stats.total_clusters = final_cluster_map.len();
    stats.points_covered = point_seen_map
        .iter()
        .fold(0, |acc, y| acc + point_map.get(y).unwrap().points as usize);
    stats.total_distance = 0.;
    stats.longest_distance = 0.;
    final_cluster_map.clone().into_iter().for_each(|(_, info)| {
        if info.points.len() >= stats.best_cluster_point_count {
            if info.points.len() != stats.best_cluster_point_count {
                stats.best_clusters = vec![];
                stats.best_cluster_point_count = info.points.len();
            }
            stats.best_clusters.push([info.coord.y, info.coord.x]);
        }
    });
    final_cluster_map
        .values()
        .map(|x| [x.coord.y, x.coord.x])
        .collect()
}

// Part 1: Get Information
// Expensive operation to learn which points are in which circles and vice verse
// TODO: Make this less expensive
fn create_maps(
    points: &Vec<GenericData>,
    honeycomb: SingleVec,
    radius: f64,
) -> (HashMap<String, PointInfo>, HashMap<String, CircleInfo>) {
    let time = Instant::now();

    // HashMap of approximate points using trimmed geohashes
    let mut approx_map = HashMap::<String, Vec<(String, Coordinate)>>::new();
    // Set of seen points when using approximate geohashes
    let mut seen_set = HashSet::<String>::new();

    // Hashmap of points and each of the circles they belong in
    // x: lon, y: lat
    let mut point_map: HashMap<String, PointInfo> = HashMap::new();
    let point_total = points.len();

    points.into_iter().for_each(|x| {
        // Flip & create the coord
        let coord = Coordinate {
            x: x.p[1],
            y: x.p[0],
        };
        // Precise Geohash
        let point_key = encode(coord, PRECISION).unwrap();
        // Approximate Geohash
        let approx_key = encode(coord, APPROX_PRECISION).unwrap();

        // Insert into master point map
        point_map
            .entry(point_key.clone())
            .and_modify(|info| info.points += 1)
            .or_insert(PointInfo {
                coord,
                circles: HashSet::new(),
                points: 1,
            });
        // Insert into approx map
        approx_map
            .entry(approx_key)
            .and_modify(|x| x.push((point_key.clone(), coord)))
            .or_insert(vec![(point_key, coord)]);
    });
    println!(
        "Points Total: {} | Consolidated: {} | Check: {}",
        point_total,
        point_map.len(),
        point_map.values().fold(0, |acc, x| acc + x.points)
    );
    println!(
        "Approx Hashes: {} | Approx Total Points: {}",
        approx_map.len(),
        approx_map.values().fold(0, |acc, x| acc + x.len())
    );

    // Hashmap of the circles from the bootstrap generator
    // x: lon, y: lat
    let circle_total = honeycomb.len();

    let mut circle_map: HashMap<_, _> = honeycomb
        .into_iter()
        .map(|x| {
            // Flip & create the coord
            let coord = Coordinate { x: x[1], y: x[0] };
            // Precise Geohash
            let circle_key = encode(coord, PRECISION).unwrap();
            // Approximate Geohash
            let approx_key = encode(coord, APPROX_PRECISION).unwrap();
            // Circle's bbox
            let mut bbox = BBox::new(None);

            let points: HashSet<String> = if let Some(approx_points) = approx_map.get(&approx_key) {
                // Get the points from the approx geohash
                approx_points
                    .into_iter()
                    .filter_map(|(point_key, point_coord)| {
                        // Check if the point is actually within the radius of the precise circle
                        if coord.vincenty_inverse(&point_coord) <= radius {
                            // Mark point as seen
                            seen_set.insert(point_key.clone());
                            // Update circle's bbox
                            bbox.update(*point_coord);

                            point_map
                                .entry(point_key.clone())
                                .and_modify(|mut_point_info| {
                                    mut_point_info.circles.insert(circle_key.clone());
                                });

                            // Insert the point
                            Some(point_key.clone())
                        } else {
                            None
                        }
                    })
                    .collect()
            } else {
                HashSet::new()
            };
            // Insert the circle
            (
                circle_key,
                CircleInfo {
                    coord,
                    bbox,
                    points,
                },
            )
        })
        .collect();

    println!(
        "Pre Check:\nTotal: {} | Consolidated: {} | Circle Checks: {} | Point Checks: {}\nPoints: {} / {}",
        circle_total,
        circle_map.len(),
        circle_map.values().fold(0, |acc, x| acc + x.points.len()),
        point_map.values().fold(0, |acc, x| acc + x.circles.len()),
        seen_set.len(),
        point_map.len(),
    );

    // Loops through points and adds any that were missed by the approx geohashing
    for (point_key, point_info) in point_map.clone().into_iter() {
        if seen_set.contains(&point_key) {
            // Point was seen by the approx geohashing
            continue;
        }
        for (circle_key, circle_info) in circle_map.clone().into_iter() {
            if point_info.coord.vincenty_inverse(&circle_info.coord) <= radius {
                seen_set.insert(point_key.clone());

                circle_map
                    .entry(circle_key.clone())
                    .and_modify(|mut_circle_info| {
                        mut_circle_info.bbox.update(point_info.coord);
                        mut_circle_info.points.insert(point_key.clone());
                    });
                point_map
                    .entry(point_key.clone())
                    .and_modify(|mut_point_info| {
                        mut_point_info.circles.insert(circle_key.clone());
                    });
            }
        }
    }

    println!(
        "Post Check:\nTotal: {} | Consolidated: {} | Circle Checks: {} | Point Checks: {}\nPoints: {} / {}",
        circle_total,
        circle_map.len(),
        circle_map.values().fold(0, |acc, x| acc + x.points.len()),
        point_map.values().fold(0, |acc, x| acc + x.circles.len()),
        seen_set.len(),
        point_map.len()
    );

    // Cleans out empty circles, help with loop times
    let circle_map: HashMap<String, CircleInfo> = circle_map
        .into_iter()
        .filter_map(|(circle_key, circle_info)| {
            if circle_info.points.is_empty() {
                None
            } else {
                Some((circle_key, circle_info))
            }
        })
        .collect();

    println!(
        "Stage 1 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        circle_map.len()
    );
    (point_map, circle_map)
}

// Part 2: Attempt To Merge
// Iterating through the circles from the honeycomb
fn merge_circles(
    circle_map: &mut HashMap<String, CircleInfo>,
    point_map: &mut HashMap<String, PointInfo>,
    radius: f64,
) {
    let time = Instant::now();

    'count: for (circle_key, circle_info) in circle_map.clone().into_iter() {
        let approx_key = circle_key[..(APPROX_PRECISION - 1)].to_string();

        let keys = circle_map
            .clone()
            .into_keys()
            .filter_map(|neighbor_key| {
                if neighbor_key[..(APPROX_PRECISION - 1)] == approx_key {
                    Some(neighbor_key)
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        // println!("Neighbors Found: {}", keys.len());
        let mut best_neighbor = CircleInfo {
            coord: Coordinate { x: 0., y: 0. },
            bbox: BBox::new(Some(&vec![circle_info.coord])),
            points: HashSet::new(),
        };
        let mut best_neighbor_key = "".to_string();

        // println!("Circle: {}", circle_key);

        for neighbor_key in keys {
            if neighbor_key == circle_key {
                continue;
            }
            if let Some(found_neighbor) = circle_map.get(&neighbor_key) {
                // if found_neighbor.exists {
                // LL of the circle and its neighbor
                let lower_left = Coordinate {
                    x: circle_info.bbox.min_x.min(found_neighbor.bbox.min_x),
                    y: circle_info.bbox.min_y.min(found_neighbor.bbox.min_y),
                };
                // UR of the circle and its neighbor
                let upper_right = Coordinate {
                    x: circle_info.bbox.max_x.max(found_neighbor.bbox.max_x),
                    y: circle_info.bbox.max_y.max(found_neighbor.bbox.max_y),
                };
                let distance = lower_left.vincenty_inverse(&upper_right);

                // Checks whether the LL and UR points are within the circle circumference
                if distance <= radius * 2. {
                    // New coord from the midpoint of the LL and UR points
                    let new_coord = lower_left.midpoint(&upper_right);

                    // Combine the points from the circle and its neighbor, ensuring uniqueness
                    let mut new_points = circle_info.points.clone();
                    for coord in found_neighbor.points.clone() {
                        new_points.insert(coord);
                    }

                    if new_points.len() > best_neighbor.points.len() {
                        best_neighbor_key = neighbor_key;
                        best_neighbor.points = new_points;
                        best_neighbor.coord = new_coord;
                    }
                } else if distance <= radius * 2. + 10. {
                    // New coord from the midpoint of the LL and UR points
                    let new_coord = lower_left.midpoint(&upper_right);

                    // Combine the points from the circle and its neighbor, ensuring uniqueness
                    let mut new_points = circle_info.points.clone();
                    for coord in found_neighbor.points.clone() {
                        new_points.insert(coord);
                    }
                    if new_points.len() > best_neighbor.points.len() {
                        if new_points.iter().all(|p| {
                            let point_info = point_map.get(p).unwrap();
                            point_info.coord.vincenty_inverse(&new_coord) <= radius
                        }) {
                            best_neighbor_key = neighbor_key;
                            best_neighbor.points = new_points;
                            best_neighbor.coord = new_coord;
                        }
                    }
                }
            }
        }
        if !best_neighbor_key.is_empty() {
            let new_key = encode(best_neighbor.coord, PRECISION).unwrap();
            circle_map.insert(
                new_key.clone(),
                CircleInfo {
                    bbox: BBox::new(Some(
                        &best_neighbor
                            .points
                            .iter()
                            .filter_map(|x| {
                                if let Some(point) = point_map.get_mut(x) {
                                    point.circles.remove(&circle_key);
                                    point.circles.remove(&best_neighbor_key);
                                    point.circles.insert(new_key.clone());
                                    Some(point.coord)
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )),
                    ..best_neighbor
                },
            );
            // // remove the the circle and the neighboring circle
            // circle_map.remove(&circle_key);
            // circle_map.remove(&best_neighbor_key);

            continue 'count;
        }
    }
    println!(
        "Stage 2 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        circle_map.len()
    );
}

// Part 3:
// Iterating through the points, marking seen,
fn filter_unique_points(
    point_map: &mut HashMap<String, PointInfo>,
    circle_map: &mut HashMap<String, CircleInfo>,
    point_seen_map: &mut HashSet<String>,
    min_points: usize,
    final_cluster_map: &mut HashMap<String, CircleInfo>,
) {
    let time = Instant::now();

    let mut point_vec: Vec<PointInfo> = point_map.clone().into_values().collect();
    point_vec.sort_by(|a, b| a.circles.len().cmp(&b.circles.len()));
    for point_info in point_vec {
        // mut of the circle with the most unique points that this point is in
        let mut best_circle = CircleInfo {
            coord: Coordinate { x: 0., y: 0. },
            bbox: BBox::new(None),
            points: HashSet::new(),
        };

        // iterating through each of the circles that this point is part of
        for circle_key in point_info.clone().circles.iter() {
            if let Some(circle) = circle_map.get(circle_key) {
                let circle = circle.clone();

                // Only counting unique points that haven't been seen already
                let points: HashSet<String> = circle
                    .clone()
                    .points
                    .into_iter()
                    .filter(|x| !point_seen_map.contains(x))
                    .collect();

                let bbox = BBox::new(Some(
                    &points
                        .iter()
                        .filter_map(|x| {
                            if let Some(point) = point_map.get(x) {
                                Some(point.coord)
                            } else {
                                None
                            }
                        })
                        .collect(),
                ));
                // if the point len is more than the best_circle and the min_points, great
                if points.len() > best_circle.points.len() && points.len() >= min_points {
                    best_circle = CircleInfo {
                        points,
                        bbox,
                        ..circle
                    };
                }
            }
        }
        // if the best_circle was found
        if !best_circle.points.is_empty() {
            for point in best_circle.points.clone().into_iter() {
                // mark the points contained in the best_circle as seen
                point_seen_map.insert(point.clone());
            }
            // insert into the final_cluster_map
            final_cluster_map.insert(
                encode(best_circle.clone().coord, PRECISION).unwrap(),
                best_circle,
            );
        }
    }
    println!(
        "Stage 3 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        final_cluster_map.len()
    );
}

fn cleanup_leftovers(
    point_map: &HashMap<String, PointInfo>,
    point_seen_map: &mut HashSet<String>,
    final_cluster_map: &mut HashMap<String, CircleInfo>,
    radius: f64,
    min_points: usize,
) {
    let time = Instant::now();
    let mut new_point_map: HashMap<String, PointInfo> = HashMap::new();

    point_map.into_iter().for_each(|(key, info)| {
        if !point_seen_map.contains(key) {
            new_point_map.insert(
                key.clone(),
                PointInfo {
                    coord: info.coord,
                    circles: HashSet::<String>::new(),
                    points: 0,
                },
            );
        }
    });

    for (point_key, point_info) in new_point_map.clone().into_iter() {
        if point_seen_map.contains(&point_key) {
            continue;
        }
        let mut points: HashSet<String> = HashSet::new();
        for (point_key_2, point_info_2) in new_point_map.clone().into_iter() {
            if point_seen_map.contains(&point_key_2) {
                continue;
            }
            if point_info.coord.vincenty_inverse(&point_info_2.coord) <= radius {
                points.insert(point_key_2);
            }
        }
        if points.len() >= min_points {
            for point in points.clone().into_iter() {
                point_seen_map.insert(point);
            }
            final_cluster_map.insert(
                point_key,
                CircleInfo {
                    coord: point_info.coord,
                    bbox: BBox::new(None),
                    points,
                },
            );
        }
    }
    println!(
        "Stage 4 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        final_cluster_map.len()
    );
}
