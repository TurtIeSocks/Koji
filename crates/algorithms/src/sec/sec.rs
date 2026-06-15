use super::{circle::Circle, state::State, *};

use geo::Point;
use rand::seq::SliceRandom;

#[derive(Debug)]
pub enum SmallestEnclosingCircle {
    None,
    RadiusTooBig,
    MissingPoints,
    Centered(Point),
}

pub fn multi_attempt<I: Iterator<Item = Point>>(
    points: I,
    radius: f64,
    max_attempts: usize,
) -> SmallestEnclosingCircle {
    let points: Vec<_> = points.collect();
    let mut circle = Circle::None;
    let mut attempt = 0;
    let mut rng = rand::rng();

    for i in 0..max_attempts {
        attempt = i;

        let mut points = points.clone();
        points.shuffle(&mut rng);
        circle = smallest_enclosing_circle(points.clone(), radius);

        if let Some(center) = circle.center()
            && !utils::is_missing_points(points, center, radius)
            && circle.radius() <= radius
        {
            break;
        }
    }

    if attempt > 0 {
        log::debug!("Attempt: {}", attempt);
    }
    eval_result(points, circle, radius)
}

// pub fn single_attempt<I: Iterator<Item = Point>>(points: I, radius: f64) -> CircleResult {
//     let points: Vec<_> = points.collect();
//     let circle = smallest_enclosing_circle(points.clone(), radius);
//     eval_result(points, circle, radius)
// }

fn smallest_enclosing_circle(points: Vec<Point>, radius: f64) -> Circle {
    let mut p = points;
    let mut circle = Circle::None;
    let mut r = Vec::new();
    let mut stack = Vec::from([State::S0]);

    while let Some(state) = stack.pop() {
        match state {
            State::S0 => {
                if p.is_empty() || r.len() == 3 {
                    circle = Circle::new(&r);
                } else {
                    stack.push(State::S1);
                }
            }
            State::S1 => {
                let element = p.pop().unwrap();
                stack.push(State::S2(element));
                stack.push(State::S0);
            }
            State::S2(element) => {
                stack.push(State::S3(element));

                if !circle.contains(element, radius) {
                    r.push(element);
                    stack.push(State::S4);
                    stack.push(State::S0);
                }
            }
            State::S3(element) => {
                p.push(element);
            }
            State::S4 => {
                r.pop();
            }
        }
    }
    circle
}

fn eval_result(points: Vec<Point>, circle: Circle, radius: f64) -> SmallestEnclosingCircle {
    if let Some(center) = circle.center() {
        if circle.radius() > radius {
            SmallestEnclosingCircle::RadiusTooBig
        } else if utils::is_missing_points(points, center, radius) {
            SmallestEnclosingCircle::MissingPoints
        } else {
            SmallestEnclosingCircle::Centered(center)
        }
    } else {
        SmallestEnclosingCircle::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Distance, Haversine};

    fn pt(lon: f64, lat: f64) -> Point {
        Point::new(lon, lat)
    }

    // ── multi_attempt: empty input ─────────────────────────────────────────────

    #[test]
    fn empty_points_returns_none() {
        let result = multi_attempt(std::iter::empty(), 1000.0, 10);
        assert!(
            matches!(result, SmallestEnclosingCircle::None),
            "expected None for empty input"
        );
    }

    // ── multi_attempt: single point → Centered at that point ──────────────────

    #[test]
    fn single_point_returns_centered() {
        let p = pt(-74.0, 40.0);
        let result = multi_attempt(std::iter::once(p), 1_000.0, 10);
        match result {
            SmallestEnclosingCircle::Centered(c) => {
                assert!(
                    Haversine.distance(c, p) < 1.0,
                    "center not at input point"
                );
            }
            other => panic!("expected Centered, got {other:?}"),
        }
    }

    // ── multi_attempt: tight cluster → Centered with center enclosing all ─────

    #[test]
    fn tight_cluster_centered_encloses_all_points() {
        // 5 points within ~10 m of 40°N -74°W; radius 100 m is more than enough.
        let points: Vec<Point> = [
            (-74.0, 40.0),
            (-74.00005, 40.00005),
            (-73.99995, 40.0),
            (-74.0, 39.99995),
            (-74.00005, 39.99995),
        ]
        .iter()
        .map(|&(lon, lat)| pt(lon, lat))
        .collect();

        let result = multi_attempt(points.clone().into_iter(), 100.0, 50);
        match result {
            SmallestEnclosingCircle::Centered(center) => {
                for p in &points {
                    let d = Haversine.distance(center, *p);
                    assert!(d <= 100.0 + 1.0, "point not enclosed: d={d}");
                }
            }
            other => panic!("expected Centered, got {other:?}"),
        }
    }

    // ── multi_attempt: points spread beyond radius → RadiusTooBig or MissingPoints

    #[test]
    fn spread_points_not_centered_when_radius_too_small() {
        // Two points ~111 km apart; radius 50 m — impossible to enclose.
        let a = pt(-74.0, 40.0);
        let b = pt(-74.0, 41.0);
        let result = multi_attempt([a, b].into_iter(), 50.0, 20);
        assert!(
            matches!(
                result,
                SmallestEnclosingCircle::RadiusTooBig | SmallestEnclosingCircle::MissingPoints
            ),
            "expected failure variant for oversized spread, got {result:?}"
        );
    }

    // ── Two points just within radius → Centered ──────────────────────────────

    #[test]
    fn two_points_within_radius_centered() {
        // ~11 m apart, radius 100 m.
        let a = pt(-74.0, 40.0);
        let b = pt(-74.0001, 40.0);
        let result = multi_attempt([a, b].into_iter(), 100.0, 20);
        assert!(
            matches!(result, SmallestEnclosingCircle::Centered(_)),
            "expected Centered for nearby pair, got {result:?}"
        );
    }
}

