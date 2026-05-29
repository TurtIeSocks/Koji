use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

use super::{SortDedupe, point::Point};

#[derive(Debug, Clone)]
pub struct Cluster<'a> {
    pub point: Point,
    pub unique: Vec<&'a Point>,
    pub all: Vec<&'a Point>,
}

const CLUSTER_SIZE: usize = std::mem::size_of::<Cluster<'_>>();
const POINT_SIZE: usize = std::mem::size_of::<&Point>();

impl<'a> Cluster<'a> {
    pub fn new(point: Point, all: Vec<&'a Point>, unique: Vec<&'a Point>) -> Cluster<'a> {
        Cluster { point, all, unique }
    }

    pub fn get_size(&self) -> usize {
        let mut size = CLUSTER_SIZE;

        size += self.unique.capacity() * POINT_SIZE;
        size += self.all.capacity() * POINT_SIZE;

        size
    }

    pub fn set_all(&mut self, tree: &'a RTree<Point>) {
        let mut points: Vec<_> = tree
            .locate_all_at_point(&self.point.center)
            .into_iter()
            .collect();
        points.sort_dedupe();
        self.all = points;
    }

    pub fn set_unique(&mut self, tree: &RTree<Point>) {
        let mut points: Vec<_> = self
            .all
            .par_iter()
            .filter_map(|p| {
                let points = tree.locate_all_at_point(&p.center).count();
                if points == 1 { Some(*p) } else { None }
            })
            .collect();
        points.sort_dedupe();
        self.unique = points;
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
        display.push_str(&format!(")\nPoints: {} (", self.unique.len()));
        for (i, point) in self.unique.iter().enumerate() {
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
