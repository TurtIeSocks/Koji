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
    pub fn new(points: &Vec<Point>) -> Self {
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
                let center = utils::midpoint(&a, &b);
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
