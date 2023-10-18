use std::hash::{Hash, Hasher};

use hashbrown::HashSet;

use super::point::Point;

#[derive(Debug, Clone)]
pub struct Cluster<'a> {
    pub point: &'a Point,
    pub points: HashSet<&'a Point>,
    pub all: HashSet<&'a Point>,
}

impl<'a> Cluster<'_> {
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
