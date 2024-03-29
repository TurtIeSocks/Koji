pub mod cluster;
pub mod point;

use model::api::{single_vec::SingleVec, Precision};
use point::Point;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

pub trait SortDedupe {
    fn sort_dedupe(&mut self);
}

pub fn spawn(radius: Precision, points: &SingleVec) -> RTree<Point> {
    let points = points
        .iter()
        .map(|p| Point::new(radius, 20, *p))
        .collect::<Vec<_>>();
    RTree::bulk_load(points)
}

pub fn cluster_info<'a>(
    point_tree: &'a RTree<Point>,
    clusters: &'a Vec<Point>,
) -> Vec<cluster::Cluster<'a>> {
    clusters
        .par_iter()
        .map(|cluster| {
            cluster::Cluster::new(
                *cluster,
                point_tree
                    .locate_all_at_point(&cluster.center)
                    .into_iter()
                    .collect(),
                vec![],
            )
        })
        .collect::<Vec<_>>()
}
