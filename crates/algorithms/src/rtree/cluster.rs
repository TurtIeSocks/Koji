#[cfg(test)]
use koji_core::Precision;
use std::hash::{Hash, Hasher};

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

    pub fn set_unique(&mut self, tree: &RTree<Point>) {
        let mut points: Vec<_> = self
            .all
            .par_iter()
            .filter_map(|p| {
                let points = tree.locate_all_at_point(p.center).count();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtree;

    // ── spawn + cluster_info: basic query correctness ─────────────────────────

    #[test]
    fn spawn_finds_nearby_point() {
        let points: Vec<[Precision; 2]> = vec![[40.0, -74.0], [41.0, -74.0]];
        let tree = rtree::spawn(1_000.0, &points);
        // Query at the first point — must find it (radius = 1 km > 0).
        let found = tree.locate_at_point([40.0, -74.0]);
        assert!(found.is_some(), "should find the exact point");
    }

    #[test]
    fn spawn_does_not_find_far_point() {
        let points: Vec<[Precision; 2]> = vec![[40.0, -74.0]];
        let tree = rtree::spawn(10.0, &points); // 10 m radius
        // ~111 km away — must not be found.
        let found = tree.locate_at_point([41.0, -74.0]);
        assert!(found.is_none(), "far point must not be within 10 m radius");
    }

    #[test]
    fn cluster_info_all_field_populated() {
        let data: Vec<[Precision; 2]> = vec![[40.0, -74.0], [40.0001, -74.0]];
        let radius = 1_000.0; // 1 km — both points well within range.
        let tree = rtree::spawn(radius, &data);
        let cluster_centers: Vec<Point> = vec![Point::new(radius, 20, [40.0, -74.0])];
        let clusters = rtree::cluster_info(&tree, &cluster_centers);
        assert_eq!(clusters.len(), 1);
        // Both data points are within 1 km of 40°N -74°W.
        assert!(
            !clusters[0].all.is_empty(),
            "expected at least the exact point"
        );
    }

    // ── Cluster::new / get_size ───────────────────────────────────────────────

    #[test]
    fn cluster_new_stores_fields() {
        let center = [40.0_f64, -74.0_f64];
        let cp = Point::new(70.0, 20, center);
        let p1 = Point::new(70.0, 20, [40.0001, -74.0]);
        let all: Vec<&Point> = vec![&p1];
        let c = Cluster::new(cp, all.clone(), vec![]);
        assert_eq!(c.all.len(), 1);
        assert!(c.unique.is_empty());
        assert!(c.get_size() > 0);
    }

    // ── Cluster equality / hash: cell_id-based ────────────────────────────────

    #[test]
    fn clusters_with_same_center_are_equal() {
        let cp = Point::new(70.0, 20, [40.0, -74.0]);
        let c1 = Cluster::new(cp, vec![], vec![]);
        let c2 = Cluster::new(cp, vec![], vec![]);
        assert_eq!(c1, c2);
    }

    #[test]
    fn clusters_with_different_centers_are_not_equal() {
        let cp1 = Point::new(70.0, 20, [40.0, -74.0]);
        let cp2 = Point::new(70.0, 20, [41.0, -74.0]);
        let c1 = Cluster::new(cp1, vec![], vec![]);
        let c2 = Cluster::new(cp2, vec![], vec![]);
        assert_ne!(c1, c2);
    }
}
