use crate::utils;

use super::utils::debug_hashmap;

use geo::{Coord, HaversineDestination, HaversineDistance, Point};
use geohash::encode;
use model::api::{args::SortBy, single_vec::SingleVec, BBox};
use rand::rngs::mock::StepRng;
use shuffle::irs::Irs;
use shuffle::shuffler::Shuffler;

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

pub mod helpers;
pub mod leftovers;
pub mod maps;
pub mod unique;
pub mod wiggle;

trait ClusterCoords {
    fn midpoint(&self, other: &Self) -> Point;
}

impl ClusterCoords for Point {
    fn midpoint(&self, other: &Point) -> Point {
        Point::new((self.x() + other.x()) / 2., (self.y() + other.y()) / 2.)
    }
}

#[derive(Debug, Clone, Default)]
pub struct CircleInfo {
    pub coord: Point,
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
    pub fn get_points(&self, point_map: &HashMap<String, PointInfo>, key: CiKeys) -> Vec<Point> {
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
    pub coord: Point,
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

pub fn cluster(
    points: &SingleVec,
    honeycomb: SingleVec,
    radius: f64,
    min_points: usize,
    only_unique: bool,
    sort_by: &SortBy,
) -> SingleVec {
    // unfortunately, due to the borrower, we have to maintain this separately from the point_map
    let mut point_seen_map: HashSet<String> = HashSet::new();

    let (mut point_map, mut circle_map) = maps::run(points, honeycomb, radius, min_points);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        debug_hashmap("pre_circles.txt", &circle_map).expect("Unable to write circles.txt");
        debug_hashmap("pre_points.txt", &point_map).expect("Unable to write points.txt");
    }

    wiggle::run(&mut circle_map, &mut point_map, radius, min_points);

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        debug_hashmap("wiggle_circles.txt", &circle_map).expect("Unable to write circles.txt");
        debug_hashmap("wiggle_points.txt", &point_map).expect("Unable to write points.txt");
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
    let mut circle_map = utils::get_sorted(&circle_map)
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
    let mut sorted = utils::get_sorted(&circle_map);

    match sort_by {
        SortBy::ClusterCount => {
            sorted.sort_by(|a, b| b.1.combine().len().cmp(&a.1.combine().len()));
        }
        SortBy::Random => {
            let mut rng = StepRng::new(2, 13);
            let mut irs = Irs::default();
            match irs.shuffle(&mut sorted, &mut rng) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Error while shuffling: {}", e);
                }
            }
        }
        _ => {}
    }

    if std::env::var("DEBUG").unwrap_or("false".to_string()) == "true" {
        debug_hashmap("circles.txt", &circle_map).expect("Unable to write circles.txt");
        debug_hashmap("points.txt", &point_map).expect("Unable to write points.txt");
        debug_hashmap(
            "sorting.txt",
            &sorted
                .iter()
                .map(|x| (&x.0, x.1.combine().len()))
                .collect::<Vec<(&String, usize)>>(),
        )
        .expect("Unable to write sorting.txt");
    }

    sorted
        .into_iter()
        .filter_map(|x| {
            if x.1.meets_min {
                Some([x.1.coord.y(), x.1.coord.x()])
            } else {
                None
            }
        })
        .collect()
}
