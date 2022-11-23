use geo::{Coordinate, HaversineDestination, HaversineDistance, Point};
use geohash::encode;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{models::scanner::GenericData, utils::drawing::helpers::*};

#[derive(Debug, Clone)]
struct BBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BBox {
    fn new(points: Option<&Vec<Coordinate>>) -> BBox {
        let mut base = BBox {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        if let Some(points) = points {
            for point in points.into_iter() {
                base.min_x = base.min_x.min(point.x);
                base.min_y = base.min_y.min(point.y);
                base.max_x = base.max_x.max(point.x);
                base.max_y = base.max_y.max(point.y);
            }
        }
        base
    }
    fn update(&mut self, coord: Coordinate) {
        self.min_x = self.min_x.min(coord.x);
        self.min_y = self.min_y.min(coord.y);
        self.max_x = self.max_x.max(coord.x);
        self.max_y = self.max_y.max(coord.y);
    }
}

trait ClusterCoords {
    fn midpoint(&self, other: &Self) -> Coordinate;
}

impl ClusterCoords for Point {
    fn midpoint(&self, other: &Point) -> Coordinate {
        Coordinate {
            x: (self.x() + other.x()) / 2.,
            y: (self.y() + other.y()) / 2.,
        }
    }
}

#[derive(Debug, Clone)]
struct CircleInfo {
    coord: Coordinate,
    bbox: BBox,
    points: HashSet<String>,
    exists: bool,
}

#[derive(Debug, Clone)]
struct PointInfo {
    coord: Coordinate,
    circles: HashSet<String>,
    points: i16,
}

const PRECISION: usize = 9;
const APPROX_PRECISION: usize = PRECISION - 3;

pub fn brute_force(
    points: Vec<GenericData>,
    honeycomb: Vec<[f64; 2]>,
    radius: f64,
    min_points: usize,
    generations: usize,
) -> Vec<[f64; 2]> {
    // unfortunately, due to the borrower, we have to maintain this separately from the point_map
    let mut point_seen_map: HashSet<String> = HashSet::new();

    // Return value is a HashMap to ensure no duplicates are sent
    // TODO: Make into a Vec<[f64; 2]> once the algorithm is solid
    let mut final_cluster_map: HashMap<String, CircleInfo> = HashMap::new();

    let (mut point_map, mut circle_map) = create_maps(points, honeycomb, radius);

    let mut count = generations.clone();
    let total = count.clone();
    while count > 0 {
        println!("Generation {} / {}", generations.clone() - count + 1, total);
        // let mut points_map: HashMap<_, _> = if count == generations {
        //     point_map.clone()
        // } else {
        //     point_map
        //         .clone()
        //         .into_iter()
        //         .filter_map(|(key, info)| {
        //             if point_seen_map.contains(&key) {
        //                 None
        //             } else {
        //                 Some((key, info))
        //             }
        //         })
        //         .collect()
        // };

        step_2(
            &mut circle_map,
            &mut point_map,
            radius,
            &mut final_cluster_map,
            &mut point_seen_map,
            min_points,
        );

        step_3(
            &point_map,
            &circle_map,
            &mut point_seen_map,
            min_points,
            &mut final_cluster_map,
        );

        final_cluster_map = step_5(&mut point_map, &circle_map, &mut final_cluster_map, radius);
        count -= 1;
    }

    println!(
        "Circles: {} | Points Seen: {}",
        final_cluster_map.len(),
        point_seen_map
            .iter()
            .fold(0, |acc, y| acc + point_map.get(y).unwrap().points)
    );

    final_cluster_map
        .values()
        .map(|x| [x.coord.y, x.coord.x])
        .collect()
}

// Part 1: Get Information
// Expensive operation to learn which points are in which circles and vice verse
// TODO: Make this less expensive
fn create_maps(
    points: Vec<GenericData>,
    honeycomb: Vec<[f64; 2]>,
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
                    // points: HashSet::new(),
                    points,
                    exists: true,
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

    // if let Some(info) = circle_map.get("drt2z678v") {
    //     println!("drt2z678v Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z67nk") {
    //     println!("drt2z67nk Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z668g") {
    //     println!("drt2z668g Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z655k") {
    //     println!("drt2z655k Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z6h5q") {
    //     println!("drt2z6h5q Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z6kbb") {
    //     println!("drt2z6kbb Points: {}, {:?}", info.points.len(), info.points);
    // }
    // if let Some(info) = circle_map.get("drt2z6knq") {
    //     println!("drt2z6knq Points: {}, {:?}", info.points.len(), info.points);
    // }

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
fn step_2(
    circle_map: &mut HashMap<String, CircleInfo>,
    point_map: &mut HashMap<String, PointInfo>,
    radius: f64,
    final_cluster_map: &mut HashMap<String, CircleInfo>,
    point_seen_map: &mut HashSet<String>,
    min_points: usize,
) {
    let time = Instant::now();
    let neighbor_distance = 0.75_f64.sqrt() * 2. * radius;

    'count: for (circle_key, circle_info) in circle_map.clone().into_iter() {
        for bearing in [30., 90., 150., 210., 270., 330.] {
            let circle_key = circle_key.clone();
            let point: Point = circle_info.coord.into();
            let neighbor_point = point.haversine_destination(bearing, neighbor_distance);
            let neighbor_key = encode(neighbor_point.into(), PRECISION).unwrap();

            // If the circle map already has the neighbor entry
            if let Some(found_neighbor) = circle_map.get(&neighbor_key) {
                // if found_neighbor.exists {
                // LL of the circle and its neighbor
                let lower_left = Point::new(
                    circle_info.bbox.min_x.min(found_neighbor.bbox.min_x),
                    circle_info.bbox.min_y.min(found_neighbor.bbox.min_y),
                );
                // UR of the circle and its neighbor
                let upper_right = Point::new(
                    circle_info.bbox.max_x.max(found_neighbor.bbox.max_x),
                    circle_info.bbox.max_y.max(found_neighbor.bbox.max_y),
                );

                // Checks whether the LL and UR points are within the circle circumference
                if lower_left.haversine_distance(&upper_right) <= radius * 2. {
                    // New coord from the midpoint of the LL and UR points
                    let new_coord = lower_left.midpoint(&upper_right);

                    // Combine the points from the circle and its neighbor, ensuring uniqueness
                    let mut new_points = circle_info.points.clone();
                    for coord in found_neighbor.points.clone() {
                        if !new_points.contains(&coord) {
                            new_points.insert(coord);
                        }
                    }
                    if new_points.len() >= min_points {
                        for point in new_points.clone() {
                            point_seen_map.insert(point);
                        }
                        final_cluster_map.insert(
                            encode(new_coord, PRECISION).unwrap(),
                            CircleInfo {
                                coord: new_coord,
                                bbox: BBox::new(Some(
                                    &new_points
                                        .iter()
                                        .filter_map(|x| {
                                            if let Some(point) = point_map.get(x) {
                                                Some(point.coord)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect(),
                                )),
                                points: new_points,
                                exists: true,
                                // exists_2: true,
                            },
                        );
                        // // remove the the circle and the neighboring circle
                        circle_map
                            .entry(circle_key)
                            .and_modify(|info| info.exists = false);
                        circle_map
                            .entry(neighbor_key)
                            .and_modify(|info| info.exists = false);

                        continue 'count;
                    }
                }
            }
        }
    }
    println!(
        "Stage 2 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        circle_map
            .clone()
            .into_values()
            .filter(|x| x.exists)
            .count()
    );
}

// Part 3:
// Iterating through the points, marking seen,
fn step_3(
    point_map: &HashMap<String, PointInfo>,
    circle_map: &HashMap<String, CircleInfo>,
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
            exists: false,
            // exists_2: false,
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

// fn step_4(
//     point_map: &HashMap<String, PointInfo>,
//     final_cluster_map: &mut HashMap<String, CircleInfo>,
// ) {
//     for (circle_key, circle_info) in final_cluster_map.clone().into_iter() {}
// }

fn step_5(
    point_map: &mut HashMap<String, PointInfo>,
    _circle_map: &HashMap<String, CircleInfo>,
    final_cluster_map: &mut HashMap<String, CircleInfo>,
    radius: f64,
) -> HashMap<String, CircleInfo> {
    let time = Instant::now();
    let mut final_cluster_map_2 = HashMap::new();

    'first: for (key_1, info_1) in final_cluster_map.clone().into_iter() {
        for (key_2, info_2) in final_cluster_map.clone().into_iter() {
            // LL of the circle and its neighbor
            let lower_left = Point::new(
                info_1.bbox.min_x.min(info_2.bbox.min_x),
                info_1.bbox.min_y.min(info_2.bbox.min_y),
            );
            // UR of the circle and its neighbor
            let upper_right = Point::new(
                info_1.bbox.max_x.max(info_2.bbox.max_x),
                info_1.bbox.max_y.max(info_2.bbox.max_y),
            );

            // Checks whether the LL and UR points are within the circle circumference
            if lower_left.haversine_distance(&upper_right) <= radius * 2. {
                // New coord from the midpoint of the LL and UR points
                let new_coord = lower_left.midpoint(&upper_right);

                // Combine the points from the circle and its neighbor, ensuring uniqueness
                let mut new_points = info_1.points.clone();
                let new_key = encode(new_coord, PRECISION).unwrap();
                for coord in info_2.points.clone() {
                    if !new_points.contains(&coord.clone()) {
                        new_points.insert(coord.clone());
                    }
                    if let Some(point) = point_map.get_mut(&coord) {
                        point.circles.insert(new_key.clone());
                    }
                }
                final_cluster_map_2.insert(
                    new_key,
                    CircleInfo {
                        coord: new_coord,
                        bbox: BBox::new(Some(
                            &new_points
                                .iter()
                                .filter_map(|x| {
                                    if let Some(point) = point_map.get(x) {
                                        Some(point.coord)
                                    } else {
                                        None
                                    }
                                })
                                .collect(),
                        )),
                        points: new_points,
                        exists: false,
                    },
                );
                final_cluster_map
                    .entry(key_1)
                    .and_modify(|info| info.exists = false);
                final_cluster_map
                    .entry(key_2)
                    .and_modify(|info| info.exists = false);

                continue 'first;
            }
        }
        if final_cluster_map.get(&key_1).unwrap().exists {
            final_cluster_map_2.insert(key_1, info_1);
        }
    }

    final_cluster_map
        .values_mut()
        .for_each(|info| info.exists = true);
    println!(
        "Stage 4 time: {}s | Circles: {}",
        time.elapsed().as_secs_f64(),
        final_cluster_map_2.len(),
    );

    final_cluster_map_2
}
