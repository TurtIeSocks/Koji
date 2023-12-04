use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use geo::{HaversineDistance, Point};
use s2::cellid::CellID;
use s2::latlng::LatLng;

use crate::routing::sorting::SortS2;
use crate::s2::create_cell_map;
use crate::utils::{self, stringify_points};
use model::api::{point_array::PointArray, single_vec::SingleVec};

#[derive(Debug)]
pub struct PluginManager<'a> {
    plugin: String,
    plugin_path: String,
    interpreter: String,
    route_split_level: u64,
    radius: f64,
    clusters: &'a SingleVec,
}

impl<'a> PluginManager<'a> {
    pub fn new(
        plugin: &str,
        route_split_level: u64,
        radius: f64,
        clusters: &'a SingleVec,
    ) -> std::io::Result<Self> {
        let path = Path::new("algorithms/src/routing/plugins");
        let path = path.join(plugin);
        let plugin_path = if path.exists() {
            path.display().to_string()
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "{plugin} does not exist{}",
                    if plugin == "tsp" {
                        ", rerun the OR Tools Script"
                    } else {
                        ""
                    }
                ),
            ));
        };

        let interpreter = match plugin.split(".").last() {
            Some("py") => "python3",
            Some("js") => "node",
            Some("ts") => "ts-node",
            val => {
                if plugin == val.unwrap_or("") {
                    ""
                } else {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Unrecognized plugin, please create a PR to add support for it",
                    ));
                }
            }
        };
        Ok(PluginManager {
            plugin: plugin.to_string(),
            plugin_path,
            interpreter: interpreter.to_string(),
            route_split_level,
            radius,
            clusters,
        })
    }

    pub fn run(self) -> Result<SingleVec, std::io::Error> {
        log::info!("starting {}...", self.plugin);
        let time = Instant::now();

        if self.route_split_level < 2 || self.plugin != "tsp" {
            return self.spawn_child_process(self.clusters);
        }
        let get_cell_id = |point: PointArray| {
            CellID::from(LatLng::from_degrees(point[0], point[1]))
                .parent(self.route_split_level)
                .0
        };
        let merged_routes: Vec<(PointArray, SingleVec)> =
            create_cell_map(&self.clusters, self.route_split_level as u64)
                .into_iter()
                .enumerate()
                .map(|(i, (cell_id, segment))| {
                    log::debug!("Creating thread: {} for hash {}", i + 1, cell_id);
                    let mut route = self.spawn_child_process(&segment).unwrap_or(vec![]);
                    if let Some(last) = route.last() {
                        if let Some(first) = route.first() {
                            if first == last {
                                route.pop();
                            }
                        }
                    }
                    (
                        if route.len() > 0 {
                            utils::centroid(&route)
                        } else {
                            [0., 0.]
                        },
                        route,
                    )
                })
                .collect();
        let mut centroids = vec![];

        let mut point_map = HashMap::<u64, SingleVec>::new();
        merged_routes
            .into_iter()
            .enumerate()
            .for_each(|(_i, (hash, r))| {
                centroids.push(hash);
                point_map.insert(get_cell_id(hash), r);
            });

        let clusters: Vec<SingleVec> = self
            .spawn_child_process(&centroids)?
            .into_iter()
            .filter_map(|c| {
                let hash = get_cell_id(c);
                point_map.remove(&hash)
            })
            .collect();

        let mut final_routes: SingleVec = vec![];

        for (i, current) in clusters.clone().iter_mut().enumerate() {
            let next: &SingleVec = if i == clusters.len() - 1 {
                clusters[0].as_ref()
            } else {
                clusters[i + 1].as_ref()
            };

            let mut shortest = std::f64::MAX;
            let mut shortest_current_index = 0;

            for (current_index, current_point) in current.iter().enumerate() {
                let current_point = Point::new(current_point[1], current_point[0]);
                for (_next_index, next_point) in next.iter().enumerate() {
                    let next_point = Point::new(next_point[1], next_point[0]);
                    let distance = current_point.haversine_distance(&next_point);
                    if distance < shortest {
                        shortest = distance;
                        shortest_current_index = current_index;
                    }
                }
            }
            current.rotate_left(shortest_current_index);
            final_routes.append(current);
        }

        log::info!(
            "full {} time: {}",
            self.plugin,
            time.elapsed().as_secs_f32()
        );
        Ok(final_routes)
    }

    fn spawn_child_process(&self, points: &SingleVec) -> Result<SingleVec, std::io::Error> {
        log::info!("spawning {} child process", self.plugin);
        let time = Instant::now();
        let clusters = points.clone().sort_s2();
        let stringified_points = stringify_points(&clusters);

        let mut child = if self.interpreter.is_empty() {
            Command::new(&self.plugin_path)
        } else {
            Command::new(&self.interpreter)
        };
        if !self.interpreter.is_empty() {
            child.arg(&self.plugin_path);
        }
        let mut child = match child
            .args(&["--input", &stringified_points])
            .args(&["--radius", &self.radius.to_string()])
            .args(&["--route_split_level", &self.route_split_level.to_string()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(err) => return Err(err),
        };

        let mut stdin = match child.stdin.take() {
            Some(stdin) => stdin,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to open stdin",
                ));
            }
        };

        std::thread::spawn(
            move || match stdin.write_all(stringified_points.as_bytes()) {
                Ok(_) => match stdin.flush() {
                    Ok(_) => {}
                    Err(err) => {
                        log::error!("failed to flush stdin: {}", err);
                    }
                },
                Err(err) => {
                    log::error!("failed to write to stdin: {}", err)
                }
            },
        );

        let output = match child.wait_with_output() {
            Ok(result) => result,
            Err(err) => return Err(err),
        };
        let output = String::from_utf8_lossy(&output.stdout);
        let output_indexes = output
            .split(",")
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect::<Vec<usize>>();

        if output_indexes.is_empty() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "no valid output from child process {}, output should return comma separated indexes of the input clusters in the order they should be routed",
                    output
                ),
            ))
        } else {
            log::info!(
                "{} child process finished in {}s",
                self.plugin,
                time.elapsed().as_secs_f32()
            );
            Ok(output_indexes.into_iter().map(|i| clusters[i]).collect())
        }
    }
}
