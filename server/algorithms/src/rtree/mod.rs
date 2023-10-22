pub mod cluster;
pub mod point;

use model::api::single_vec::SingleVec;
use point::Point;

use rstar::RTree;

pub fn spawn(radius: f64, points: &SingleVec) -> RTree<Point> {
    let points = points
        .iter()
        .map(|p| Point::new(radius, 20, *p))
        .collect::<Vec<_>>();
    RTree::bulk_load(points)
}
