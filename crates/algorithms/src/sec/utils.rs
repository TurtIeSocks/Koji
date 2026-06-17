use geo::{Centroid, Distance, Haversine, Point};
use koji_core::Precision;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub fn is_missing_points(points: Vec<Point>, center: Point, radius: Precision) -> bool {
    points
        .par_iter()
        .any(|p| Haversine.distance(center, *p) > radius)
}

pub fn midpoint(a: &Point, b: &Point) -> Point {
    Point::new((a.x() + b.x()) / 2., (a.y() + b.y()) / 2.)
}

pub fn smallest_three_point_circle(p1: &Point, p2: &Point, p3: &Point) -> (Point, Precision) {
    let center = geo::Triangle::new(p1.0, p2.0, p3.0).centroid();
    let radius = Haversine
        .distance(center, *p1)
        .max(Haversine.distance(center, *p2))
        .max(Haversine.distance(center, *p3));
    (center, radius)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(lon: Precision, lat: Precision) -> Point {
        Point::new(lon, lat)
    }

    #[test]
    fn midpoint_is_arithmetic_mean() {
        // midpoint() uses (a+b)/2 arithmetic averaging, not geodesic midpoint.
        // Assert the coordinates are at the arithmetic mean.
        let a = pt(0.0, 0.0);
        let b = pt(2.0, 2.0);
        let m = midpoint(&a, &b);
        assert!((m.x() - 1.0).abs() < 1e-10, "lon not at mean: {}", m.x());
        assert!((m.y() - 1.0).abs() < 1e-10, "lat not at mean: {}", m.y());
    }

    #[test]
    fn midpoint_of_nearby_points_is_close() {
        // Sub-degree points: arithmetic and geodesic midpoints are very close.
        let a = pt(-74.0, 40.0);
        let b = pt(-74.001, 40.0);
        let m = midpoint(&a, &b);
        let da = Haversine.distance(a, m);
        let db = Haversine.distance(b, m);
        // Within 1 m difference (arithmetic error on tiny segment).
        assert!(
            (da - db).abs() < 1.0,
            "midpoint not approx equidistant: {da} vs {db}"
        );
    }

    #[test]
    fn is_missing_points_returns_false_when_all_inside() {
        // Center at (0,0), points at ~50 m — well within 100 m radius.
        let center = pt(0.0, 0.0);
        let points = vec![pt(0.0001, 0.0), pt(-0.0001, 0.0), pt(0.0, 0.0001)];
        assert!(!is_missing_points(points, center, 100.0));
    }

    #[test]
    fn is_missing_points_returns_true_when_one_outside() {
        let center = pt(0.0, 0.0);
        // ~111 km away — definitely outside 100 m radius.
        let far = pt(1.0, 0.0);
        assert!(is_missing_points(vec![far], center, 100.0));
    }

    #[test]
    fn smallest_three_point_circle_radius_encloses_all() {
        // Three points at ~56 km apart; the circumscribed circle must reach all.
        let a = pt(-74.0, 40.0);
        let b = pt(-73.0, 40.0);
        let c = pt(-73.5, 41.0);
        let (center, r) = smallest_three_point_circle(&a, &b, &c);
        for p in &[a, b, c] {
            let d = Haversine.distance(center, *p);
            assert!(d <= r + 1.0, "point not enclosed: d={d}, r={r}");
        }
        assert!(r > 0.0);
    }

    #[test]
    fn smallest_three_point_circle_center_inside_triangle() {
        // Equilateral-ish triangle in Northern hemisphere — center stays inside.
        let a = pt(0.0, 0.0);
        let b = pt(0.01, 0.0);
        let c = pt(0.005, 0.01);
        let (center, _r) = smallest_three_point_circle(&a, &b, &c);
        // Center lon between min and max, same for lat.
        assert!(center.x() > -0.01 && center.x() < 0.02);
        assert!(center.y() > -0.01 && center.y() < 0.02);
    }
}
