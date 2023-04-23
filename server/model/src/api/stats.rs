use super::*;

use geo::{HaversineDistance, Point};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Stats {
    pub best_clusters: single_vec::SingleVec,
    pub best_cluster_point_count: usize,
    pub cluster_time: Precision,
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: Precision,
    pub longest_distance: Precision,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            best_clusters: vec![],
            best_cluster_point_count: 0,
            cluster_time: 0.,
            total_points: 0,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
        }
    }
    pub fn log(&self, area: Option<String>, min_points: Option<usize>) {
        let width = "=======================================================================";
        let get_row = |text: String, replace: bool| {
            format!(
                "  {}{}{}\n",
                text,
                width[..(width.len() - text.len())].replace("=", if replace { " " } else { "=" }),
                if replace { "||" } else { "==" }
            )
        };
        log::info!(
            "\n{}{}{}{}{}{}{}  {}==\n",
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
                    "|| [CLUSTERS] Time: {}s | Total: {} | Avg Points: {}",
                    self.cluster_time as f32,
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
                    "|| [DISTANCE] Total {} | Longest {} | Avg: {}",
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
                    "|| [MYGOD_SCORE] {}",
                    self.total_clusters * min_points.unwrap_or(1)
                        + (self.total_points - self.points_covered)
                ),
                true
            ),
            width,
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
