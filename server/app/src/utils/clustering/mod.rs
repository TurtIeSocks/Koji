use super::write_debug;

use geo::{Coord, HaversineDestination, HaversineDistance, Point};
use geohash::encode;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    models::{api::Stats, scanner::GenericData, single_vec::SingleVec, BBox},
    utils::drawing::helpers::*,
};

pub mod helpers;
pub mod leftovers;
pub mod maps;
pub mod unique;
pub mod wiggle;

trait ClusterCoords {
    fn midpoint(&self, other: &Self) -> Coord;
}

impl ClusterCoords for Coord {
    fn midpoint(&self, other: &Coord) -> Coord {
        Coord {
            x: (self.x + other.x) / 2.,
            y: (self.y + other.y) / 2.,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircleInfo {
    pub coord: Coord,
    pub bbox: BBox,
    pub points: HashSet<String>,
    pub unique: HashSet<String>,
    pub meets_min: bool,
}

pub enum CiKeys {
    // Points,
    Unique,
    // Combined,
}

impl CircleInfo {
    pub fn combine(&self) -> HashSet<String> {
        let mut points = self.points.clone();
        points.extend(self.unique.clone());
        points
    }
    pub fn get_points(&self, point_map: &HashMap<String, PointInfo>, key: CiKeys) -> Vec<Coord> {
        match key {
            // CiKeys::Points => self.points.clone(),
            CiKeys::Unique => self.unique.clone(),
            // CiKeys::Combined => self.combine(),
        }
        .iter()
        .map(|point| point_map.get(point).unwrap().coord)
        .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PointInfo {
    pub coord: Coord,
    pub circles: HashSet<String>,
    pub points: usize,
}

pub const PRECISION: usize = 9;
pub const APPROX_PRECISION: usize = PRECISION - 3;

pub fn _dev_log(
    circle_map: &HashMap<String, CircleInfo>,
    hash: &str,
    point_map: &HashMap<String, PointInfo>,
) {
    if let Some(info) = circle_map.get(hash) {
        let combined = info.combine();
        println!(
            "\n{} Points: {}, {:?}",
            hash,
            info.points.len(),
            info.points
        );
        println!(
            "{} Unique: {}, {:?}\n",
            hash,
            info.unique.len(),
            info.unique
        );
        for point in combined.iter() {
            if let Some(p) = point_map.get(point) {
                println!("{} Circles: {}, {:?}", point, p.circles.len(), p.circles);
            } else {
                println!("Point_map does not contain {}", point);
            }
        }
    } else {
        println!("Circle_map does not contain {}", hash);
    }
}

pub fn brute_force(
    points: &Vec<GenericData>,
    honeycomb: SingleVec,
    radius: f64,
    min_points: usize,
    _generations: usize,
    stats: &mut Stats,
    only_unique: bool,
) -> SingleVec {
    let time = Instant::now();
    // unfortunately, due to the borrower, we have to maintain this separately from the point_map
    let mut point_seen_map: HashSet<String> = HashSet::new();

    let (mut point_map, mut circle_map) = maps::run(points, honeycomb, radius, min_points);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        write_debug::hashmap("pre_circles.txt", &circle_map).expect("Unable to write circles.txt");
        write_debug::hashmap("pre_points.txt", &point_map).expect("Unable to write points.txt");
    }

    wiggle::run(&mut circle_map, &mut point_map, radius, min_points);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        write_debug::hashmap("wiggle_circles.txt", &circle_map)
            .expect("Unable to write circles.txt");
        write_debug::hashmap("wiggle_points.txt", &point_map).expect("Unable to write points.txt");
    }

    unique::run(&mut point_map, &mut circle_map, radius, min_points);

    for info in circle_map.clone().values() {
        if info.meets_min {
            for point in info.combine() {
                point_seen_map.insert(point);
            }
        }
    }
    let mut count = 0;
    let mut circle_map = helpers::get_sorted(&circle_map)
        .into_iter()
        .filter_map(|(circle_key, circle_info)| {
            if circle_info.unique.is_empty() {
                count += 1;
                return None;
            }
            Some((circle_key, circle_info))
        })
        .collect();

    println!("Removed at the end {}", count);
    if point_seen_map.len() != points.len() {
        println!("Missed Points: {}", points.len() - point_seen_map.len());
        leftovers::run(
            &point_map,
            &mut point_seen_map,
            &mut circle_map,
            radius,
            min_points,
        );
    }
    if only_unique {
        for info in circle_map.values_mut() {
            info.meets_min = info.unique.len() >= min_points;
        }
    }
    let sorted = helpers::get_sorted(&circle_map);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        write_debug::hashmap("circles.txt", &circle_map).expect("Unable to write circles.txt");
        write_debug::hashmap("points.txt", &point_map).expect("Unable to write points.txt");
    }

    // stats.points_covered = point_seen_map
    //     .iter()
    //     .fold(0, |acc, y| acc + point_map.get(y).unwrap().points);
    stats.total_distance = 0.;
    stats.longest_distance = 0.;
    stats.total_clusters = 0;
    point_seen_map.clear();

    for (i, info) in sorted.iter().enumerate() {
        if info.1.meets_min {
            for point in info.1.combine() {
                point_seen_map.insert(point);
            }
            let point: Point = info.1.coord.into();
            let point2: Point = if i == sorted.len() - 1 {
                sorted[0].1.coord.into()
            } else {
                sorted[i + 1].1.coord.into()
            };
            let distance = point.haversine_distance(&point2);
            stats.total_distance += distance;
            if distance > stats.longest_distance {
                stats.longest_distance = distance;
            }
            let combined = info.1.combine();
            if combined.len() >= stats.best_cluster_point_count {
                if combined.len() != stats.best_cluster_point_count {
                    stats.best_clusters = vec![];
                    stats.best_cluster_point_count = combined.len();
                }
                stats.best_clusters.push([info.1.coord.y, info.1.coord.x]);
            }
        }
    }
    stats.points_covered = point_seen_map.len();
    stats.cluster_time = time.elapsed().as_secs_f32();

    sorted
        .into_iter()
        .filter_map(|x| {
            if x.1.meets_min {
                stats.total_clusters += 1;
                Some([x.1.coord.y, x.1.coord.x])
            } else {
                None
            }
        })
        .collect()
}
