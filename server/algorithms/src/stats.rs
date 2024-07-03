use std::{ops::AddAssign, time::Instant};

use geo::{HaversineDistance, Point};
use hashbrown::HashSet;
use model::api::{single_vec::SingleVec, Precision};
use serde::{ser::SerializeStruct, Serialize};

use crate::rtree::{self, cluster::Cluster, cluster_info, point};

const WIDTH: &str = "=======================================================================";

#[derive(Debug, Clone)]
pub struct Stats {
    stats_start_time: Option<Instant>,
    label: String,
    min_points: usize,

    pub best_clusters: SingleVec,
    pub best_cluster_point_count: usize,
    pub worst_cluster_point_count: usize,
    pub worst_cluster_count: usize,
    pub cluster_time: Precision,
    pub route_time: Precision,
    pub stats_time: Precision,
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub total_distance: Precision,
    pub longest_distance: Precision,
    pub mygod_score: usize,
}

impl Stats {
    pub fn new(label: String, min_points: usize) -> Self {
        Self {
            best_clusters: vec![],
            best_cluster_point_count: 0,
            worst_cluster_point_count: 0,
            worst_cluster_count: 0,
            cluster_time: 0.,
            route_time: 0.,
            stats_time: 0.,
            total_points: 0,
            points_covered: 0,
            total_clusters: 0,
            total_distance: 0.,
            longest_distance: 0.,
            mygod_score: 0,
            stats_start_time: None,
            label,
            min_points,
        }
    }

    pub fn get_score(&self) -> usize {
        self.total_clusters * self.min_points + (self.total_points - self.points_covered)
    }

    pub fn set_score(&mut self) {
        self.start_timer();
        self.mygod_score = self.get_score();
        self.stop_timer();
    }

    pub fn log(&self, area: Option<String>) {
        let get_row = |text: String, replace: bool| {
            let safe_text: String = if text.len() > WIDTH.len() - 4 {
                text[..(WIDTH.len() - 4)].to_string()
            } else {
                text
            };
            format!(
                "  {}{}{}\n",
                safe_text,
                WIDTH[..(WIDTH.len() - safe_text.len())]
                    .replace("=", if replace { " " } else { "=" }),
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
                    get_row(format!("|| [AREA] {} | {}", area, self.label), true)
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
                        self.points_covered / self.total_clusters
                    } else {
                        0
                    },
                ),
                true
            ),
            get_row(
                format!(
                    "|| [COVERAGE] Best: {} ({}) | Worst: {} ({})",
                    self.best_cluster_point_count,
                    self.best_clusters.len(),
                    self.worst_cluster_point_count,
                    self.worst_cluster_count,
                ),
                true
            ),
            get_row(
                format!(
                    "|| [DISTANCE] Total: {}m | Longest: {}m | Avg: {}m",
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
                    "|| [TIMES] Clustering: {:.2}s | Routing: {:.2}s | Stats: {:.2}s",
                    self.cluster_time, self.route_time, self.stats_time,
                ),
                true
            ),
            get_row(format!("|| [MYGOD_SCORE] {}", self.mygod_score,), true),
            WIDTH,
        )
    }

    pub fn distance_stats(&mut self, clusters: &SingleVec) {
        self.start_timer();
        log::info!("generating distance stats for {} points", clusters.len());
        self.total_distance = 0.;
        self.longest_distance = 0.;
        for (i, point) in clusters.iter().enumerate() {
            let point = Point::new(point[1], point[0]);
            let point2 = if i == clusters.len() - 1 {
                Point::new(clusters[0][1], clusters[0][0])
            } else {
                Point::new(clusters[i + 1][1], clusters[i + 1][0])
            };
            let distance = point.haversine_distance(&point2);
            self.total_distance += distance;
            if distance > self.longest_distance {
                self.longest_distance = distance;
            }
        }
        log::info!(
            "distance stats complete {:.4}s",
            self.stats_start_time.unwrap().elapsed().as_secs_f32()
        );
        self.stop_timer();
    }

    pub fn set_cluster_time(&mut self, time: Instant) {
        self.cluster_time = time.elapsed().as_secs_f64();
        log::debug!("Cluster Time: {}s", self.cluster_time as Precision);
    }

    pub fn set_route_time(&mut self, time: Instant) {
        self.route_time = time.elapsed().as_secs_f64();
        log::debug!("Route Time: {}s", self.route_time as Precision);
    }

    fn start_timer(&mut self) {
        self.stats_start_time = Some(Instant::now());
    }

    fn stop_timer(&mut self) {
        if let Some(timer) = self.stats_start_time {
            self.stats_time += timer.elapsed().as_secs_f64();
            self.stats_start_time = None;
            log::debug!("Stats Time: {}s", self.stats_time as Precision);
        }
    }

    pub fn cluster_stats(&mut self, radius: Precision, points: &SingleVec, clusters: &SingleVec) {
        self.start_timer();
        log::info!("starting coverage check for {} points", points.len());
        self.total_points = points.len();
        self.total_clusters = clusters.len();

        if points.is_empty() {
        } else {
            let tree = rtree::spawn(radius, points);
            let clusters: Vec<point::Point> = clusters
                .into_iter()
                .map(|c| point::Point::new(radius, 20, *c))
                .collect();
            let clusters: Vec<Cluster<'_>> = cluster_info(&tree, &clusters);
            let mut points_covered: HashSet<&point::Point> = HashSet::new();
            let mut best_clusters = SingleVec::new();
            let mut best = usize::MIN;
            let mut worst = usize::MAX;
            let mut worst_count = 0;

            for cluster in clusters.iter() {
                if cluster.all.len() > best {
                    best_clusters.clear();
                    best = cluster.all.len();
                    best_clusters.push(cluster.point.center);
                } else if cluster.all.len() == best {
                    best_clusters.push(cluster.point.center);
                }
                if cluster.all.len() < worst {
                    worst = cluster.all.len();
                    worst_count = 1;
                } else if cluster.all.len() == worst {
                    worst_count += 1;
                }
                if let Some(point) = tree.locate_at_point(&cluster.point.center) {
                    points_covered.insert(point);
                }
                points_covered.extend(&cluster.all);
            }
            if worst == usize::MAX {
                worst = 0;
            }

            self.best_cluster_point_count = best;
            self.worst_cluster_point_count = worst;
            self.worst_cluster_count = worst_count;
            self.best_clusters = best_clusters;
            self.points_covered = points_covered.len();

            if self.points_covered > self.total_points {
                log::warn!(
                    "points covered ({}) is greater than total points ({}), please report this to the developers",
                    self.points_covered,
                    self.total_points
                );
            }
        }
        log::info!(
            "coverage check complete in {:.4}s",
            self.stats_start_time.unwrap().elapsed().as_secs_f32()
        );
        self.stop_timer();
    }
}

impl Serialize for Stats {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Stats", 11)?;
        state.serialize_field("best_clusters", &self.best_clusters)?;
        state.serialize_field("best_cluster_point_count", &self.best_cluster_point_count)?;
        state.serialize_field("worst_cluster_point_count", &self.worst_cluster_point_count)?;
        state.serialize_field("cluster_time", &self.cluster_time)?;
        state.serialize_field("route_time", &self.route_time)?;
        state.serialize_field("stats_time", &self.stats_time)?;
        state.serialize_field("total_points", &self.total_points)?;
        state.serialize_field("points_covered", &self.points_covered)?;
        state.serialize_field("total_clusters", &self.total_clusters)?;
        state.serialize_field("total_distance", &self.total_distance)?;
        state.serialize_field("longest_distance", &self.longest_distance)?;
        state.serialize_field("mygod_score", &self.mygod_score)?;
        state.end()
    }
}

impl<'a> AddAssign<&'a Self> for Stats {
    fn add_assign(&mut self, rhs: &'a Self) {
        if self.best_cluster_point_count < rhs.best_cluster_point_count {
            self.best_clusters = rhs.best_clusters.clone();
            self.best_cluster_point_count = rhs.best_cluster_point_count;
        } else if self.best_cluster_point_count == rhs.best_cluster_point_count {
            self.best_clusters.extend(rhs.best_clusters.clone());
        }
        self.cluster_time += rhs.cluster_time;
        self.route_time += rhs.route_time;
        self.stats_time += rhs.stats_time;
        self.total_points += rhs.total_points;
        self.points_covered += rhs.points_covered;
        self.total_clusters += rhs.total_clusters;
        self.total_distance += rhs.total_distance;
        self.longest_distance += rhs.longest_distance;
        self.set_score();
    }
}
