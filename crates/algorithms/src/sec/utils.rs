use geo::{Centroid, Distance, Haversine, Point};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub fn is_missing_points(points: Vec<Point>, center: Point, radius: f64) -> bool {
    points
        .par_iter()
        .any(|p| Haversine.distance(center, *p) > radius)
}

pub fn midpoint(a: &Point, b: &Point) -> Point {
    Point::new((a.x() + b.x()) / 2., (a.y() + b.y()) / 2.)
}

pub fn smallest_three_point_circle(p1: &Point, p2: &Point, p3: &Point) -> (Point, f64) {
    let center = geo::Triangle::new(p1.0, p2.0, p3.0).centroid();
    let radius = Haversine
        .distance(center, *p1)
        .max(Haversine.distance(center, *p2))
        .max(Haversine.distance(center, *p3));
    (center, radius)
}
