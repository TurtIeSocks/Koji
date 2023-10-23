use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use hashbrown::HashSet;

use super::point::Point;

#[derive(Debug, Clone)]
pub struct Cluster<'a> {
    pub point: &'a Point,
    pub points: HashSet<&'a Point>,
    pub all: HashSet<&'a Point>,
}

impl<'a> Cluster<'a> {
    pub fn new<T, U>(point: &'a Point, all: T, points: U) -> Cluster<'a>
    where
        T: Iterator<Item = &'a Point>,
        U: Iterator<Item = &'a Point>,
    {
        Cluster {
            point,
            all: all.collect(),
            points: points.collect(),
        }
    }
}

impl PartialEq for Cluster<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.point.cell_id == other.point.cell_id
    }
}

impl Eq for Cluster<'_> {}

impl Hash for Cluster<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.point.cell_id.hash(state);
    }
}

impl Display for Cluster<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut display = format!("\n\n{}\nAll: {} (", self.point, self.all.len());
        for (i, point) in self.all.iter().enumerate() {
            display.push_str(&format!(
                "{}{}",
                point._get_geohash(),
                if i == self.all.len() - 1 { "" } else { ", " }
            ));
        }
        display.push_str(&format!(")\nPoints: {} (", self.points.len()));
        for (i, point) in self.points.iter().enumerate() {
            display.push_str(&format!(
                "{}{}",
                point._get_geohash(),
                if i == self.all.len() - 1 { "" } else { ", " }
            ));
        }
        display.push_str(")\n");

        write!(f, "{}", display)
    }
}
