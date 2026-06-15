use std::fmt::Display;

use geo::{Distance, Haversine, Point};

use super::*;

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Circle {
    None,
    One(Point),
    Two(Point, Point),
    Three(Point, Point, Point),
}

impl Display for Circle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Circle::None => write!(f, "None"),
            Circle::One(_) => write!(f, "One"),
            Circle::Two(_, _) => write!(f, "Two"),
            Circle::Three(_, _, _) => write!(f, "Three"),
        }
    }
}

impl Circle {
    pub fn new(points: &[Point]) -> Self {
        match points.len() {
            0 => Circle::None,
            1 => Circle::One(points[0]),
            2 => Circle::Two(points[0], points[1]),
            3 => {
                let [a, b, c] = [points[0], points[1], points[2]];
                let [ab, bc, ca] = [a == b, b == c, c == a];
                match (ab, bc, ca) {
                    (true, true, true) => Circle::One(a),
                    (true, true, false) | (true, false, true) | (false, true, true) => {
                        unreachable!()
                    }
                    (true, false, false) => Circle::Two(a, c),
                    (false, true, false) => Circle::Two(a, b),
                    (false, false, true) => Circle::Two(b, c),
                    (false, false, false) => Circle::Three(a, b, c),
                }
            }
            _ => {
                panic!()
            }
        }
    }

    pub fn contains(&self, point: Point, radius: f64) -> bool {
        match self {
            Circle::None => false,
            Circle::One(a) => a.x() == point.x() && a.y() == point.y(),
            Circle::Two(a, b) => {
                let center = utils::midpoint(a, b);
                let dis = Haversine.distance(center, point);
                dis <= radius
            }
            Circle::Three(a, b, c) => {
                let (circle, radius) = utils::smallest_three_point_circle(a, b, c);
                Haversine.distance(circle, point) <= radius
            }
        }
    }

    pub fn radius(&self) -> f64 {
        match self {
            Circle::None => 0.,
            Circle::One(_) => 0.,
            Circle::Two(a, b) => Haversine.distance(*a, *b) / 2.,
            Circle::Three(a, b, c) => utils::smallest_three_point_circle(a, b, c).1,
        }
    }

    pub fn center(&self) -> Option<Point> {
        match self {
            Circle::None => None,
            &Circle::One(a) => Some(a),
            Circle::Two(a, b) => Some(utils::midpoint(a, b)),
            Circle::Three(a, b, c) => Some(utils::smallest_three_point_circle(a, b, c).0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(lon: f64, lat: f64) -> Point {
        Point::new(lon, lat)
    }

    // ── Circle::new ────────────────────────────────────────────────────────────

    #[test]
    fn new_from_empty_is_none() {
        assert_eq!(Circle::new(&[]), Circle::None);
    }

    #[test]
    fn new_from_one_is_one() {
        let p = pt(0.0, 0.0);
        assert_eq!(Circle::new(&[p]), Circle::One(p));
    }

    #[test]
    fn new_from_two_is_two() {
        let a = pt(0.0, 0.0);
        let b = pt(1.0, 1.0);
        assert!(matches!(Circle::new(&[a, b]), Circle::Two(_, _)));
    }

    #[test]
    fn new_from_three_distinct_is_three() {
        let a = pt(0.0, 0.0);
        let b = pt(1.0, 0.0);
        let c = pt(0.5, 1.0);
        assert!(matches!(Circle::new(&[a, b, c]), Circle::Three(_, _, _)));
    }

    #[test]
    fn new_from_three_all_equal_collapses_to_one() {
        let p = pt(5.0, 5.0);
        assert!(matches!(Circle::new(&[p, p, p]), Circle::One(_)));
    }

    #[test]
    fn new_from_three_first_two_equal_is_two() {
        let a = pt(0.0, 0.0);
        let b = pt(1.0, 1.0);
        // a==a, c==b → Circle::Two(a, b)
        assert!(matches!(Circle::new(&[a, a, b]), Circle::Two(_, _)));
    }

    // ── Circle::radius ─────────────────────────────────────────────────────────

    #[test]
    fn radius_none_is_zero() {
        assert_eq!(Circle::None.radius(), 0.0);
    }

    #[test]
    fn radius_one_is_zero() {
        assert_eq!(Circle::One(pt(0.0, 0.0)).radius(), 0.0);
    }

    #[test]
    fn radius_two_is_half_haversine() {
        // Two points ~111 km apart on a meridian → radius ≈ 55.5 km.
        let a = pt(-74.0, 40.0);
        let b = pt(-74.0, 41.0);
        let r = Circle::Two(a, b).radius();
        assert!((r - 55_500.0).abs() < 500.0, "got {r}");
    }

    #[test]
    fn radius_three_is_positive() {
        let a = pt(0.0, 0.0);
        let b = pt(0.01, 0.0);
        let c = pt(0.005, 0.01);
        let r = Circle::Three(a, b, c).radius();
        assert!(r > 0.0);
    }

    // ── Circle::center ─────────────────────────────────────────────────────────

    #[test]
    fn center_none_is_none() {
        assert!(Circle::None.center().is_none());
    }

    #[test]
    fn center_one_returns_that_point() {
        let p = pt(12.34, 56.78);
        let c = Circle::One(p).center().unwrap();
        assert!((c.x() - 12.34).abs() < 1e-10);
        assert!((c.y() - 56.78).abs() < 1e-10);
    }

    #[test]
    fn center_two_is_midpoint() {
        let a = pt(0.0, 0.0);
        let b = pt(2.0, 2.0);
        let c = Circle::Two(a, b).center().unwrap();
        assert!((c.x() - 1.0).abs() < 1e-6);
        assert!((c.y() - 1.0).abs() < 1e-6);
    }

    // ── Circle::contains ───────────────────────────────────────────────────────

    #[test]
    fn contains_none_always_false() {
        assert!(!Circle::None.contains(pt(0.0, 0.0), 1_000.0));
    }

    #[test]
    fn contains_one_only_exact() {
        let p = pt(10.0, 20.0);
        assert!(Circle::One(p).contains(p, 1_000_000.0));
        assert!(!Circle::One(p).contains(pt(10.0001, 20.0), 1_000_000.0));
    }

    #[test]
    fn contains_two_within_radius() {
        // Midpoint of a and b with radius > half-distance → contains midpoint.
        let a = pt(-74.0, 40.0);
        let b = pt(-74.0, 41.0);
        let mid = pt(-74.0, 40.5);
        let r = Circle::Two(a, b).radius();
        assert!(Circle::Two(a, b).contains(mid, r + 1.0));
    }

    // ── Display ────────────────────────────────────────────────────────────────

    #[test]
    fn display_variants() {
        assert_eq!(format!("{}", Circle::None), "None");
        assert_eq!(format!("{}", Circle::One(pt(0.0, 0.0))), "One");
        assert!(format!("{}", Circle::Two(pt(0.0, 0.0), pt(1.0, 1.0))).contains("Two"));
    }
}
