use super::write_debug;

use geo::{Coordinate, HaversineDestination, Point};
use geohash::encode;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use crate::{
    models::{api::Stats, scanner::GenericData, BBox, SingleVec},
    utils::drawing::helpers::*,
};

pub mod helpers;
pub mod leftovers;
pub mod maps;
pub mod unique;
pub mod wiggle;

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
pub struct CircleInfo {
    pub coord: Coordinate,
    pub bbox: BBox,
    pub points: HashSet<String>,
    pub unique: HashSet<String>,
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
    pub fn get_points(
        &self,
        point_map: &HashMap<String, PointInfo>,
        key: CiKeys,
    ) -> Vec<Coordinate> {
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
    pub coord: Coordinate,
    pub circles: HashSet<String>,
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
            }
        }
    }
}

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

    let (mut point_map, mut circle_map) = maps::run(points, honeycomb, radius);

    wiggle::run(&mut circle_map, &mut point_map, radius);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        write_debug::hashmap("pre_circles.txt", &circle_map).expect("Unable to write circles.txt");
        write_debug::hashmap("pre_points.txt", &point_map).expect("Unable to write points.txt");
    }

    unique::run(&mut point_map, &mut circle_map, radius);

    for info in circle_map.clone().values() {
        for point in info.combine() {
            point_seen_map.insert(point);
        }
    }
    if point_seen_map.len() != points.len() {
        // println!("Missed Points: {}", points.len() - point_seen_map.len());
        leftovers::run(
            &point_map,
            &mut point_seen_map,
            &mut circle_map,
            radius,
            min_points,
        );
    }
    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        write_debug::hashmap("circles.txt", &circle_map).expect("Unable to write circles.txt");
        write_debug::hashmap("points.txt", &point_map).expect("Unable to write points.txt");
    }

    stats.cluster_time = time.elapsed().as_secs_f32();
    stats.total_clusters = circle_map.len();
    stats.points_covered = point_seen_map.len();
    stats.total_distance = 0.;
    stats.longest_distance = 0.;
    circle_map.clone().into_iter().for_each(|(_, info)| {
        let combined = info.combine();
        if combined.len() >= stats.best_cluster_point_count {
            if combined.len() != stats.best_cluster_point_count {
                stats.best_clusters = vec![];
                stats.best_cluster_point_count = combined.len();
            }
            stats.best_clusters.push([info.coord.y, info.coord.x]);
        }
    });
    circle_map
        .values()
        .filter_map(|x| {
            if x.combine().len() >= min_points {
                Some([x.coord.y, x.coord.x])
            } else {
                None
            }
        })
        .collect()
}
