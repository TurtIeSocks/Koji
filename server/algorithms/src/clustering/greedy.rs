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

use crate::{
    clustering::rtree::{cluster::Cluster, point::Point},
    rtree::{self, point::ToPoint},
    s2,
    utils::info_log,
};

pub struct Greedy {
    cluster_mode: ClusterMode,
    cluster_split_level: u64,
    max_clusters: usize,
    min_points: usize,
    radius: f64,
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
    pub fn set_radius(&mut self, radius: f64) -> &mut Self {
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
        let mut set = HashSet::<Point>::new();
        for neighbor in neighbors.iter() {
            for i in 0..=7 {
                let ratio = i as Precision / 8 as Precision;
                let new_point = point.interpolate(neighbor, ratio, 0., 0.);
                set.insert(new_point);
                if self.cluster_mode == ClusterMode::Balanced {
                    for wiggle in vec![0.00025, 0.0001] {
                        let wiggle_lat: f64 = wiggle / 2.;
                        let wiggle_lon = wiggle;
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, wiggle_lat, -wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, wiggle_lon);
                        set.insert(random_point);
                        let random_point =
                            point.interpolate(neighbor, ratio, -wiggle_lat, -wiggle_lon);
                        set.insert(random_point);
                    }
                }
            }
        }
        set.insert(point.to_owned());
        set
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

        clusters.into_iter().collect::<Vec<Point>>()
    }

    fn flat_map_cells(&self, cell: CellID) -> Vec<Point> {
        let point = cell.to_point(self.radius);
        if cell.level() == 22 {
            vec![point]
        } else {
            // let mut children: Vec<Point> = cell
            //     .children()
            //     .into_par_iter()
            //     .flat_map(|c| self.flat_map_cells(c))
            //     .collect();
            // children.push(point);
            // children
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
            .collect()
    }

    fn associate_clusters(
        &'a self,
        points: &'a SingleVec,
        point_tree: &'a RTree<Point>,
    ) -> Vec<Vec<Cluster>> {
        let time = Instant::now();
        let clusters: Vec<Point> = match self.cluster_mode {
            ClusterMode::Better | ClusterMode::Best => self.get_s2_clusters(points),
            ClusterMode::Fast => self.gen_estimated_clusters(&point_tree),
            _ => {
                let time = Instant::now();
                let neighbor_tree: RTree<Point> = rtree::spawn(self.radius * 2., points);
                log::info!("created neighbor tree {:.2}s", time.elapsed().as_secs_f32());
                self.gen_estimated_clusters(&neighbor_tree)
            }
        };
        log::info!(
            "created {} possible clusters in {:.2}s",
            clusters.len(),
            time.elapsed().as_secs_f32(),
        );

        let time = Instant::now();
        let clusters_with_data: Vec<Cluster> = clusters
            .into_par_iter()
            .filter_map(|cluster| {
                let mut points: Vec<&'a Point> = point_tree
                    .locate_all_at_point(&cluster.center)
                    .collect::<Vec<&Point>>();
                if let Some(point) = point_tree.locate_at_point(&cluster.center) {
                    points.push(point);
                }
                if points.len() < self.min_points {
                    log::debug!("Empty");
                    None
                } else {
                    Some(Cluster::new(
                        cluster,
                        points.into_iter(),
                        vec![].into_iter(),
                    ))
                }
            })
            .collect();
        let filtered_count = clusters_with_data.len();
        let max = clusters_with_data
            .par_iter()
            .map(|cluster| cluster.all.len())
            .max()
            .unwrap_or(100);
        let mut clustered_clusters: Vec<Vec<Cluster>> = vec![vec![]; max + 1];

        for cluster in clusters_with_data.into_iter() {
            clustered_clusters[cluster.all.len()].push(cluster);
        }
        log::info!(
            "associated points with {} clusters in {:.2}s",
            filtered_count,
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

        solution.into_iter().map(|x| x.into()).collect()

        // let mut seen_points: HashSet<&Point> = HashSet::new();

        // for cluster in solution.iter() {
        //     let points = point_tree
        //         .locate_all_at_point(&cluster.center)
        //         .into_iter()
        //         .collect::<Vec<&Point>>();
        //     seen_points.extend(points);
        // }
        // let missing = points
        //     .iter()
        //     .filter_map(|p| {
        //         let point = Point::new(self.radius, 20, *p);

        //         if seen_points.contains(&point) {
        //             None
        //         } else {
        //             Some(point)
        //         }
        //     })
        //     .collect::<Vec<Point>>();

        // println!(
        //     "seen points: {} | missing: {}",
        //     seen_points.len(),
        //     missing.len()
        // );
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

        'greedy: while highest >= self.min_points && new_clusters.len() < self.max_clusters {
            let mut clusters_of_interest: Vec<&Cluster<'_>> = vec![];
            current_iteration += 1;

            for (max, clusters) in clusters_with_data.iter().enumerate() {
                if max < highest {
                    continue;
                }
                clusters_of_interest.extend(clusters);
            }
            let mut local_clusters = clusters_of_interest
                .into_par_iter()
                .filter_map(|cluster| {
                    if new_clusters.contains(cluster) {
                        None
                    } else {
                        let points: HashSet<&Point> = cluster
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
                            Some(Cluster {
                                point: cluster.point,
                                points,
                                all: cluster.all.iter().map(|p| *p).collect(),
                            })
                        }
                    }
                })
                .collect::<Vec<Cluster>>();
            if local_clusters.is_empty() {
                highest -= 1;
                continue;
            }

            local_clusters.par_sort_by(|a, b| {
                if a.points.len() == b.points.len() {
                    b.all.len().cmp(&a.all.len())
                } else {
                    b.points.len().cmp(&a.points.len())
                }
            });

            'cluster: for cluster in local_clusters.into_iter() {
                if new_clusters.len() >= self.max_clusters {
                    break 'greedy;
                }
                let length = cluster.points.len();
                if length >= highest {
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

            highest -= 1;
        }
        stdout.write(format!("\n",).as_bytes()).unwrap();

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
        clusters.par_iter_mut().for_each(|cluster| {
            cluster.points = cluster
                .all
                .iter()
                .collect::<Vec<&&Point>>()
                .into_par_iter()
                .filter_map(|p| {
                    let mut count = cluster_tree.locate_all_at_point(&p.center).count();
                    if cluster_tree.contains(p) {
                        count += 1;
                    };
                    if count == 1 {
                        Some(*p)
                    } else {
                        None
                    }
                })
                .collect::<Vec<&Point>>()
                .into_iter()
                .collect();
        });

        clusters.retain(|cluster| cluster.points.len() >= self.min_points);

        log::info!(
            "finished updating unique in {:.2}s",
            time.elapsed().as_secs_f32()
        );
        log::info!("unique solution size: {}", clusters.len());

        // _debug_clusters(&x, "x");
    }
}
