use geojson::{Feature, Geometry};
use hashbrown::HashSet;
use koji_core::{KojiBbox, Precision, SingleVec};
use macros::time;

use super::ClusterMode;

use ::s2::{cellid::CellID, latlng::LatLng};
use rayon::{
    iter::IntoParallelRefMutIterator,
    prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use rstar::RTree;
#[cfg(feature = "native")]
use std::io::Write;
#[cfg(feature = "native")]
use sysinfo::System;
use web_time::Instant;

use crate::{
    bootstrap::radius,
    clustering::{
        candidates,
        partition::BYTE,
        rtree::{cluster::Cluster, point::Point},
    },
    rtree::{self, SortDedupe},
    utils,
};

pub struct Greedy {
    cluster_mode: ClusterMode,
    max_clusters: usize,
    min_points: usize,
    radius: Precision,
}

impl Default for Greedy {
    fn default() -> Self {
        Greedy {
            cluster_mode: ClusterMode::Balanced,
            max_clusters: usize::MAX,
            min_points: 1,
            radius: 70.,
        }
    }
}

impl<'a> Greedy {
    pub fn set_cluster_mode(&mut self, cluster_mode: ClusterMode) -> &mut Self {
        self.cluster_mode = cluster_mode;
        self
    }
    pub fn set_radius(&mut self, radius: Precision) -> &mut Self {
        self.radius = radius;
        self
    }
    pub fn set_max_clusters(&mut self, max_clusters: usize) -> &mut Self {
        self.max_clusters = max_clusters;
        self
    }
    pub fn set_min_points(&mut self, min_points: usize) -> &mut Self {
        self.min_points = min_points;
        self
    }

    pub fn run(&'a self, points: &SingleVec) -> SingleVec {
        let time = Instant::now();
        log::info!("starting algorithm with {} data points", points.len());
        let return_set = self.setup(points);
        log::info!("finished in {:.2}s", time.elapsed().as_secs_f32());
        return_set.into_iter().map(|p| p.center).collect()
    }

    /// Add clusters centered on previously-uncovered points when a single cluster
    /// at that point would cover MORE than `min_points` other uncovered points.
    ///
    /// mygod_score math: adding a cluster covering `k` previously-uncovered
    /// points changes (clusters * min_points + uncovered) by
    ///   +min_points (one more cluster) - k (covered points removed from uncovered)
    /// Net negative iff k > min_points. We use strict `>` so we only add when
    /// it's a clear win; `=` would be neutral.
    ///
    /// Processes uncovered points in arbitrary order (rayon par_iter is unsafe
    /// here because each addition affects later points' "still uncovered" set);
    /// future work could sort by local density to maximize coverage per cluster.
    fn fill_coverage_gaps(
        &self,
        mut solution: Vec<Cluster<'a>>,
        all_points: &SingleVec,
        all_points_tree: &'a RTree<Point>,
    ) -> Vec<Cluster<'a>> {
        if all_points.is_empty() || self.min_points == 0 {
            return solution;
        }

        // Build a tree of current cluster centers so we can ask "is point p
        // already covered?" (within radius of some cluster center).
        let center_coords: SingleVec = solution.iter().map(|c| c.point.center).collect();
        let center_tree: RTree<Point> = crate::rtree::spawn(self.radius, &center_coords);

        // Initial set of uncovered point cell_ids (level 20, dedupes nearby
        // duplicates the way the rest of the algorithm does).
        let mut still_uncovered: HashSet<::s2::cellid::CellID> = all_points
            .iter()
            .filter_map(|p| {
                if center_tree.locate_at_point(p).is_some() {
                    None
                } else {
                    let cell_id =
                        ::s2::cellid::CellID::from(::s2::latlng::LatLng::from_degrees(p[0], p[1]))
                            .parent(20);
                    Some(cell_id)
                }
            })
            .collect();

        if still_uncovered.is_empty() {
            return solution;
        }

        let initial_uncovered = still_uncovered.len();
        let mut additions: Vec<Cluster<'a>> = Vec::new();

        // Iterate input order; for each point still uncovered, try as candidate.
        // Sort-by-local-density would do better but adds O(n log n) over each
        // uncovered point; this single-pass version is the cheap baseline.
        for p in all_points {
            let cell_id =
                ::s2::cellid::CellID::from(::s2::latlng::LatLng::from_degrees(p[0], p[1]))
                    .parent(20);
            if !still_uncovered.contains(&cell_id) {
                continue;
            }

            // Find points within self.radius of `p` that are STILL uncovered.
            let candidates: Vec<&Point> = all_points_tree
                .locate_all_at_point(p)
                .filter(|q| still_uncovered.contains(&q.cell_id))
                .collect();

            if candidates.len() > self.min_points {
                // Strict mygod_score win. Place cluster, mark covered.
                let new_pt = Point::new(self.radius, 20, *p);
                let mut all_vec: Vec<&Point> = candidates;
                all_vec.sort_dedupe();
                for q in &all_vec {
                    still_uncovered.remove(&q.cell_id);
                }
                additions.push(Cluster::new(new_pt, all_vec, vec![]));
            }
        }

        log::info!(
            "fill_coverage_gaps: {} uncovered -> +{} new clusters ({} still uncovered)",
            initial_uncovered,
            additions.len(),
            still_uncovered.len(),
        );

        solution.extend(additions);
        // No update_unique here; callers will run check_missing / final conversion.
        // The added clusters' unique = all (no overlap with existing since they
        // covered uncovered points by construction); other clusters' unique is
        // unchanged for the same reason.
        solution
    }

    /// Build the set of single-point clusters needed to cover any input points
    /// that the greedy pass never put inside a cluster. Used only when
    /// `min_points == 1`, where the contract is "every input point must end up
    /// covered".
    ///
    /// `max_to_add` is the cap on how many new single-point clusters this fn
    /// is allowed to emit. Callers pass `self.max_clusters.saturating_sub(
    /// current_solution_size)` so the missing-recovery pass cannot push the
    /// final solution past the user's `max_clusters` ceiling. `0` is the
    /// fast path: cap reached, nothing to do.
    ///
    /// When the unfiltered set of missing points exceeds the cap, the result
    /// is truncated. Truncation keeps the first N points in input order
    /// (deterministic for deterministic inputs); we don't reorder by local
    /// density today.
    fn recover_missing_points(
        &self,
        seen_cell_ids: &HashSet<CellID>,
        points: &SingleVec,
        max_to_add: usize,
    ) -> Vec<Point> {
        if max_to_add == 0 || seen_cell_ids.len() == points.len() {
            return vec![];
        }
        let mut missing: Vec<Point> = points
            .into_par_iter()
            .filter_map(|p| {
                let cell_id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                if seen_cell_ids.contains(&cell_id) {
                    None
                } else {
                    Some(Point::new(self.radius, 20, *p))
                }
            })
            .collect();
        if missing.len() > max_to_add {
            missing.truncate(max_to_add);
        }
        missing
    }

    fn bucket_clusters_by_size(
        &self,
        clusters_with_data: Vec<Cluster<'a>>,
    ) -> Vec<Vec<Cluster<'a>>> {
        let max = clusters_with_data
            .iter()
            .map(|cluster| cluster.all.len())
            .max()
            .unwrap_or(0);
        let mut clustered_clusters = vec![vec![]; max + 1];
        for cluster in clusters_with_data.into_iter() {
            clustered_clusters[cluster.all.len()].push(cluster);
        }
        clustered_clusters
    }

    fn get_honeycomb_clusters(&self, points: &SingleVec) -> SingleVec {
        let kb = KojiBbox::from_points(points)
            .map(|b| b.trim(6))
            .unwrap_or(KojiBbox {
                min_lat: 0.0,
                min_lon: 0.0,
                max_lat: 0.0,
                max_lon: 0.0,
            });
        let arr = kb.to_geojson_bbox(); // [min_lon, min_lat, max_lon, max_lat]
        let bbox = Some(kb.to_geojson_bbox_vec());

        let feat = Feature {
            bbox: bbox.clone(),
            geometry: Some(Geometry {
                bbox,
                foreign_members: None,
                value: geojson::Value::Polygon(vec![vec![
                    vec![arr[0], arr[1]],
                    vec![arr[2], arr[1]],
                    vec![arr[2], arr[3]],
                    vec![arr[0], arr[3]],
                    vec![arr[0], arr[1]],
                ]]),
            }),
            ..Default::default()
        };
        radius::BootstrapRadius::new(&feat, self.radius).result()
    }

    fn gen_clusters(&self, density: usize, points: &'a SingleVec) -> SingleVec {
        candidates::generate_clusters_from_points(points, self.radius, density)
    }

    fn generate_candidates_for_mode(&self, points: &'a SingleVec, mode: ClusterMode) -> SingleVec {
        match mode {
            ClusterMode::Honeycomb => self.get_honeycomb_clusters(points),
            ClusterMode::Fast => self.gen_clusters(BYTE / 2, points),
            ClusterMode::Balanced => self.gen_clusters(BYTE, points),
            _ => vec![],
        }
    }

    fn associate_clusters(
        &'a self,
        points: &'a SingleVec,
        point_tree: &'a RTree<Point>,
    ) -> Vec<Vec<Cluster<'a>>> {
        #[cfg(feature = "native")]
        let sys_mem = {
            let sys = System::new_all();
            sys.available_memory() as usize / BYTE / BYTE
        };
        #[cfg(not(feature = "native"))]
        let sys_mem: usize = 512; // MB — conservative wasm baseline

        let time = Instant::now();
        let clusters_with_data: Vec<Cluster> = self
            .generate_candidates_for_mode(points, self.cluster_mode.clone())
            .into_par_iter()
            .filter_map(|cluster| {
                let iter = point_tree.locate_all_at_point(&cluster);
                let mut points = Vec::with_capacity(iter.size_hint().0);
                points.extend(iter);

                (points.len() >= self.min_points)
                    .then(|| Cluster::new(Point::new(self.radius, 20, cluster), points, vec![]))
            })
            .collect();

        log::info!(
            "associated points with {} clusters in {:.2}s",
            clusters_with_data.len(),
            time.elapsed().as_secs_f32(),
        );

        let size = (clusters_with_data
            .iter()
            .map(|cluster| cluster.get_size())
            .sum::<usize>()
            / BYTE
            / BYTE)
            * 2;
        if size > sys_mem {
            let (size, label) = if size > BYTE {
                (size as f32 / BYTE as f32, "GB")
            } else {
                (size as f32, "MB")
            };
            log::warn!(
                "Kōji is taking a lot of memory ({:.2}{label}), I hope you know what you're doing! If you're getting this warning, try sending smaller areas or not using the `Better` algorithm.",
                size,
            );
        }

        self.bucket_clusters_by_size(clusters_with_data)
    }

    fn setup(&'a self, points: &SingleVec) -> HashSet<Point> {
        let time = Instant::now();
        let point_tree: RTree<Point> = rtree::spawn(self.radius, points);
        log::info!("created point tree in {:.2}s", time.elapsed().as_secs_f32());

        let clusters_with_data = self.associate_clusters(points, &point_tree);

        let mut solution: Vec<Cluster> = self.cluster(&clusters_with_data).into_iter().collect();
        self.update_unique(&mut solution);

        // Post-greedy gap-fill pass: greedy stops when no candidate covers
        // min_points unique points. Many points remain uncovered. Adding a new
        // cluster centered on an uncovered point that covers ≥ min_points OTHER
        // uncovered points is a strict mygod_score win (covers ≥ min_points+1,
        // costs min_points -> net negative). See fill_coverage_gaps for math.
        let solution = self.fill_coverage_gaps(solution, points, &point_tree);

        if self.min_points == 1 {
            self.check_missing(solution, points)
        } else {
            solution.into_iter().map(|c| c.into()).collect()
        }
    }

    #[time()]
    fn cluster(&'a self, clusters_with_data: &'a Vec<Vec<Cluster<'a>>>) -> HashSet<Cluster<'a>> {
        let mut new_clusters = HashSet::<Cluster>::new();
        let mut blocked_points = HashSet::<&Point>::new();

        // No candidate cluster is large enough to seed the greedy pass (e.g. data too
        // sparse for any radius-disc to hold `min_points`): nothing to select. Also guards
        // the `current - min_points` subtraction below against unsigned underflow.
        if clusters_with_data.len() <= self.min_points {
            return new_clusters;
        }
        let mut current = clusters_with_data.len() - 1;
        let total_iterations = current - self.min_points + 1;
        let mut current_iteration = 0;
        #[cfg(feature = "native")]
        let mut stdout = std::io::stdout();

        let capacity = clusters_with_data.iter().map(|c| c.len()).sum::<usize>();
        let mut clusters_of_interest: Vec<&Cluster<'_>> = Vec::with_capacity(capacity);

        'greedy: while current >= self.min_points && new_clusters.len() < self.max_clusters {
            current_iteration += 1;
            clusters_of_interest.clear();
            clusters_of_interest.extend(clusters_with_data[current..].iter().flatten());

            let mut local_clusters = clusters_of_interest
                .par_iter()
                .filter_map(|cluster| {
                    let mut points: Vec<&Point> = cluster
                        .all
                        .iter()
                        .filter_map(|p| {
                            if blocked_points.contains(p) {
                                None
                            } else {
                                Some(*p)
                            }
                        })
                        .collect();
                    if points.len() < current {
                        None
                    } else {
                        points.sort_dedupe();

                        Some(Cluster {
                            point: cluster.point,
                            unique: points.into_iter().collect(),
                            all: cluster.all.to_vec(),
                        })
                    }
                })
                .collect::<Vec<Cluster>>();

            if local_clusters.is_empty() {
                current -= 1;
                continue;
            }

            local_clusters.par_sort_by(|a, b| {
                if a.unique.len() == b.unique.len() {
                    b.all.len().cmp(&a.all.len())
                } else {
                    b.unique.len().cmp(&a.unique.len())
                }
            });

            'cluster: for cluster in local_clusters.into_iter() {
                if new_clusters.len() >= self.max_clusters {
                    break 'greedy;
                }
                if cluster.unique.len() >= current {
                    for point in cluster.unique.iter() {
                        if blocked_points.contains(point) {
                            continue 'cluster;
                        }
                    }
                    for point in cluster.unique.iter() {
                        blocked_points.insert(point);
                    }
                    new_clusters.insert(cluster);
                }
            }

            #[cfg(feature = "native")]
            if current >= self.min_points {
                stdout
                    .write_all(
                        utils::info_log(
                            "algorithms::clustering::greedy",
                            format!(
                                "Progress: {:.2}% | Clusters: {}",
                                (current_iteration as f32 / total_iterations as f32) * 100.,
                                new_clusters.len()
                            ),
                        )
                        .as_bytes(),
                    )
                    .unwrap();
                stdout.flush().unwrap();
            }

            current -= 1;
        }
        #[cfg(feature = "native")]
        stdout.write_all(b"\n").unwrap();

        log::info!("initial solution size: {}", new_clusters.len());

        new_clusters
    }

    #[time()]
    fn update_unique(&'a self, clusters: &mut Vec<Cluster>) {
        let cluster_tree = rtree::spawn(
            self.radius,
            &clusters.iter().map(|c| c.point.center).collect(),
        );

        clusters
            .par_iter_mut()
            .for_each(|cluster| cluster.set_unique(&cluster_tree));

        clusters.retain(|cluster| cluster.unique.len() >= self.min_points);

        log::info!("unique solution size: {}", clusters.len());
    }

    #[time()]
    fn check_missing(&self, clusters: Vec<Cluster>, points: &SingleVec) -> HashSet<Point> {
        let mut result: HashSet<Point> = clusters.iter().map(|c| c.point).collect();
        // Respect `max_clusters`: never let missing-point recovery push the
        // final solution above the cap. If greedy already hit the cap, skip
        // the check entirely (cap == 0).
        let cap = self.max_clusters.saturating_sub(result.len());
        if cap > 0 {
            let seen_cell_ids: HashSet<CellID> = clusters
                .iter()
                .flat_map(|c| c.all.iter())
                .map(|p| p.cell_id)
                .collect();
            let missing = self.recover_missing_points(&seen_cell_ids, points, cap);
            result.extend(missing);
        }

        log::info!("final solution size: {}", result.len());
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clustering::rtree::{cluster::Cluster, point::Point};
    use ::s2::cellid::CellID;

    /// Regression: when association yields zero candidate clusters (data too sparse for any
    /// radius-disc to hold `min_points`), `bucket_clusters_by_size` returns a single empty
    /// size-bucket, so `current` is 0. The greedy loop's `current - min_points + 1`
    /// previously underflowed (debug panic / release wraparound); it must now return empty.
    #[test]
    fn cluster_with_no_viable_candidates_returns_empty() {
        let greedy = Greedy::default(); // min_points = 1
        let empty: Vec<Vec<Cluster>> = vec![vec![]];
        assert!(greedy.cluster(&empty).is_empty());
    }

    #[test]
    fn bucket_clusters_by_size_indexes_by_all_len() {
        let pts: Vec<Point> = (0..3)
            .map(|i| Point::new(70.0, 20, [40.0 + i as f64 * 0.01, -74.0]))
            .collect();
        let center = Point::new(70.0, 20, [50.0, 0.0]);
        let one = Cluster::new(center, vec![&pts[0]], vec![]);
        let two = Cluster::new(center, vec![&pts[0], &pts[1]], vec![]);
        let three = Cluster::new(center, vec![&pts[0], &pts[1], &pts[2]], vec![]);

        let buckets = Greedy::default().bucket_clusters_by_size(vec![one, two, three]);
        assert_eq!(buckets.len(), 4, "buckets span 0..=max_all_len (3)");
        assert_eq!(buckets[0].len(), 0);
        assert_eq!(buckets[1].len(), 1);
        assert_eq!(buckets[2].len(), 1);
        assert_eq!(buckets[3].len(), 1);
    }

    #[test]
    fn recover_missing_points_filters_seen_and_caps() {
        let points: Vec<[f64; 2]> = vec![[40.0, -74.0], [41.0, -73.0], [42.0, -72.0]];
        let greedy = Greedy::default();

        let seen_first: HashSet<CellID> = [Point::new(70.0, 20, points[0]).cell_id]
            .into_iter()
            .collect();
        assert_eq!(
            greedy.recover_missing_points(&seen_first, &points, 10).len(),
            2,
            "two unseen points should be recovered"
        );
        assert_eq!(
            greedy.recover_missing_points(&seen_first, &points, 1).len(),
            1,
            "max_to_add caps the output"
        );
        assert!(
            greedy.recover_missing_points(&seen_first, &points, 0).is_empty(),
            "max_to_add = 0 is the fast path"
        );

        let seen_all: HashSet<CellID> = points
            .iter()
            .map(|p| Point::new(70.0, 20, *p).cell_id)
            .collect();
        assert!(
            greedy.recover_missing_points(&seen_all, &points, 10).is_empty(),
            "nothing missing when every point is seen"
        );
    }

    #[test]
    fn cluster_greedily_keeps_dominant_and_blocks_its_points() {
        // Four distinct points; two overlapping candidate clusters that share `c`.
        let a = Point::new(70.0, 20, [40.00, -74.0]);
        let b = Point::new(70.0, 20, [40.01, -74.0]);
        let c = Point::new(70.0, 20, [40.02, -74.0]);
        let d = Point::new(70.0, 20, [40.03, -74.0]);
        let center1 = Point::new(70.0, 20, [40.005, -74.0]); // candidate covering a, b, c
        let center2 = Point::new(70.0, 20, [40.025, -74.0]); // candidate covering c, d
        let c1 = Cluster::new(center1, vec![&a, &b, &c], vec![]);
        let c2 = Cluster::new(center2, vec![&c, &d], vec![]);

        let mut greedy = Greedy::default();
        greedy.set_min_points(2);
        let buckets = greedy.bucket_clusters_by_size(vec![c1, c2]);
        let solution = greedy.cluster(&buckets);

        // c1 (3 unique) is selected first and blocks a, b, c; c2 then has only d left
        // (1 < min_points) and is dropped.
        assert_eq!(solution.len(), 1, "only the dominant cluster survives");
        assert!(
            solution.iter().any(|cl| cl.point.cell_id == center1.cell_id),
            "the surviving cluster is the max-coverage one"
        );
    }
}
