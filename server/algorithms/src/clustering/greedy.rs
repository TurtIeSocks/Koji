use hashbrown::HashSet;
use model::api::{cluster_mode::ClusterMode, single_vec::SingleVec, GetBbox, Precision};

use ::s2::cellid::CellID;
use rayon::{
    prelude::{
        IntoParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
    },
    slice::ParallelSliceMut,
};
use rstar::RTree;
use std::{io::Write, time::Instant};
use sysinfo::{System, SystemExt};

use crate::{
    clustering::rtree::{cluster::Cluster, point::Point},
    rtree::{self, point::ToPoint, SortDedupe},
    s2,
    utils::info_log,
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
            cluster_split_level: 1,
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

        let return_set = if self.cluster_split_level == 1 {
            self.setup(points)
        } else {
            let cell_maps = s2::create_cell_map(&points, self.cluster_split_level);

            let mut return_set = HashSet::new();
            std::thread::scope(|s| {
                let mut handlers = vec![];
                for (key, values) in cell_maps.iter() {
                    log::debug!("Cell: {} | Points: {}", key, values.len());
                    let thread = s.spawn(move || self.setup(values));
                    handlers.push(thread);
                }
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

    fn generate_clusters(&self, point: &Point, neighbors: Vec<&Point>) -> HashSet<Point> {
        let mut clusters = HashSet::new();
        for neighbor in neighbors.iter() {
            for i in 0..=7 {
                let ratio = i as Precision / 8 as Precision;
                let new_point = point.interpolate(neighbor, ratio, 0., 0.);
                clusters.insert(new_point);
                if self.cluster_mode == ClusterMode::Balanced {
                    for wiggle in vec![0.00025, 0.0001] {
                        let wiggle_lat: Precision = wiggle / 2.;
                        let wiggle_lon = wiggle;
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, wiggle_lon);
                        clusters.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, -wiggle_lon);
                        clusters.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, wiggle_lon);
                        clusters.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, -wiggle_lon);
                        clusters.insert(random_point);
                    }
                }
            }
        }
        clusters.insert(point.to_owned());
        clusters
    }

    fn gen_estimated_clusters(&self, tree: &RTree<Point>) -> Vec<Point> {
        let tree_points: Vec<&Point> = tree.iter().map(|p| p).collect();

        let clusters = tree_points
            .par_iter()
            .map(|point| {
                let neighbors = tree
                    .locate_all_at_point(&point.center)
                    .into_iter()
                    .collect();
                self.generate_clusters(point, neighbors)
            })
            .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

        clusters.into_iter().collect()
    }

    fn flat_map_cells(&self, cell: CellID) -> Vec<CellID> {
        if cell.level() == 21 {
            cell.children().into_iter().collect()
        } else {
            cell.children()
                .into_par_iter()
                .flat_map(|c| self.flat_map_cells(c))
                .collect()
        }
    }

    fn get_s2_clusters(&self, points: &SingleVec) -> Vec<Point> {
        let bbox = points.get_bbox().unwrap();
        s2::get_region_cells(bbox[1], bbox[3], bbox[0], bbox[2], 16)
            .0
            .into_par_iter()
            .flat_map(|cell| self.flat_map_cells(cell))
            .map(|cell| cell.to_point(self.radius))
            .collect()
    }

    fn associate_clusters(
        &'a self,
        points: &'a SingleVec,
        point_tree: &'a RTree<Point>,
    ) -> Vec<Vec<Cluster>> {
        let sys = System::new_all();
        let sys_mem = (sys.available_memory() / 1024 / 1024) as usize;

        let time = Instant::now();
        let clusters_with_data: Vec<Cluster> = match self.cluster_mode {
            ClusterMode::Better | ClusterMode::Best => self.get_s2_clusters(points),
            ClusterMode::Fast => self.gen_estimated_clusters(&point_tree),
            _ => {
                let time = Instant::now();
                let neighbor_tree: RTree<Point> = rtree::spawn(self.radius * 2., points);
                log::info!("created neighbor tree {:.2}s", time.elapsed().as_secs_f32());
                self.gen_estimated_clusters(&neighbor_tree)
            }
        }
        .into_par_iter()
        .filter_map(|cluster| {
            let mut points: Vec<&Point> = point_tree
                .locate_all_at_point(&cluster.center)
                .collect::<Vec<&Point>>();
            if let Some(point) = point_tree.locate_at_point(&cluster.center) {
                points.push(point);
            }
            if points.len() < self.min_points {
                log::debug!("Empty");
                None
            } else {
                Some(Cluster::new(cluster, points, vec![]))
            }
        })
        .collect();

        log::info!(
            "associated points with {} clusters in {:.2}s",
            clusters_with_data.len(),
            time.elapsed().as_secs_f32(),
        );

        let size = (clusters_with_data
            .par_iter()
            .map(|cluster| cluster.get_size())
            .sum::<usize>()
            / 1024
            / 1024)
            * 2;
        if size > sys_mem {
            log::warn!(
                "Kōji is taking a lot of memory ({}MB), I hope you know what you're doing! If you're getting this warning, try sending smaller areas or not using the `Better` algorithm.",
                size,
            );
        }

        let time = Instant::now();
        let max = clusters_with_data
            .par_iter()
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

        let mut solution = self.cluster(clusters_with_data).into_iter().collect();

        self.update_unique(&mut solution);

        if self.min_points == 1 {
            self.check_missing(solution, points)
        } else {
            solution.into_iter().map(|c| c.into()).collect()
        }
    }

    fn cluster(&'a self, clusters_with_data: Vec<Vec<Cluster<'a>>>) -> HashSet<Cluster<'a>> {
        let time = Instant::now();
        log::info!("starting initial solution",);
        let mut new_clusters = HashSet::<Cluster>::new();
        let mut blocked_points = HashSet::<&Point>::new();

        let mut highest = clusters_with_data.len() - 1;
        let total_iterations = highest - self.min_points + 1;
        let mut current_iteration = 0;
        let mut stdout = std::io::stdout();

        let mut clusters_of_interest_time = 0.;
        let mut local_clusters_time = 0.;
        let mut sorting_time = 0.;
        let mut iterating_local_time = 0.;
        let mut logging_time = 0.;

        'greedy: while highest >= self.min_points && new_clusters.len() < self.max_clusters {
            let mut clusters_of_interest: Vec<&Cluster<'_>> = vec![];
            current_iteration += 1;
            let time = Instant::now();
            for (max, clusters) in clusters_with_data.iter().enumerate() {
                if max < highest {
                    continue;
                }
                clusters_of_interest.extend(clusters);
            }
            clusters_of_interest_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
            let mut local_clusters = clusters_of_interest
                .into_par_iter()
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
                    if points.len() < highest {
                        None
                    } else {
                        points.sort_dedupe();

                        Some(Cluster {
                            point: cluster.point,
                            points: points.into_iter().collect(),
                            all: cluster.all.iter().map(|p| *p).collect(),
                        })
                    }
                })
                .collect::<Vec<Cluster>>();
            local_clusters_time += time.elapsed().as_secs_f32();

            if local_clusters.is_empty() {
                highest -= 1;
                continue;
            }

            let time = Instant::now();
            local_clusters.par_sort_by(|a, b| {
                if a.points.len() == b.points.len() {
                    b.all.len().cmp(&a.all.len())
                } else {
                    b.points.len().cmp(&a.points.len())
                }
            });
            sorting_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
            'cluster: for cluster in local_clusters.into_iter() {
                if new_clusters.len() >= self.max_clusters {
                    break 'greedy;
                }
                if cluster.points.len() >= highest {
                    for point in cluster.points.iter() {
                        if blocked_points.contains(point) {
                            continue 'cluster;
                        }
                    }
                    for point in cluster.points.iter() {
                        blocked_points.insert(point);
                    }
                    new_clusters.insert(cluster);
                }
            }
            iterating_local_time += time.elapsed().as_secs_f32();

            let time = Instant::now();
            if highest >= self.min_points {
                stdout
                    .write(
                        info_log(
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

            highest -= 1;
        }
        stdout.write(format!("\n",).as_bytes()).unwrap();

        log::debug!("Interested Clusters Time: {:.4}", clusters_of_interest_time);
        log::debug!("Local Clusters Time: {:.4}", local_clusters_time);
        log::debug!("Sorting Time: {:.4}", sorting_time);
        log::debug!("Iterating Local Time: {:.4}", iterating_local_time);
        log::debug!("Logging Time: {:.4}", logging_time);

        log::info!(
            "finished initial solution in {:.2}s",
            time.elapsed().as_secs_f32()
        );
        log::info!("initial solution size: {}", new_clusters.len());

        new_clusters
    }

    fn update_unique(&'a self, clusters: &mut Vec<Cluster>) {
        let time = Instant::now();
        log::info!("updating unique");

        let cluster_tree = rtree::spawn(
            self.radius,
            &clusters.iter().map(|c| c.point.center).collect(),
        );

        clusters
            .par_iter_mut()
            .for_each(|cluster| cluster.update_unique(&cluster_tree));

        clusters.retain(|cluster| cluster.points.len() >= self.min_points);

        log::info!(
            "finished updating unique in {:.2}s",
            time.elapsed().as_secs_f32()
        );
        log::info!("unique solution size: {}", clusters.len());

        // crate::utils::_debug_clusters(&clusters.clone().into_iter().collect(), "x");
    }

    fn check_missing(&self, clusters: Vec<Cluster>, points: &SingleVec) -> HashSet<Point> {
        let missing = {
            let seen_points = clusters
                .par_iter()
                .map(|cluster| cluster.all.iter().collect())
                .reduce(HashSet::new, |a, b| a.union(&b).cloned().collect());

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

        clusters
    }
}
