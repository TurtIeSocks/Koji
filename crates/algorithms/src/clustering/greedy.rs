use geojson::{Feature, Geometry};
use hashbrown::HashSet;
use koji_core::{ClusterMode, GetBbox, PointArray, Precision, SingleVec};
use macros::time;

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
    s2::{self, ToPointArray},
    utils,
};

pub struct Greedy {
    cluster_mode: ClusterMode,
    max_clusters: usize,
    min_points: usize,
    radius: Precision,
    /// Dev-only A/B toggle: when `true`, `run()` skips the adaptive S2 partition
    /// and the post-greedy gap-fill pass, falling back to the pre-PR `setup()`
    /// path for every mode. See [DevArgs] on the API side.
    bypass_adaptive_partition: bool,
}

impl Default for Greedy {
    fn default() -> Self {
        Greedy {
            cluster_mode: ClusterMode::Balanced,
            max_clusters: usize::MAX,
            min_points: 1,
            radius: 70.,
            bypass_adaptive_partition: false,
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
    pub fn set_cluster_split_level(&mut self, cluster_split_level: u64) -> &mut Self {
        if cluster_split_level != 0 {
            log::warn!(
                "cluster_split_level is deprecated and will be ignored. \
                 Adaptive S2 partitioning is now automatic for Better/Best modes."
            );
        }
        self
    }
    /// Dev-only A/B toggle. When `true`, [`Greedy::run`] uses the pre-PR
    /// `setup()` path for every mode (no adaptive partition, no gap-fill).
    /// Wired from `DevArgs::bypass_adaptive_partition` via `clustering::main`.
    pub fn set_bypass_adaptive_partition(&mut self, bypass: bool) -> &mut Self {
        self.bypass_adaptive_partition = bypass;
        self
    }

    pub fn run(&'a self, points: &SingleVec) -> SingleVec {
        let time = Instant::now();
        log::info!("starting algorithm with {} data points", points.len());

        // Dev A/B toggle: when `bypass_adaptive_partition` is on, every mode
        // (including Better/Best) goes through the pre-PR `setup()` path with
        // no partition and no gap-fill — i.e. exactly the algorithm shape from
        // commit prior to docs/superpowers/specs/2026-05-27-greedy-bbox-adaptive-partition-design.md.
        // Used for side-by-side quality comparison during PR review.
        let return_set = if self.bypass_adaptive_partition {
            log::info!("dev: bypass_adaptive_partition=true (pre-PR algorithm path)");
            self.setup(points)
        } else {
            match self.cluster_mode {
                ClusterMode::Better | ClusterMode::Best => self.run_partitioned(points),
                _ => self.setup(points),
            }
        };

        log::info!("finished in {:.2}s", time.elapsed().as_secs_f32());
        return_set.into_iter().map(|p| p.center).collect()
    }

    #[time()]
    fn run_partitioned(&'a self, points: &SingleVec) -> HashSet<Point> {
        use crate::clustering::partition::{
            PartitionConfig, PartitionStats, adaptive_partition, mode_tag,
        };
        use rayon::prelude::*;

        let config = PartitionConfig::load();
        // Global rtree built once for halo gathering and gap-fill. Per-chunk
        // candidate generation + greedy uses chunk-local data (bounded memory).
        let all_points_tree: RTree<Point> = crate::rtree::spawn(self.radius, points);
        let chunks = adaptive_partition(
            points,
            config.budget,
            &self.cluster_mode,
            config.start_level,
            config.max_level,
        );

        let mut stats = PartitionStats::default();

        // Per-chunk greedy: each chunk builds its own (owned + halo) rtree,
        // generates candidates bounded to chunk extent, runs greedy + update_unique
        // locally, then drops the chunk-local rtree. Memory peak per chunk is
        // bounded by the budget; flat global Vec<Cluster> never holds all
        // candidates simultaneously.
        let chunk_results: Vec<(HashSet<Point>, Option<(ClusterMode, ClusterMode)>)> = chunks
            .par_iter()
            .map(|chunk| self.solve_chunk_local(chunk, &all_points_tree, config.budget))
            .collect();

        let mut solution: HashSet<Point> = HashSet::with_capacity(chunks.len() * 32);
        for (chunk_solution, downgrade) in chunk_results {
            solution.extend(chunk_solution);
            if let Some((from, to)) = downgrade {
                let key = (mode_tag(&from), mode_tag(&to));
                *stats.downgrades.entry(key).or_insert(0) += 1;
            }
        }

        log::info!(
            "partition: {} chunks, downgrades: {:?}, post-chunk centers: {}",
            chunks.len(),
            stats.downgrades,
            solution.len(),
        );

        // Convert HashSet<Point> back to Vec<Cluster> for the merge + gap-fill
        // passes. Each Cluster's `all` is recomputed against the GLOBAL tree so
        // these passes see correct coverage counts even though earlier greedy
        // ran chunk-locally.
        let solution_vec: Vec<Cluster<'_>> = solution
            .into_iter()
            .filter_map(|p| {
                let mut covered: Vec<&Point> =
                    all_points_tree.locate_all_at_point(&p.center).collect();
                covered.sort_dedupe();
                (!covered.is_empty()).then(|| Cluster::new(p, covered, vec![]))
            })
            .collect();

        // Post-greedy gap-fill: each new cluster covering > min_points
        // previously-uncovered points is a strict mygod_score win.
        // (merge_redundant_clusters was removed from this path: SEC-per-pair
        // cost dominates wall-clock on non-dense inputs for ≈0.4% mygod gain.
        // The fn is still defined and available behind future opt-in flags.)
        let solution_vec = self.fill_coverage_gaps(solution_vec, points, &all_points_tree);

        let mut final_solution: HashSet<Point> =
            solution_vec.into_iter().map(|c| c.into()).collect();

        if self.min_points == 1 {
            let cap = self.max_clusters.saturating_sub(final_solution.len());
            if cap > 0 {
                let seen_cell_ids: HashSet<CellID> =
                    final_solution.iter().map(|p| p.cell_id).collect();
                let missing = self.recover_missing_points(&seen_cell_ids, points, cap);
                final_solution.extend(missing);
            }
        }
        log::info!("final solution size: {}", final_solution.len());
        final_solution
    }

    /// Solve a single partition chunk locally and return its cluster centers.
    /// Greedy runs chunk-locally to bound memory: only the chunk's owned + halo
    /// points + candidates ever live in memory at once. Result is the set of
    /// cluster centers owned by this chunk (ownership = cell-of-center).
    fn solve_chunk_local(
        &'a self,
        chunk: &crate::clustering::partition::Chunk,
        all_points_tree: &'a RTree<Point>,
        budget: usize,
    ) -> (HashSet<Point>, Option<(ClusterMode, ClusterMode)>) {
        use crate::clustering::partition::{
            contains_latlng, gather_halo, s2_walk_cost, scaled_grid_density, select_effective_mode,
        };

        let halo = gather_halo(chunk.cell, all_points_tree, self.radius);
        let mut combined: SingleVec = Vec::with_capacity(chunk.owned.len() + halo.len());
        combined.extend(chunk.owned.iter().copied());
        combined.extend(halo.iter().map(|p| p.center));

        let effective_mode = select_effective_mode(self.cluster_mode.clone(), &combined, budget);
        let downgrade = (effective_mode != self.cluster_mode)
            .then(|| (self.cluster_mode.clone(), effective_mode.clone()));

        let grid_density = if matches!(effective_mode, ClusterMode::Best) {
            let s2_cost = s2_walk_cost(&combined);
            Some(scaled_grid_density(s2_cost, budget))
        } else {
            None
        };

        // chunk_tree scoped so it drops before this fn returns — its memory is
        // released before the next chunk runs on the same thread.
        let chunk_tree: RTree<Point> = crate::rtree::spawn(self.radius, &combined);
        let raw_candidates =
            self.generate_candidates_for_mode(&combined, &chunk_tree, effective_mode, grid_density);

        let clusters_with_data: Vec<Cluster> = raw_candidates
            .into_par_iter()
            .filter_map(|center| {
                let iter = chunk_tree.locate_all_at_point(&center);
                let mut covered = Vec::with_capacity(iter.size_hint().0);
                covered.extend(iter);
                (covered.len() >= self.min_points)
                    .then(|| Cluster::new(Point::new(self.radius, 20, center), covered, vec![]))
            })
            .collect();

        let bucketed = self.bucket_clusters_by_size(clusters_with_data);
        let mut solution: Vec<Cluster> = self.cluster(&bucketed).into_iter().collect();
        self.update_unique(&mut solution);

        // Ownership filter: a chunk only emits cluster centers whose location
        // parents to this chunk's owning cell. Adjacent chunks emit their own;
        // the global merge + gap-fill passes downstream stitch the boundaries.
        solution.retain(|c| contains_latlng(chunk.cell, c.point.center));
        let result: HashSet<Point> = solution.into_iter().map(|c| c.into()).collect();
        (result, downgrade)
    }

    /// Post-greedy merge: walk pairs of nearby cluster centers (within 2*radius)
    /// and replace any pair whose union of covered points fits within `radius` of
    /// the pair's midpoint with a single cluster at that midpoint. Lossless —
    /// coverage is preserved by construction.
    ///
    /// The mygod_score formula (`clusters * min_points + uncovered_points`, source
    /// of truth: stats.rs::Stats::get_score) drops by exactly `min_points` per
    /// successful merge: one fewer cluster, zero new uncovered points.
    ///
    /// Greedy avoidance: each cluster is considered exactly once in index order.
    /// When two clusters merge they're both marked removed; later iterations skip
    /// them. A single pass per `run`; cheap (~O(n*k) where k is average neighbor
    /// count, typically 1-3 for non-degenerate point distributions).
    #[allow(dead_code)] // kept for future opt-in once SEC pre-filter is cheaper
    fn merge_redundant_clusters(
        &self,
        mut solution: Vec<Cluster<'a>>,
        all_points_tree: &'a RTree<Point>,
    ) -> Vec<Cluster<'a>> {
        use ::s2::cellid::CellID;
        use rstar::AABB;

        if solution.len() < 2 {
            return solution;
        }

        // Tree over cluster centers (radius=2*self.radius so envelope queries find
        // any cluster center within 2r of the query point — the only candidates
        // geometrically able to merge).
        let centers: SingleVec = solution.iter().map(|c| c.point.center).collect();
        let center_tree: RTree<Point> = crate::rtree::spawn(self.radius * 2.0, &centers);

        let id_to_idx: hashbrown::HashMap<CellID, usize> = solution
            .iter()
            .enumerate()
            .map(|(i, c)| (c.point.cell_id, i))
            .collect();

        let mut removed: HashSet<CellID> = HashSet::with_capacity(solution.len() / 8);
        let mut additions: Vec<Cluster<'a>> = Vec::new();
        let mut attempted_merges: usize = 0;
        let mut successful_merges: usize = 0;

        for i in 0..solution.len() {
            let cluster = &solution[i];
            if removed.contains(&cluster.point.cell_id) {
                continue;
            }

            // Find candidate neighbors: cluster centers within 2*radius envelope.
            // Filter out self + already-removed.
            let neighbors: Vec<&Point> = center_tree
                .locate_in_envelope_intersecting(&AABB::from_point(cluster.point.center))
                .filter(|p| p.cell_id != cluster.point.cell_id && !removed.contains(&p.cell_id))
                .collect();

            for neighbor in neighbors {
                attempted_merges += 1;
                let nb_idx = match id_to_idx.get(&neighbor.cell_id) {
                    Some(idx) => *idx,
                    None => continue,
                };
                let nb_cluster = &solution[nb_idx];

                // Compute SEC of the union of points (de-duped). multi_attempt
                // returns Centered iff the SEC radius is within self.radius,
                // i.e. the merge is geometrically possible without coverage loss.
                let union_points: Vec<&Point> = cluster
                    .all
                    .iter()
                    .chain(nb_cluster.all.iter())
                    .copied()
                    .collect();
                let mut union_dedup: Vec<&Point> = union_points;
                union_dedup.sort_dedupe();

                let sec_result = crate::sec::sec::multi_attempt(
                    union_dedup
                        .iter()
                        .map(|p| geo::Point::new(p.center[1], p.center[0])),
                    self.radius,
                    16,
                );
                let merge_center: PointArray = match sec_result {
                    crate::sec::sec::SmallestEnclosingCircle::Centered(g) => [g.y(), g.x()],
                    _ => continue, // SEC won't fit — lossy merges hurt mygod_score
                };

                // Verify against rtree (planar vs geodesic distance differ).
                let mid_coverage: HashSet<CellID> = all_points_tree
                    .locate_all_at_point(&merge_center)
                    .map(|p| p.cell_id)
                    .collect();
                let union_ids: HashSet<CellID> = union_dedup.iter().map(|p| p.cell_id).collect();
                if !union_ids.is_subset(&mid_coverage) {
                    continue;
                }

                // Build merged cluster.
                let new_pt = Point::new(self.radius, 20, merge_center);
                let mut new_all: Vec<&Point> =
                    all_points_tree.locate_all_at_point(&merge_center).collect();
                new_all.sort_dedupe();
                additions.push(Cluster::new(new_pt, new_all, vec![]));

                removed.insert(cluster.point.cell_id);
                removed.insert(nb_cluster.point.cell_id);
                successful_merges += 1;
                break;
            }
        }

        log::info!(
            "merge_pass: {} attempts, {} successful merges, -{} clusters",
            attempted_merges,
            successful_merges,
            successful_merges,
        );

        // Rebuild final solution: kept clusters + new merged clusters.
        solution.retain(|c| !removed.contains(&c.point.cell_id));
        solution.extend(additions);

        // Recompute `unique` for the merged solution so downstream callers see
        // correct uniqueness counts.
        self.update_unique(&mut solution);
        solution
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
        let bbox = points.get_bbox();
        let bbox_unwrap = bbox.clone().unwrap();

        let feat = Feature {
            bbox: bbox.clone(),
            geometry: Some(Geometry {
                bbox,
                foreign_members: None,
                value: geojson::Value::Polygon(vec![vec![
                    vec![bbox_unwrap[0], bbox_unwrap[1]],
                    vec![bbox_unwrap[2], bbox_unwrap[1]],
                    vec![bbox_unwrap[2], bbox_unwrap[3]],
                    vec![bbox_unwrap[0], bbox_unwrap[3]],
                    vec![bbox_unwrap[0], bbox_unwrap[1]],
                ]]),
            }),
            ..Default::default()
        };
        radius::BootstrapRadius::new(&feat, self.radius).result()
    }

    fn flat_map_cells(&self, cell: CellID, point_tree: &'a RTree<Point>) -> Vec<CellID> {
        if cell.level() == 21 {
            cell.children().into_iter().collect()
        } else if point_tree.locate_at_point(&cell.point_array()).is_some() {
            cell.children()
                .into_iter()
                .flat_map(|c| self.flat_map_cells(c, point_tree))
                .collect()
        } else {
            vec![]
        }
    }

    #[time()]
    fn get_s2_clusters(&self, points: &SingleVec, point_tree: &'a RTree<Point>) -> SingleVec {
        let bbox = points.get_bbox().unwrap();
        s2::get_region_cells(bbox[1], bbox[3], bbox[0], bbox[2], 16)
            .0
            .into_par_iter()
            .flat_map(|cell| self.flat_map_cells(cell, point_tree))
            .map(|cell| cell.point_array())
            .collect()
    }

    fn gen_clusters(&self, density: usize, points: &'a SingleVec) -> SingleVec {
        candidates::generate_clusters_from_points(points, self.radius, density)
    }

    fn generate_candidates_for_mode(
        &self,
        points: &'a SingleVec,
        point_tree: &'a RTree<Point>,
        mode: ClusterMode,
        grid_density_override: Option<usize>,
    ) -> SingleVec {
        match mode {
            ClusterMode::Honeycomb => self.get_honeycomb_clusters(points),
            ClusterMode::Fast => self.gen_clusters(BYTE / 2, points),
            ClusterMode::Balanced => self.gen_clusters(BYTE, points),
            ClusterMode::Better => self.get_s2_clusters(points, point_tree),
            ClusterMode::Best => {
                let mut pcs = self.get_s2_clusters(points, point_tree);
                let density = grid_density_override.unwrap_or(BYTE * 6);
                if density > 0 {
                    pcs.extend_from_slice(&self.gen_clusters(density, points));
                }
                pcs
            }
            _ => vec![],
        }
    }

    // associate_clusters_for_chunk + solve_chunk removed: the new run_partitioned
    // generates candidates per-chunk but defers ALL greedy selection to a single
    // global pass (see generate_chunk_clusters + global cluster() call in
    // run_partitioned). This avoids the per-chunk greedy quality loss observed on
    // medium-density inputs where chunk-local greedy commits to suboptimal
    // boundary clusters that no consolidation pass could fully recover.

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
            .generate_candidates_for_mode(points, point_tree, self.cluster_mode.clone(), None)
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
        // Skipped when bypass_adaptive_partition is on so the dev path is
        // exactly the pre-PR algorithm (no gap-fill).
        let solution = if self.bypass_adaptive_partition {
            solution
        } else {
            self.fill_coverage_gaps(solution, points, &point_tree)
        };

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

        let mut current = clusters_with_data.len() - 1;
        let total_iterations = current - self.min_points + 1;
        let mut current_iteration = 0;
        #[cfg(feature = "native")]
        let mut stdout = std::io::stdout();

        let mut clusters_of_interest_time = 0.;
        let mut local_clusters_time = 0.;
        let mut sorting_time = 0.;
        let mut iterating_local_time = 0.;
        let mut logging_time = 0.;
        let capacity = clusters_with_data.iter().map(|c| c.len()).sum::<usize>();
        let mut clusters_of_interest: Vec<&Cluster<'_>> = Vec::with_capacity(capacity);

        'greedy: while current >= self.min_points && new_clusters.len() < self.max_clusters {
            current_iteration += 1;
            let time = Instant::now();
            clusters_of_interest.clear();
            for (index, clusters) in clusters_with_data.iter().enumerate() {
                if index < current {
                    continue;
                }
                clusters_of_interest.extend(clusters);
            }
            clusters_of_interest_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
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
                            all: cluster.all.iter().copied().collect(),
                        })
                    }
                })
                .collect::<Vec<Cluster>>();
            local_clusters_time += time.elapsed().as_secs_f32();

            if local_clusters.is_empty() {
                current -= 1;
                continue;
            }

            let time = Instant::now();
            local_clusters.par_sort_by(|a, b| {
                if a.unique.len() == b.unique.len() {
                    b.all.len().cmp(&a.all.len())
                } else {
                    b.unique.len().cmp(&a.unique.len())
                }
            });
            sorting_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
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
            iterating_local_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
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
            logging_time += time.elapsed().as_secs_f32();

            current -= 1;
        }
        #[cfg(feature = "native")]
        stdout.write_all(b"\n").unwrap();

        log::debug!("Interested Clusters Time: {:.4}", clusters_of_interest_time);
        log::debug!("Local Clusters Time: {:.4}", local_clusters_time);
        log::debug!("Sorting Time: {:.4}", sorting_time);
        log::debug!("Iterating Local Time: {:.4}", iterating_local_time);
        log::debug!("Logging Time: {:.4}", logging_time);

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

        // crate::utils::_debug_clusters(&clusters.clone().into_iter().collect(), "x");
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
