use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;
use std::vec;

use geo::{HaversineDistance, Point};
use s2::cellid::CellID;
use s2::latlng::LatLng;

use crate::s2::create_cell_map;
use crate::utils;
use model::api::{point_array::PointArray, single_vec::SingleVec};

use super::basic::ClusterSorting;

pub fn run(clusters: SingleVec, route_split_level: u64) -> SingleVec {
    log::info!("starting TSP...");
    let time = Instant::now();

    if route_split_level < 2 {
        return or_tools(clusters);
    }
    let get_cell_id = |point: PointArray| {
        CellID::from(LatLng::from_degrees(point[0], point[1]))
            .parent(route_split_level)
            .0
    };
    let merged_routes: Vec<(PointArray, SingleVec)> =
        create_cell_map(&clusters, route_split_level as u64)
            .into_iter()
            .enumerate()
            .map(|(i, (cell_id, segment))| {
                log::debug!("Creating thread: {} for hash {}", i + 1, cell_id);
                let mut route = or_tools(segment);
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

    let clusters: Vec<SingleVec> = or_tools(centroids)
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

    log::info!("full tsp time: {}", time.elapsed().as_secs_f32());
    final_routes
}

fn directory() -> std::io::Result<String> {
    let mut path = std::env::current_dir()?;
    path.push("algorithms");
    path.push("src");
    path.push("routing");
    path.push("tsp");
    if path.exists() {
        Ok(path.display().to_string())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "TSP solver does not exist, rerun the OR Tools Script",
        ))
    }
}

fn stringify_points(points: &SingleVec) -> String {
    points
        .iter()
        .enumerate()
        .map(|(i, cluster)| {
            format!(
                "{},{}, {}",
                cluster[0],
                cluster[1],
                if i == points.len() - 1 { "" } else { "," }
            )
        })
        .collect()
}

fn spawn_tsp(dir: String, clusters: &SingleVec) -> Result<SingleVec, std::io::Error> {
    log::info!("spawning TSP child process");
    let time = Instant::now();
    let clusters = clusters.sort_s2();
    let stringified_points = stringify_points(&clusters);
    let mut child = match Command::new(&dir)
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
                "Failed to open stdin",
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
    let output = output
        .split(",")
        .filter_map(|s| s.parse::<usize>().ok())
        .collect::<Vec<usize>>();

    log::info!(
        "TSP child process finished in {}s",
        time.elapsed().as_secs_f32()
    );
    Ok(output.into_iter().map(|i| clusters[i]).collect())
}

fn or_tools(clusters: SingleVec) -> SingleVec {
    if let Ok(dir) = directory() {
        match spawn_tsp(dir, &clusters) {
            Ok(result) => result,
            Err(err) => {
                log::error!("TSP failed to spawn child process {}", err);
                clusters
            }
        }
    } else {
        log::error!("TSP solver not found, rerun the OR-Tools script to generate it");
        clusters
    }
}
