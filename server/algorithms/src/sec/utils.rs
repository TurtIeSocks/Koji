use geo::{Centroid, HaversineDistance, Point};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub fn is_missing_points(points: Vec<Point>, center: Point, radius: f64) -> bool {
    points
        .par_iter()
        .any(|p| center.haversine_distance(p) > radius)
}

pub fn midpoint(a: &Point, b: &Point) -> Point {
    Point::new((a.x() + b.x()) / 2., (a.y() + b.y()) / 2.)
}

pub fn smallest_three_point_circle(p1: &Point, p2: &Point, p3: &Point) -> (Point, f64) {
    let center = geo::Triangle::new(p1.0, p2.0, p3.0).centroid();
    let radius = center
        .haversine_distance(&p1)
        .max(center.haversine_distance(&p2))
        .max(center.haversine_distance(&p3));
    (center, radius)
}
