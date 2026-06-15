use std::{collections::HashMap, ops::AddAssign};
use web_time::Instant;

use geo::{Distance, Haversine, Point};
use hashbrown::HashSet;
use koji_core::{PointArray, Precision, SingleVec};
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use serde::{Deserialize, Serialize};

use crate::rtree::{self, cluster::Cluster, cluster_info, point};

const WIDTH: &str = "=======================================================================";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStats {
    unique: HashMap<usize, usize>,
    all: HashMap<usize, usize>,
}

impl ClusterStats {
    fn new() -> Self {
        ClusterStats {
            all: HashMap::new(),
            unique: HashMap::new(),
        }
    }

    fn increment(&mut self, unique: usize, all: usize) {
        self.all.entry(all).and_modify(|a| *a += 1).or_insert(1);
        self.unique
            .entry(unique)
            .and_modify(|a| *a += 1)
            .or_insert(1);
    }
}

/// Tiered score components (koji_score v2). The composite reproduces
/// `mygod_score` with default weights; the components let operators see WHY a
/// solution scores the way it does and compare solutions that the scalar
/// can't distinguish.
///
/// - Tier 1 (optimized): `cluster_cost + uncovered_cost` — identical to
///   `mygod_score` today (`alpha = min_points`, point weights = 1).
/// - Tier 2 (reported): `route_est_m` / `route_est_s` — estimated scan-cycle
///   travel via an S2-sorted tour (a space-filling-curve TSP approximation)
///   and an approximate cooldown curve.
/// - Tier 3 (reported): `knife_edge` — covered points with < 5% radius
///   margin, i.e. coverage that GPS jitter could break.
/// - `lb` / `quality`: provable minimum cluster count (min_points = 1 only)
///   and `lb / score` — a normalized 0..1 "how close to optimal" ratio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreComponents {
    pub cluster_cost: usize,
    pub uncovered_cost: usize,
    pub route_est_m: Precision,
    pub route_est_s: Precision,
    pub knife_edge: usize,
    /// Covered points reached by ≥ 2 cluster centers.
    pub multi_covered: usize,
    /// Total redundant coverage: Σ over covered points of (coverers − 1).
    pub overlap_excess: usize,
    pub lb: usize,
    pub quality: Precision,
}

/// Env-tunable score weight; absent/unparsable means 0 (component reported
/// but not priced).
pub fn score_lambda(key: &str) -> Precision {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<Precision>().ok())
        .unwrap_or(0.0)
}

/// Composite v2 score from components. All-zero lambdas reproduce
/// `mygod_score` exactly (tier 1 only).
pub fn compose_score_v2(
    c: &ScoreComponents,
    lambda_route: Precision,
    lambda_knife: Precision,
    lambda_overlap: Precision,
) -> usize {
    let extra = lambda_route * c.route_est_s
        + lambda_knife * c.knife_edge as Precision
        + lambda_overlap * c.overlap_excess as Precision;
    c.cluster_cost + c.uncovered_cost + extra.round().max(0.0) as usize
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    // Skipped on the wire (not part of the stats contract) AND on read — they
    // default when a `Stats` is deserialized back from a queue result. `Instant`
    // isn't `Deserialize` anyway, so `skip` (not `skip_serializing`) is required.
    #[serde(skip)]
    stats_start_time: Option<Instant>,
    #[serde(skip)]
    label: String,
    #[serde(skip)]
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
    /// Composite v2 score. With the default weights (all lambdas at 0) this
    /// equals `mygod_score`; weights are env-tunable for experiments
    /// (KOJI_SCORE_LAMBDA_ROUTE seconds-weight, KOJI_SCORE_LAMBDA_KNIFE,
    /// KOJI_SCORE_LAMBDA_OVERLAP per-excess-coverage weight).
    #[serde(default)]
    pub score_v2: usize,
    #[serde(default)]
    pub score_components: ScoreComponents,
    pub cluster_stats: ClusterStats,
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
            score_v2: 0,
            score_components: ScoreComponents::default(),
            stats_start_time: None,
            label,
            min_points,
            cluster_stats: ClusterStats::new(),
        }
    }

    pub fn get_score(&self) -> usize {
        self.total_clusters * self.min_points + (self.total_points - self.points_covered)
    }

    pub fn set_score(&mut self) {
        self.start_timer();
        self.mygod_score = self.get_score();
        // Tier-1 components (alpha = min_points, weights = 1) reproduce
        // mygod_score exactly; tier 2/3 fold in via env-tunable weights so
        // the default composite stays wire-identical to mygod_score.
        self.score_components.cluster_cost = self.total_clusters * self.min_points;
        self.score_components.uncovered_cost = self.total_points - self.points_covered;
        if self.score_components.lb > 0 && self.mygod_score > 0 {
            self.score_components.quality =
                self.score_components.lb as Precision / self.mygod_score as Precision;
        }
        self.score_v2 = compose_score_v2(
            &self.score_components,
            score_lambda("KOJI_SCORE_LAMBDA_ROUTE"),
            score_lambda("KOJI_SCORE_LAMBDA_KNIFE"),
            score_lambda("KOJI_SCORE_LAMBDA_OVERLAP"),
        );
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
            "\n{}{}{}{}{}{}{}{}{}  {}==\n",
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
                    self.points_covered
                        .checked_div(self.total_clusters)
                        .unwrap_or(0),
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
            get_row(
                format!(
                    "|| [SCORE_V2] {} | Rt: {:.0}m/{:.0}s | Knife: {} | Overlap: {}/{} | LB: {} ({:.0}%)",
                    self.score_v2,
                    self.score_components.route_est_m,
                    self.score_components.route_est_s,
                    self.score_components.knife_edge,
                    self.score_components.multi_covered,
                    self.score_components.overlap_excess,
                    self.score_components.lb,
                    self.score_components.quality * 100.0,
                ),
                true
            ),
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
            let distance = Haversine.distance(point, point2);
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
            let clusters_input = clusters;
            let tree = rtree::spawn(radius, points);
            let clusters: Vec<point::Point> = clusters
                .iter()
                .map(|c| point::Point::new(radius, 20, *c))
                .collect();
            let mut clusters: Vec<Cluster<'_>> = cluster_info(&tree, &clusters);

            let cluster_tree =
                rtree::spawn(radius, &clusters.iter().map(|c| c.point.center).collect());

            clusters
                .par_iter_mut()
                .for_each(|cluster| cluster.set_unique(&cluster_tree));

            let mut points_covered: HashSet<&point::Point> = HashSet::new();
            let mut best_clusters = SingleVec::new();
            let mut best = usize::MIN;
            let mut worst = usize::MAX;
            let mut worst_count = 0;

            // Tier-3: per covered point, (coverer count, min distance to a
            // center). Marginal coverage (< 5% radius spare) is GPS-fragile;
            // multi-coverage is wasted overlap.
            let mut cover_info: HashMap<u64, (u32, Precision)> = HashMap::new();

            for cluster in clusters.iter() {
                let length = cluster.all.len();
                if length > best {
                    best_clusters.clear();
                    best = length;
                    best_clusters.push(cluster.point.center);
                } else if length == best {
                    best_clusters.push(cluster.point.center);
                } else if length < worst {
                    worst = length;
                    worst_count = 1;
                } else if length == worst {
                    worst_count += 1;
                }
                self.cluster_stats
                    .increment(cluster.unique.len(), cluster.all.len());

                if let Some(point) = tree.locate_at_point(&cluster.point.center) {
                    points_covered.insert(point);
                }
                points_covered.extend(&cluster.all);

                let center = Point::new(cluster.point.center[1], cluster.point.center[0]);
                for p in &cluster.all {
                    let d = Haversine.distance(center, Point::new(p.center[1], p.center[0]));
                    cover_info
                        .entry(p.cell_id.0)
                        .and_modify(|(n, m)| {
                            *n += 1;
                            *m = m.min(d);
                        })
                        .or_insert((1, d));
                }
            }

            if worst == usize::MAX {
                worst = 0;
            }

            self.best_cluster_point_count = best;
            self.worst_cluster_point_count = worst;
            self.worst_cluster_count = worst_count;
            self.best_clusters = best_clusters;
            self.points_covered = points_covered.len();

            self.score_components.knife_edge = cover_info
                .values()
                .filter(|&&(_, d)| d > radius * 0.95)
                .count();
            self.score_components.multi_covered =
                cover_info.values().filter(|&&(n, _)| n >= 2).count();
            self.score_components.overlap_excess = cover_info
                .values()
                .map(|&(n, _)| (n as usize).saturating_sub(1))
                .sum();
            self.score_components.route_est_m = s2_tour_length_m(clusters_input);
            self.score_components.route_est_s = s2_tour_cooldown_s(clusters_input);
            self.score_components.lb = if self.min_points == 1 {
                independent_set_lb(points, radius)
            } else {
                0
            };

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

/// Tour-length estimate over `centers`: visit order = sort by full-depth S2
/// cell id (a space-filling curve on the sphere), sum consecutive Haversine
/// hops. Tracks true TSP length within roughly 15–25% on POI-like data —
/// good enough to compare solutions, cheap enough to run in stats.
pub fn s2_tour_length_m(centers: &SingleVec) -> Precision {
    s2_tour(centers).0
}

/// Cooldown-seconds estimate for the same tour (see [`cooldown_seconds`]).
pub fn s2_tour_cooldown_s(centers: &SingleVec) -> Precision {
    s2_tour(centers).1
}

fn s2_tour(centers: &SingleVec) -> (Precision, Precision) {
    use s2::{cellid::CellID, latlng::LatLng};
    if centers.len() < 2 {
        return (0.0, 0.0);
    }
    let mut order: Vec<(u64, &PointArray)> = centers
        .iter()
        .map(|c| (CellID::from(LatLng::from_degrees(c[0], c[1])).0, c))
        .collect();
    order.sort_unstable_by_key(|(id, _)| *id);
    let mut meters = 0.0;
    let mut seconds = 0.0;
    for pair in order.windows(2) {
        let a = pair[0].1;
        let b = pair[1].1;
        let d = Haversine.distance(Point::new(a[1], a[0]), Point::new(b[1], b[0]));
        meters += d;
        seconds += cooldown_seconds(d);
    }
    (meters, seconds)
}

/// APPROXIMATE teleport cooldown curve (community-reported, linearly
/// interpolated between anchors, capped at 2 h). Used only for the reported
/// route-cost estimate — never for coverage or score-tier-1 math.
pub fn cooldown_seconds(meters: Precision) -> Precision {
    const ANCHORS: [(Precision, Precision); 8] = [
        (0.0, 0.0),
        (1_000.0, 60.0),
        (2_000.0, 120.0),
        (4_000.0, 180.0),
        (10_000.0, 420.0),
        (30_000.0, 1_020.0),
        (100_000.0, 2_700.0),
        (500_000.0, 7_200.0),
    ];
    if meters >= ANCHORS[ANCHORS.len() - 1].0 {
        return ANCHORS[ANCHORS.len() - 1].1;
    }
    for w in ANCHORS.windows(2) {
        let (d0, s0) = w[0];
        let (d1, s1) = w[1];
        if meters <= d1 {
            return s0 + (s1 - s0) * (meters - d0) / (d1 - d0);
        }
    }
    ANCHORS[ANCHORS.len() - 1].1
}

/// Provable lower bound on the cluster count of any full-coverage solution:
/// a maximal independent set under "pairwise Haversine > 2r" — no disk can
/// cover two such points, so each needs its own cluster. Valid for
/// min_points = 1 (where full coverage is the contract).
pub fn independent_set_lb(points: &SingleVec, radius: Precision) -> usize {
    let two_r = 2.0 * radius;
    let dlat = two_r / 111_132.0;
    let mut kept: SingleVec = Vec::new();
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let key_of = |p: &PointArray| -> (i32, i32) {
        let row = (p[0] / dlat).floor() as i32;
        let lat_mid = (row as Precision + 0.5) * dlat;
        let dlon = two_r / (111_320.0 * lat_mid.to_radians().cos().abs().max(0.01));
        (row, (p[1] / dlon).floor() as i32)
    };
    'points: for p in points {
        let (row, _) = key_of(p);
        for dr in -1..=1 {
            let r2 = row + dr;
            let lat_mid = (r2 as Precision + 0.5) * dlat;
            let dlon = two_r / (111_320.0 * lat_mid.to_radians().cos().abs().max(0.01));
            let col = (p[1] / dlon).floor() as i32;
            for dc in -1..=1 {
                if let Some(bucket) = grid.get(&(r2, col + dc)) {
                    for &k in bucket {
                        let q = kept[k];
                        let d = Haversine.distance(Point::new(p[1], p[0]), Point::new(q[1], q[0]));
                        if d <= two_r {
                            continue 'points;
                        }
                    }
                }
            }
        }
        kept.push(*p);
        let key = key_of(p);
        grid.entry(key).or_default().push(kept.len() - 1);
    }
    kept.len()
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
        self.score_components.route_est_m += rhs.score_components.route_est_m;
        self.score_components.route_est_s += rhs.score_components.route_est_s;
        self.score_components.knife_edge += rhs.score_components.knife_edge;
        self.score_components.multi_covered += rhs.score_components.multi_covered;
        self.score_components.overlap_excess += rhs.score_components.overlap_excess;
        self.score_components.lb += rhs.score_components.lb;

        macro_rules! merge_stat_field {
            ($dst:expr, $src:expr, $field:ident) => {{
                $src.$field.iter().for_each(|(k, v)| {
                    $dst.$field.entry(*k).and_modify(|a| *a += v).or_insert(*v);
                });
            }};
        }

        merge_stat_field!(self.cluster_stats, rhs.cluster_stats, all);
        merge_stat_field!(self.cluster_stats, rhs.cluster_stats, unique);

        self.set_score();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_interpolates_and_caps() {
        assert_eq!(cooldown_seconds(0.0), 0.0);
        assert!((cooldown_seconds(500.0) - 30.0).abs() < 1e-9);
        assert!((cooldown_seconds(1_000.0) - 60.0).abs() < 1e-9);
        assert!((cooldown_seconds(3_000.0) - 150.0).abs() < 1e-9);
        assert_eq!(cooldown_seconds(2_000_000.0), 7_200.0);
    }

    #[test]
    fn tour_length_matches_haversine_sum() {
        // Two points ~111 km apart on a meridian: tour = one hop.
        let centers: SingleVec = vec![[40.0, -74.0], [41.0, -74.0]];
        let m = s2_tour_length_m(&centers);
        assert!((m - 111_000.0).abs() < 500.0, "got {m}");
        assert_eq!(s2_tour_length_m(&vec![[40.0, -74.0]]), 0.0);
    }

    #[test]
    fn independent_set_lb_counts_far_points() {
        // Three points pairwise much farther than 2r → LB 3; adding a point
        // within 2r of the first must not raise it.
        let pts: SingleVec = vec![
            [40.0, -74.0],
            [40.01, -74.0],
            [40.02, -74.0],
            [40.0001, -74.0],
        ];
        assert_eq!(independent_set_lb(&pts, 70.0), 3);
    }

    #[test]
    fn score_v2_defaults_to_mygod_and_components_fill() {
        let points: SingleVec = vec![
            [40.0, -74.0],
            [40.0003, -74.0], // ~33 m from first (same cluster)
            [40.1, -74.0],    // far, uncovered
        ];
        let clusters: SingleVec = vec![[40.00015, -74.0]];
        let mut stats = Stats::new("test".into(), 1);
        stats.cluster_stats(70.0, &points, &clusters);
        stats.set_score();
        assert_eq!(stats.mygod_score, stats.score_v2);
        assert_eq!(stats.score_components.cluster_cost, 1);
        assert_eq!(stats.score_components.uncovered_cost, 1);
        assert!(stats.score_components.lb >= 2);
        assert!(stats.score_components.quality > 0.0);
        // Single cluster → no tour.
        assert_eq!(stats.score_components.route_est_m, 0.0);
        // Both covered points are well inside 95% of r.
        assert_eq!(stats.score_components.knife_edge, 0);
    }

    #[test]
    fn compose_score_v2_prices_components() {
        let c = ScoreComponents {
            cluster_cost: 10,
            uncovered_cost: 2,
            route_est_s: 100.0,
            knife_edge: 4,
            overlap_excess: 6,
            ..Default::default()
        };
        // All lambdas zero → tier 1 only.
        assert_eq!(compose_score_v2(&c, 0.0, 0.0, 0.0), 12);
        // Each lambda prices its component: 12 + 0.5·100 + 2·4 + 1·6 = 76.
        assert_eq!(compose_score_v2(&c, 0.5, 2.0, 1.0), 76);
    }

    #[test]
    fn knife_edge_detects_marginal_coverage() {
        // Point ~67 m from the center (> 0.95 * 70 = 66.5 m): knife edge.
        let points: SingleVec = vec![[40.0, -74.0], [40.000602, -74.0]];
        let clusters: SingleVec = vec![[40.0, -74.0]];
        let mut stats = Stats::new("test".into(), 1);
        stats.cluster_stats(70.0, &points, &clusters);
        assert_eq!(stats.score_components.knife_edge, 1);
    }

    // ── Stats::distance_stats ─────────────────────────────────────────────────

    #[test]
    fn distance_stats_empty_is_zero() {
        let mut stats = Stats::new("test".into(), 1);
        let empty: SingleVec = vec![];
        stats.distance_stats(&empty);
        assert_eq!(stats.total_distance, 0.0);
        assert_eq!(stats.longest_distance, 0.0);
    }

    #[test]
    fn distance_stats_single_point_is_zero() {
        let mut stats = Stats::new("test".into(), 1);
        let pts: SingleVec = vec![[40.0, -74.0]];
        // single point: the "tour" goes point → itself, distance = 0.
        stats.distance_stats(&pts);
        assert_eq!(stats.total_distance, 0.0);
    }

    #[test]
    fn distance_stats_two_points_nonzero() {
        let mut stats = Stats::new("test".into(), 1);
        // ~111 km apart.
        let pts: SingleVec = vec![[40.0, -74.0], [41.0, -74.0]];
        stats.distance_stats(&pts);
        // Total round-trip distance = 2 × ~111 km.
        assert!(stats.total_distance > 100_000.0, "too small: {}", stats.total_distance);
        assert!(stats.longest_distance > 100_000.0);
    }

    // ── Stats::get_score ──────────────────────────────────────────────────────

    #[test]
    fn get_score_formula() {
        let mut stats = Stats::new("test".into(), 3);
        stats.total_clusters = 4;
        stats.total_points = 10;
        stats.points_covered = 7;
        // score = total_clusters * min_points + (total_points - points_covered)
        // = 4*3 + (10-7) = 12 + 3 = 15
        assert_eq!(stats.get_score(), 15);
    }

    // ── Stats::AddAssign ──────────────────────────────────────────────────────

    #[test]
    fn add_assign_sums_counts() {
        let mut a = Stats::new("a".into(), 1);
        a.total_points = 5;
        a.points_covered = 3;
        a.total_clusters = 2;

        let mut b = Stats::new("b".into(), 1);
        b.total_points = 7;
        b.points_covered = 4;
        b.total_clusters = 1;

        a += &b;
        assert_eq!(a.total_points, 12);
        assert_eq!(a.points_covered, 7);
        assert_eq!(a.total_clusters, 3);
    }

    #[test]
    fn add_assign_propagates_best_cluster() {
        let mut a = Stats::new("a".into(), 1);
        a.best_cluster_point_count = 2;
        a.best_clusters = vec![[40.0, -74.0]];

        let mut b = Stats::new("b".into(), 1);
        b.best_cluster_point_count = 5;
        b.best_clusters = vec![[41.0, -74.0]];

        a += &b;
        assert_eq!(a.best_cluster_point_count, 5);
        assert_eq!(a.best_clusters.len(), 1);
        assert!((a.best_clusters[0][0] - 41.0).abs() < 1e-6);
    }

    // ── s2_tour_length_m edge cases ───────────────────────────────────────────

    #[test]
    fn s2_tour_single_point_is_zero() {
        let pts: SingleVec = vec![[40.0, -74.0]];
        assert_eq!(s2_tour_length_m(&pts), 0.0);
    }

    #[test]
    fn s2_tour_empty_is_zero() {
        let empty: SingleVec = vec![];
        assert_eq!(s2_tour_length_m(&empty), 0.0);
    }

    // ── independent_set_lb ────────────────────────────────────────────────────

    #[test]
    fn lb_empty_is_zero() {
        let empty: SingleVec = vec![];
        assert_eq!(independent_set_lb(&empty, 70.0), 0);
    }

    #[test]
    fn lb_single_point_is_one() {
        let pts: SingleVec = vec![[40.0, -74.0]];
        assert_eq!(independent_set_lb(&pts, 70.0), 1);
    }

    #[test]
    fn lb_collocated_points_is_one() {
        // All at the same location → only 1 independent.
        let pts: SingleVec = vec![[40.0, -74.0]; 10];
        assert_eq!(independent_set_lb(&pts, 70.0), 1);
    }

    // ── cluster_stats: empty cluster list ─────────────────────────────────────

    #[test]
    fn cluster_stats_no_clusters_all_zeros() {
        let pts: SingleVec = vec![[40.0, -74.0]];
        let no_clusters: SingleVec = vec![];
        let mut stats = Stats::new("test".into(), 1);
        stats.cluster_stats(70.0, &pts, &no_clusters);
        assert_eq!(stats.points_covered, 0);
        assert_eq!(stats.total_clusters, 0);
    }

    // ── score_lambda: returns 0.0 for absent env var ──────────────────────────

    #[test]
    fn score_lambda_absent_returns_zero() {
        // Env var not set (or non-parseable) → 0.0.
        assert_eq!(score_lambda("__KOJI_TEST_ABSENT_VAR__"), 0.0);
    }

    // ── cooldown_seconds: all anchor endpoints are exact ──────────────────────

    #[test]
    fn cooldown_anchor_endpoints_exact() {
        // Verify every anchor pair boundary is hit exactly.
        let anchors: &[(f64, f64)] = &[
            (0.0, 0.0),
            (1_000.0, 60.0),
            (2_000.0, 120.0),
            (4_000.0, 180.0),
            (10_000.0, 420.0),
            (30_000.0, 1_020.0),
            (100_000.0, 2_700.0),
        ];
        for &(d, s) in anchors {
            assert!(
                (cooldown_seconds(d) - s).abs() < 1e-6,
                "anchor ({d}, {s}): got {}",
                cooldown_seconds(d)
            );
        }
    }

    #[test]
    fn cooldown_cap_above_max_anchor() {
        // Anything ≥ 500 000 m → capped at 7200 s.
        assert_eq!(cooldown_seconds(500_000.0), 7_200.0);
        assert_eq!(cooldown_seconds(1_000_000.0), 7_200.0);
        assert_eq!(cooldown_seconds(f64::MAX), 7_200.0);
    }

    #[test]
    fn cooldown_is_monotone() {
        // Cooldown is non-decreasing with distance.
        let distances = [0.0, 500.0, 1_000.0, 3_000.0, 7_000.0, 20_000.0, 50_000.0, 200_000.0];
        let mut prev = 0.0_f64;
        for d in distances {
            let s = cooldown_seconds(d);
            assert!(s >= prev, "cooldown decreased at {d}: {prev} -> {s}");
            prev = s;
        }
    }

    // ── s2_tour_cooldown_s mirrors tour length ────────────────────────────────

    #[test]
    fn s2_tour_cooldown_two_close_points() {
        // Two points ~1 km apart → cooldown ≈ 60 s.
        let centers: SingleVec = vec![[40.0, -74.0], [40.009, -74.0]]; // ~1 km
        let s = s2_tour_cooldown_s(&centers);
        assert!(s > 0.0 && s < 7_200.0, "expected reasonable cooldown, got {s}");
    }

    #[test]
    fn s2_tour_cooldown_empty_is_zero() {
        assert_eq!(s2_tour_cooldown_s(&vec![]), 0.0);
    }

    // ── independent_set_lb: varied radii ─────────────────────────────────────

    #[test]
    fn lb_all_within_2r_of_first_is_one() {
        // 5 points all within 2r of [0,0] → LB = 1.
        let center = [0.0_f64, 0.0_f64];
        let pts: SingleVec = vec![
            center,
            [0.0001, 0.0],   // ~11 m
            [0.0, 0.0001],
            [-0.0001, 0.0],
            [0.0, -0.0001],
        ];
        // radius 1000 m → 2r = 2000 m; all within that range.
        assert_eq!(independent_set_lb(&pts, 1_000.0), 1);
    }

    #[test]
    fn lb_two_far_points_is_two() {
        // Two points 1° (~111 km) apart, radius 70 m → LB = 2.
        let pts: SingleVec = vec![[40.0, -74.0], [41.0, -74.0]];
        assert_eq!(independent_set_lb(&pts, 70.0), 2);
    }

    #[test]
    fn lb_grows_with_dense_grid() {
        // 9 points on a 3×3 grid spaced 0.01° apart (~1.1 km).
        // Radius 70 m → 2r ≈ 140 m << 1.1 km spacing → all are independent → LB = 9.
        let pts: SingleVec = (0..3)
            .flat_map(|r| (0..3).map(move |c| [40.0 + r as f64 * 0.01, -74.0 + c as f64 * 0.01]))
            .collect();
        assert_eq!(independent_set_lb(&pts, 70.0), 9);
    }

    // ── Stats: worst_cluster tracking in cluster_stats ────────────────────────

    #[test]
    fn cluster_stats_worst_and_best_tracking() {
        // Two clusters: one covers 2 points, one covers 0 (far point) →
        // best = 2 (or 1 if the cluster center itself counts), worst = 0 or 1.
        // Use geometry: pts[0] and pts[1] are within 70 m of cluster[0];
        // pts[2] is 1 km from cluster[1] → cluster[1] covers 0 extra.
        let pts: SingleVec = vec![
            [40.0, -74.0],
            [40.0003, -74.0],   // ~33 m from cluster[0]
            [40.1, -74.0],      // ~11 km from cluster[1] — uncovered
        ];
        let clusters: SingleVec = vec![[40.00015, -74.0], [40.05, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        stats.cluster_stats(70.0, &pts, &clusters);
        assert!(
            stats.best_cluster_point_count >= 1,
            "best must be ≥1, got {}",
            stats.best_cluster_point_count
        );
        // worst cluster covers fewer points than best.
        assert!(
            stats.worst_cluster_point_count <= stats.best_cluster_point_count,
            "worst ({}) should be <= best ({})",
            stats.worst_cluster_point_count,
            stats.best_cluster_point_count
        );
    }

    // ── Stats: distance_stats with 3-point circular route ────────────────────

    #[test]
    fn distance_stats_three_points_longest_tracked() {
        // Points form a right triangle: A=(40,−74), B=(41,−74), C=(40,−75).
        // AB ≈ 111 km, AC ≈ 82 km, BC ≈ 137 km. Longest leg = BC.
        let pts: SingleVec = vec![[40.0, -74.0], [41.0, -74.0], [40.0, -75.0]];
        let mut stats = Stats::new("t".into(), 1);
        stats.distance_stats(&pts);
        // Total distance = AB + BC + CA (circular).
        assert!(
            stats.total_distance > stats.longest_distance,
            "total ({}) should exceed longest ({}) for 3+ points",
            stats.total_distance,
            stats.longest_distance
        );
        assert!(
            stats.longest_distance > 100_000.0,
            "longest leg should be > 100 km, got {}",
            stats.longest_distance
        );
    }

    // ── Stats::AddAssign: worst cluster bookkeeping ───────────────────────────

    #[test]
    fn add_assign_ties_for_best_extend_list() {
        let mut a = Stats::new("a".into(), 1);
        a.best_cluster_point_count = 5;
        a.best_clusters = vec![[40.0, -74.0]];

        let mut b = Stats::new("b".into(), 1);
        b.best_cluster_point_count = 5; // same count → extend
        b.best_clusters = vec![[41.0, -74.0]];

        a += &b;
        assert_eq!(a.best_cluster_point_count, 5);
        assert_eq!(a.best_clusters.len(), 2, "tied bests should be merged");
    }

    #[test]
    fn add_assign_lower_b_does_not_overwrite_best() {
        let mut a = Stats::new("a".into(), 1);
        a.best_cluster_point_count = 10;
        a.best_clusters = vec![[40.0, -74.0]];

        let mut b = Stats::new("b".into(), 1);
        b.best_cluster_point_count = 3;
        b.best_clusters = vec![[41.0, -74.0]];

        a += &b;
        assert_eq!(a.best_cluster_point_count, 10, "lower b should not replace best");
        assert_eq!(a.best_clusters.len(), 1);
    }

    // ── score_components: multi_covered and overlap_excess ────────────────────

    #[test]
    fn multi_covered_detected_with_overlapping_clusters() {
        // Point at origin covered by two overlapping clusters → multi_covered ≥ 1.
        let pts: SingleVec = vec![[40.0, -74.0]];
        // Two cluster centers both within 70 m of the point.
        let clusters: SingleVec = vec![[40.0, -74.0], [40.0003, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        stats.cluster_stats(70.0, &pts, &clusters);
        assert!(
            stats.score_components.multi_covered >= 1,
            "expected ≥1 multi-covered point, got {}",
            stats.score_components.multi_covered
        );
        assert!(
            stats.score_components.overlap_excess >= 1,
            "expected ≥1 overlap excess, got {}",
            stats.score_components.overlap_excess
        );
    }

    // ── s2_tour: sorted order produces consistent length ─────────────────────

    #[test]
    fn s2_tour_four_points_returns_positive_length() {
        // 4 points at corners of ~1° box.
        let pts: SingleVec = vec![
            [40.0, -74.0],
            [40.0, -73.0],
            [41.0, -73.0],
            [41.0, -74.0],
        ];
        let m = s2_tour_length_m(&pts);
        // S2-sorted tour of a ~100 km box should be in the hundreds of km range.
        assert!(m > 100_000.0, "tour length should be >100 km, got {m}");
        assert!(m < 2_000_000.0, "tour length should be <2000 km, got {m}");
    }
}

