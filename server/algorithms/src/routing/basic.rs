use geo::Coord;
use geohash::{decode, encode};
use model::api::{args::SortBy, single_vec::SingleVec};
use rand::rngs::mock::StepRng;
use rayon::{
    iter::{IntoParallelIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use s2::{cell::Cell, latlng::LatLng};
use shuffle::{irs::Irs, shuffler::Shuffler};

use crate::rtree::{self, cluster::Cluster, point};

pub trait ClusterSorting {
    fn sort_random(self) -> Self;
    fn sort_random_mut(&mut self);
    fn sort_geohash(&self) -> Self;
    fn sort_geohash_mut(&mut self);
    fn sort_s2(&self) -> Self;
    fn sort_s2_mut(&mut self);
    fn sort_point_count(&self, points: &SingleVec, radius: f64) -> Self;
    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64);
}

impl ClusterSorting for SingleVec {
    fn sort_point_count(&self, points: &SingleVec, radius: f64) -> Self {
        let tree = rtree::spawn(radius, points);
        let clusters: Vec<point::Point> = self
            .into_par_iter()
            .map(|c| point::Point::new(radius, 20, *c))
            .collect();

        let mut clusters: Vec<Cluster<'_>> = rtree::cluster_info(&tree, &clusters);

        clusters.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));

        clusters.into_iter().map(|c| c.point.center).collect()
    }

    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64) {
        *self = self.sort_point_count(points, radius)
    }

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

    fn sort_geohash(&self) -> Self {
        let mut points: Vec<String> = self
            .into_iter()
            .filter_map(|p| match encode(Coord { x: p[1], y: p[0] }, 12) {
                Ok(geohash) => Some(geohash),
                Err(e) => {
                    log::warn!("Error while encoding geohash: {}", e);
                    None
                }
            })
            .collect();

        points.par_sort();

        points
            .into_iter()
            .map(|p| {
                let coord = decode(&p);
                match coord {
                    Ok(coord) => [coord.0.y, coord.0.x],
                    Err(e) => {
                        log::warn!("Error while decoding geohash: {}", e);
                        [0., 0.]
                    }
                }
            })
            .collect()
    }

    fn sort_geohash_mut(&mut self) {
        *self = self.sort_geohash()
    }

    fn sort_s2(&self) -> Self {
        let mut points: Vec<Cell> = self
            .into_iter()
            .map(|p| LatLng::from_degrees(p[0], p[1]).into())
            .collect();

        points.par_sort_by(|a, b| a.id.cmp(&b.id));

        points
            .into_iter()
            .map(|p| {
                let center = p.center();
                [center.latitude().deg(), center.longitude().deg()]
            })
            .collect()
    }

    fn sort_s2_mut(&mut self) {
        *self = self.sort_s2()
    }
}

pub fn sort(points: &SingleVec, clusters: SingleVec, radius: f64, sort_by: &SortBy) -> SingleVec {
    match sort_by {
        SortBy::Random => clusters.sort_random(),
        SortBy::GeoHash => clusters.sort_geohash(),
        SortBy::S2Cell => clusters.sort_s2(),
        SortBy::ClusterCount => clusters.sort_point_count(points, radius),
        _ => clusters,
    }
}
