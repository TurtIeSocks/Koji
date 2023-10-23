pub mod cluster;
pub mod point;

use model::api::single_vec::SingleVec;
use point::Point;

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

pub fn spawn(radius: f64, points: &SingleVec) -> RTree<Point> {
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
            let points = point_tree.locate_all_at_point(&cluster.center).into_iter();
            cluster::Cluster::new(cluster, points, vec![].into_iter())
        })
        .collect::<Vec<_>>()
}
