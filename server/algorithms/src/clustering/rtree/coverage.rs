use std::time::Instant;

use hashbrown::HashSet;
use model::api::single_vec::SingleVec;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use rstar::RTree;

use super::point::{self, Point};

pub struct ClusterStats {
    tree: RTree<Point>,
    clusters: Vec<Point>,
    radius: f64,
    point_count: usize,
}

impl ClusterStats {
    /// Creates a new ClusterStats instance.
    pub fn new(points: &SingleVec, radius: f64) -> ClusterStats {
        Self {
            tree: point::main(radius, &points),
            radius,
            point_count: points.len(),
            clusters: vec![],
        }
    }

    pub fn set_clusters(&mut self, clusters: &SingleVec) {
        self.clusters = clusters
            .into_iter()
            .map(|p| Point::new(self.radius, *p))
            .collect::<Vec<_>>();
    }

    /// Checks coverage of the points in the tree using a provided cluster
    fn get_coverage(&self, cluster: &Point) -> HashSet<&Point> {
        self.tree
            .locate_all_at_point(&cluster.center)
            .into_iter()
            .collect()
    }

    /// Checks coverage of the points in the tree across multiple clusters
    pub fn check_full_coverage(&self) -> usize {
        let time = Instant::now();
        log::info!("starting coverage check for {} points", self.point_count);

        let covered_point_count = self
            .clusters
            .par_iter()
            .map(|cluster| self.get_coverage(cluster))
            .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

        log::info!(
            "finished coverage check in {}s",
            time.elapsed().as_secs_f32()
        );

        covered_point_count.len()
    }

    pub fn get_best_clusters(&self) -> (usize, SingleVec) {
        let mut best_clusters = SingleVec::new();
        let mut best = 0;
        for cluster in self.clusters.iter() {
            let coverage = self.get_coverage(cluster);
            if coverage.len() >= best {
                best = coverage.len();
                best_clusters.push(cluster.center);
            }
        }

        (best, best_clusters)
    }
}
