use geo::{HaversineDistance, Point};
use model::api::{single_vec, Precision};
use serde::Serialize;

const WIDTH: &str = "=======================================================================";

#[derive(Debug, Serialize, Clone)]
pub struct Stats {
    pub best_clusters: single_vec::SingleVec,
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

    pub fn distance(&mut self, points: &single_vec::SingleVec) {
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

    pub fn set_cluster_time(&mut self, time: Precision) {
        self.cluster_time = time;
        log::debug!("Cluster Time: {}s", self.cluster_time as Precision);
    }
}