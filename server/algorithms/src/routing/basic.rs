use geo::Coord;
use geohash::{decode, encode};
use model::api::{args::SortBy, single_vec::SingleVec};
use rand::rngs::mock::StepRng;
use rayon::slice::ParallelSliceMut;
use s2::{cell::Cell, latlng::LatLng};
use shuffle::{irs::Irs, shuffler::Shuffler};

use crate::rtree::{self, cluster::Cluster, point};

fn random(clusters: SingleVec) -> SingleVec {
    let mut clusters = clusters;
    let mut rng = StepRng::new(2, 13);
    let mut irs = Irs::default();
    match irs.shuffle(&mut clusters, &mut rng) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("Error while shuffling: {}", e);
        }
    }
    clusters
}

pub fn cluster_count(points: &SingleVec, clusters: SingleVec, radius: f64) -> SingleVec {
    let tree = rtree::spawn(radius, points);
    let clusters: Vec<point::Point> = clusters
        .into_iter()
        .map(|c| point::Point::new(radius, 20, c))
        .collect();

    let mut clusters: Vec<Cluster<'_>> = rtree::cluster_info(&tree, &clusters);

    clusters.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));

    clusters.into_iter().map(|c| c.point.center).collect()
}

fn geohash(clusters: SingleVec) -> SingleVec {
    let mut points: Vec<String> = clusters
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

fn s2cell(clusters: SingleVec) -> SingleVec {
    let mut points: Vec<Cell> = clusters
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

pub fn sort(points: &SingleVec, clusters: SingleVec, radius: f64, sort_by: SortBy) -> SingleVec {
    match sort_by {
        SortBy::Random => random(clusters),
        SortBy::GeoHash => geohash(clusters),
        SortBy::S2Cell => s2cell(clusters),
        SortBy::ClusterCount => cluster_count(&points, clusters, radius),
        _ => clusters,
    }
}
