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
    let mut rng = rand::thread_rng();

    for i in 0..max_attempts {
        attempt = i;

        let mut points = points.clone();
        points.shuffle(&mut rng);
        circle = smallest_enclosing_circle(points.clone(), radius);

        if let Some(center) = circle.center() {
            if !utils::is_missing_points(points, center, radius) && circle.radius() <= radius {
                break;
            }
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

    while !stack.is_empty() {
        let state = stack.pop().unwrap();
        match state {
            State::S0 => {
                if p.len() == 0 || r.len() == 3 {
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
