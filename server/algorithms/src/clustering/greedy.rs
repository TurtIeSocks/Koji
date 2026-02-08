use geojson::{Feature, Geometry};
use hashbrown::HashSet;
use macros::time;
use model::api::{GetBbox, Precision, cluster_mode::ClusterMode, single_vec::SingleVec};

use ::s2::cellid::CellID;
use rayon::{
    prelude::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use rstar::RTree;
use std::{io::Write, time::Instant};
use sysinfo::System;

use crate::{
    bootstrap::radius,
    clustering::{
        candidates,
        rtree::{cluster::Cluster, point::Point},
    },
    rtree::{self, SortDedupe},
    s2::{self, ToPointArray},
    utils,
};

pub struct Greedy {
    cluster_mode: ClusterMode,
    cluster_split_level: u64,
    max_clusters: usize,
    min_points: usize,
    radius: Precision,
}

impl Default for Greedy {
    fn default() -> Self {
        Greedy {
            cluster_mode: ClusterMode::Balanced,
            cluster_split_level: 0,
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
    pub fn set_cluster_split_level(&mut self, cluster_split_level: u64) -> &mut Self {
        self.cluster_split_level = cluster_split_level;
        self
    }

    pub fn run(&'a self, points: &SingleVec) -> SingleVec {
        let time = Instant::now();
        log::info!("starting algorithm with {} data points", points.len());

        let return_set = if self.cluster_split_level == 0 {
            self.setup(points)
        } else {
            let cell_maps = s2::create_cell_map(&points, self.cluster_split_level);

            let mut return_set = HashSet::new();
            std::thread::scope(|s| {
                let handlers: Vec<std::thread::ScopedJoinHandle<'_, HashSet<Point>>> = cell_maps
                    .iter()
                    .map(|(key, values)| {
                        log::debug!("Cell: {} | Points: {}", key, values.len());
                        s.spawn(move || self.setup(values))
                    })
                    .collect();
                log::info!("created {} threads", handlers.len());
                for thread in handlers {
                    match thread.join() {
                        Ok(results) => {
                            return_set.extend(results);
                        }
                        Err(e) => {
                            log::error!("error joining thread: {:?}", e)
                        }
                    }
                }
            });

            return_set
        };

        log::info!("finished in {:.2}s", time.elapsed().as_secs_f32());
        return_set.into_iter().map(|p| p.center).collect()
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

    fn associate_clusters(
        &'a self,
        points: &'a SingleVec,
        point_tree: &'a RTree<Point>,
    ) -> Vec<Vec<Cluster<'a>>> {
        const BYTE: usize = 1024;

        let sys = System::new_all();
        let sys_mem = sys.available_memory() as usize / BYTE / BYTE;

        let time = Instant::now();
        let clusters_with_data: Vec<Cluster> = match self.cluster_mode {
            ClusterMode::Honeycomb => self.get_honeycomb_clusters(points),
            ClusterMode::Fast => self.gen_clusters(BYTE / 2, points),
            ClusterMode::Balanced => self.gen_clusters(BYTE, points),
            ClusterMode::Better | ClusterMode::Best => {
                let mut pcs = self.get_s2_clusters(points, point_tree);

                if self.cluster_mode == ClusterMode::Best {
                    pcs.extend_from_slice(&self.gen_clusters(BYTE * 6, points));
                }
                pcs
            }
            _ => vec![],
        }
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

        let time = Instant::now();
        let max = clusters_with_data
            .iter()
            .map(|cluster| cluster.all.len())
            .max()
            .unwrap_or(100);
        log::info!(
            "found best cluster ({}) {:.2}s",
            max,
            time.elapsed().as_secs_f32(),
        );

        let time = Instant::now();
        let mut clustered_clusters = vec![vec![]; max + 1];

        for cluster in clusters_with_data.into_iter() {
            clustered_clusters[cluster.all.len()].push(cluster);
        }
        log::info!(
            "sorted clusters by size in {:.2}s",
            time.elapsed().as_secs_f32(),
        );

        clustered_clusters
    }

    fn setup(&'a self, points: &SingleVec) -> HashSet<Point> {
        let time = Instant::now();
        let point_tree: RTree<Point> = rtree::spawn(self.radius, points);
        log::info!("created point tree in {:.2}s", time.elapsed().as_secs_f32());

        let clusters_with_data = self.associate_clusters(points, &point_tree);

        let mut solution = self.cluster(&clusters_with_data).into_iter().collect();

        self.update_unique(&mut solution);

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
                            all: cluster.all.iter().map(|p| *p).collect(),
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
            if current >= self.min_points {
                stdout
                    .write(
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
        stdout.write(format!("\n",).as_bytes()).unwrap();

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
            .iter_mut()
            .for_each(|cluster| cluster.set_unique(&cluster_tree));

        clusters.retain(|cluster| cluster.unique.len() >= self.min_points);

        log::info!("unique solution size: {}", clusters.len());

        // crate::utils::_debug_clusters(&clusters.clone().into_iter().collect(), "x");
    }

    #[time()]
    fn check_missing(&self, clusters: Vec<Cluster>, points: &SingleVec) -> HashSet<Point> {
        let missing = {
            let mut seen_points: HashSet<&Point> = HashSet::with_capacity(points.len());

            for cluster in clusters.iter() {
                seen_points.extend(cluster.all.iter());
            }

            if seen_points.len() == points.len() {
                vec![]
            } else {
                points
                    .par_iter()
                    .filter_map(|p| {
                        let point = Point::new(self.radius, 20, *p);
                        if seen_points.contains(&&point) {
                            None
                        } else {
                            Some(point)
                        }
                    })
                    .collect::<Vec<Point>>()
            }
        };

        let mut clusters: HashSet<Point> = clusters.into_iter().map(|c| c.into()).collect();

        clusters.extend(missing);

        log::info!("final solution size: {}", clusters.len());

        clusters
    }
}
