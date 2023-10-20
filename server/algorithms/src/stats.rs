use std::time::Instant;

use geo::{HaversineDistance, Point};
use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, Precision};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;

use crate::clustering::rtree::{cluster::Cluster, point};

const WIDTH: &str = "=======================================================================";

#[derive(Debug, Serialize, Clone)]
pub struct Stats {
    pub best_clusters: SingleVec,
    pub best_cluster_point_count: usize,
    pub cluster_time: Precision,
    pub route_time: Precision,
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: Precision,
    pub longest_distance: Precision,
    pub mygod_score: usize,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            best_clusters: vec![],
            best_cluster_point_count: 0,
            cluster_time: 0.,
            route_time: 0.,
            total_points: 0,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
            mygod_score: 0,
        }
    }

    pub fn get_score(&self, min_points: usize) -> usize {
        self.total_clusters * min_points + (self.total_points - self.points_covered)
    }

    pub fn set_score(&mut self, min_points: usize) {
        self.mygod_score = self.get_score(min_points);
    }

    pub fn log(&self, area: Option<String>) {
        let get_row = |text: String, replace: bool| {
            format!(
                "  {}{}{}\n",
                text,
                WIDTH[..(WIDTH.len() - text.len())].replace("=", if replace { " " } else { "=" }),
                if replace { "||" } else { "==" }
            )
        };
        log::info!(
            "\n{}{}{}{}{}{}{}{}  {}==\n",
            get_row("[STATS] ".to_string(), false),
            if let Some(area) = area {
                if area.is_empty() {
                    "".to_string()
                } else {
                    get_row(format!("|| [AREA] {}", area), true)
                }
            } else {
                "".to_string()
            },
            get_row(
                format!(
                    "|| [POINTS] Total: {} | Covered: {}",
                    self.total_points, self.points_covered,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [CLUSTERS] Total: {} | Avg Points: {}",
                    self.total_clusters,
                    if self.total_clusters > 0 {
                        self.total_points / self.total_clusters
                    } else {
                        0
                    },
                ),
                true
            ),
            get_row(
                format!(
                    "|| [BEST_CLUSTER] Amount: {:?} | Point Count: {}",
                    self.best_clusters.len(),
                    self.best_cluster_point_count,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [DISTANCE] Total: {} | Longest: {} | Avg: {}",
                    self.total_distance as u32,
                    self.longest_distance as u32,
                    if self.total_clusters > 0 {
                        (self.total_distance / self.total_clusters as f64) as u32
                    } else {
                        0
                    },
                ),
                true
            ),
            get_row(
                format!(
                    "|| [TIMES] Clustering: {:.4} | Routing: {:.4}",
                    self.cluster_time, self.route_time,
                ),
                true
            ),
            get_row(format!("|| [MYGOD_SCORE] {}", self.mygod_score,), true),
            WIDTH,
        )
    }

    pub fn distance_stats(&mut self, points: &SingleVec) {
        self.total_distance = 0.;
        self.longest_distance = 0.;
        for (i, point) in points.iter().enumerate() {
            let point = Point::new(point[1], point[0]);
            let point2 = if i == points.len() - 1 {
                Point::new(points[0][1], points[0][0])
            } else {
                Point::new(points[i + 1][1], points[i + 1][0])
            };
            let distance = point.haversine_distance(&point2);
            self.total_distance += distance;
            if distance > self.longest_distance {
                self.longest_distance = distance;
            }
        }
    }

    pub fn set_cluster_time(&mut self, time: Instant) {
        self.cluster_time = time.elapsed().as_secs_f64();
        log::debug!("Cluster Time: {}s", self.cluster_time as Precision);
    }

    pub fn cluster_stats(&mut self, radius: f64, points: &SingleVec, clusters: &SingleVec) {
        let time = Instant::now();
        log::info!("starting coverage check for {} points", points.len());
        self.total_points = points.len();

        let tree = point::main(radius, points);
        let clusters: Vec<point::Point> = clusters
            .into_iter()
            .map(|c| point::Point::new(radius, *c))
            .collect();
        let clusters: Vec<Cluster<'_>> = clusters
            .par_iter()
            .map(|cluster| {
                let points = tree.locate_all_at_point(&cluster.center).into_iter();
                Cluster::new(cluster, points, vec![].into_iter())
            })
            .collect::<Vec<_>>();
        let mut points_covered: HashSet<&&point::Point> = HashSet::new();
        let mut best_clusters = SingleVec::new();
        let mut best = 0;

        for cluster in clusters.iter() {
            if cluster.all.len() > best {
                best_clusters.clear();
                best = cluster.all.len();
                best_clusters.push(cluster.point.center);
            } else if cluster.all.len() == best {
                best_clusters.push(cluster.point.center);
            }
            points_covered.extend(&cluster.all);
        }
        self.total_clusters = clusters.len();
        self.best_cluster_point_count = best;
        self.best_clusters = best_clusters;
        self.points_covered = points_covered.len();

        log::info!(
            "coverage check complete in {}s",
            time.elapsed().as_secs_f32()
        );
    }
}
