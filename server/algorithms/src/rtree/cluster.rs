use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

use super::{point::Point, SortDedupe};

#[derive(Debug, Clone)]
pub struct Cluster<'a> {
    pub point: Point,
    pub points: Vec<&'a Point>,
    pub all: Vec<&'a Point>,
}

impl<'a> Cluster<'a> {
    pub fn new(point: Point, all: Vec<&'a Point>, points: Vec<&'a Point>) -> Cluster<'a> {
        Cluster { point, all, points }
    }

    pub fn get_size(&self) -> usize {
        let mut size = std::mem::size_of_val(&self);

        for point in self.point.center {
            size += std::mem::size_of_val(&point);
        }
        for point in self.points.iter() {
            size += std::mem::size_of_val(point);
        }
        for point in self.all.iter() {
            size += std::mem::size_of_val(point);
        }
        size
    }

    pub fn update_all(&mut self, tree: &'a RTree<Point>) {
        let mut points: Vec<_> = tree
            .locate_all_at_point(&self.point.center)
            .into_iter()
            .collect();
        points.sort_dedupe();
        self.all = points;
    }

    pub fn update_unique(&mut self, tree: &RTree<Point>) {
        let mut points: Vec<_> = self
            .all
            .par_iter()
            .filter_map(|p| {
                if tree.locate_all_at_point(&p.center).count() == 1 {
                    Some(*p)
                } else {
                    None
                }
            })
            .collect();
        points.sort_dedupe();
        self.points = points;
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
