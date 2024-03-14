use geo::Coord;
use geohash::encode;
use model::api::single_vec::SingleVec;
use rand::rngs::mock::StepRng;
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use s2::{cellid::CellID, latlng::LatLng};
use shuffle::{irs::Irs, shuffler::Shuffler};

use crate::rtree::{self, cluster, point};

pub trait SortRandom {
    fn sort_random(self) -> Self;
    fn sort_random_mut(&mut self);
}

impl SortRandom for SingleVec {
    fn sort_random(self) -> Self {
        let mut clusters = self;
        clusters.sort_random_mut();
        clusters
    }

    fn sort_random_mut(&mut self) {
        let mut rng = StepRng::new(2, 13);
        let mut irs = Irs::default();
        match irs.shuffle(self, &mut rng) {
            Ok(_) => {}
            Err(e) => {
                log::warn!("Error while shuffling: {}", e);
            }
        }
    }
}

pub trait SortGeohash {
    fn sort_geohash(self) -> Self;
    fn sort_geohash_mut(&mut self);
}

impl SortGeohash for SingleVec {
    fn sort_geohash(self) -> Self {
        let mut points = self;
        points.sort_geohash_mut();
        points
    }

    fn sort_geohash_mut(&mut self) {
        self.par_sort_by(|a, b| {
            match encode(Coord { x: a[1], y: a[0] }, 12) {
                Ok(geohash) => geohash,
                Err(e) => {
                    log::warn!("Error while encoding geohash: {}", e);
                    "".to_string()
                }
            }
            .cmp(&match encode(Coord { x: b[1], y: b[0] }, 12) {
                Ok(geohash) => geohash,
                Err(e) => {
                    log::warn!("Error while encoding geohash: {}", e);
                    "".to_string()
                }
            })
        })
    }
}

pub trait SortS2 {
    fn sort_s2(self) -> Self;
    fn sort_s2_mut(&mut self);
}

impl SortS2 for SingleVec {
    fn sort_s2(self) -> Self {
        let mut points = self;
        points.sort_s2_mut();
        points
    }

    fn sort_s2_mut(&mut self) {
        self.par_sort_by(|a, b| {
            let a: CellID = LatLng::from_degrees(a[0], a[1]).into();
            let b: CellID = LatLng::from_degrees(b[0], b[1]).into();
            a.0.cmp(&b.0)
        });
    }
}

pub trait SortLatLng {
    fn sort_lat_lng(self) -> Self;
    fn sort_lat_lng_mut(&mut self);
}

impl SortLatLng for SingleVec {
    fn sort_lat_lng(self) -> Self {
        let mut points = self;
        points.sort_lat_lng_mut();
        points
    }

    fn sort_lat_lng_mut(&mut self) {
        self.par_sort_by(|a, b| {
            b[0].partial_cmp(&a[0])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b[1].partial_cmp(&a[1]).unwrap_or(std::cmp::Ordering::Equal))
        })
    }
}

pub trait SortPointCount {
    fn sort_point_count(self, points: &SingleVec, radius: f64) -> Self;
    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64);
}

impl SortPointCount for SingleVec {
    fn sort_point_count(self, points: &SingleVec, radius: f64) -> Self {
        let mut clusters = self;
        clusters.sort_point_count_mut(points, radius);
        clusters
    }

    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64) {
        let tree = rtree::spawn(radius, points);
        let clusters: Vec<point::Point> = self
            .par_iter()
            .map(|c| point::Point::new(radius, 20, *c))
            .collect();
        let mut clusters: Vec<cluster::Cluster<'_>> = rtree::cluster_info(&tree, &clusters);
        clusters.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));
        *self = clusters.into_iter().map(|c| c.point.center).collect();
    }
}
