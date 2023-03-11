use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

use geo::{Coord, HaversineDistance, Point};
use geohash::encode;

use crate::utils;
use model::api::{point_array::PointArray, single_vec::SingleVec};

pub fn multi(clusters: &SingleVec, route_split_level: usize) -> SingleVec {
    let time = Instant::now();

    let get_hash = |point: PointArray, modifier: usize| {
        encode(
            Coord {
                x: point[1],
                y: point[0],
            },
            route_split_level + modifier,
        )
        .unwrap()
    };

    let mut point_map: HashMap<String, SingleVec> = HashMap::new();
    for point in clusters.into_iter() {
        let key = get_hash(*point, 0);
        point_map
            .entry(key)
            .and_modify(|x| x.push(*point))
            .or_insert(vec![*point]);
    }

    let merged_routes: Vec<(PointArray, SingleVec)> = point_map
        .iter()
        .enumerate()
        .map(|(i, (hash, segment))| {
            log::debug!("Creating thread: {} for hash {}", i + 1, hash);
            let mut route = or_tools(&segment);
            if let Some(last) = route.last() {
                if let Some(first) = route.first() {
                    if first == last {
                        route.pop();
                    }
                }
            }
            (utils::centroid(&route), route)
        })
        .collect();
    let mut centroids = vec![];

    point_map.clear();
    merged_routes
        .into_iter()
        .enumerate()
        .for_each(|(_i, (hash, r))| {
            centroids.push(hash);
            point_map.insert(get_hash(hash, 0), r);
        });

    let clusters: Vec<SingleVec> = or_tools(&centroids)
        .into_iter()
        .filter_map(|c| {
            let hash = get_hash(c, 0);
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

    log::info!("[TSP] time: {}", time.elapsed().as_secs_f32());
    final_routes
}

fn directory() -> std::io::Result<String> {
    let path = std::env::current_dir()?;
    Ok(path.display().to_string())
}

pub fn get_or_tools_distance_matrix(points: &SingleVec) -> String {
    points
        .into_iter()
        .map(|cluster| {
            let point = Point::new(cluster[1], cluster[0]);
            format!(
                "{}___,",
                points
                    .iter()
                    .map(|cluster_2| {
                        let point_2 = Point::new(cluster_2[1], cluster_2[0]);
                        format!("{},", point.haversine_distance(&point_2))
                    })
                    .collect::<String>()
            )
        })
        .collect::<String>()
}

pub fn or_tools(clusters: &SingleVec) -> SingleVec {
    let time = Instant::now();
    log::debug!("[TSP] Starting");
    let mut result = vec![];

    let distance_matrix = get_or_tools_distance_matrix(clusters);

    if let Ok(dir) = directory() {
        let full_dir = format!("{}algorithms/src/routing/tsp", dir);

        let mut child = Command::new(&full_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process");

        let mut stdin = child.stdin.take().expect("Failed to open stdin");
        std::thread::spawn(move || {
            stdin
                .write_all(distance_matrix.as_bytes())
                .expect("Failed to write to stdin");
        });

        let output = child.wait_with_output().expect("Failed to read stdout");
        let output = String::from_utf8_lossy(&output.stdout);
        let output = output
            .split(",")
            .filter_map(|s| s.parse::<usize>().ok())
            .collect::<Vec<usize>>();

        output.iter().for_each(|i| {
            result.push(clusters[*i]);
        });
    }
    log::debug!("[TSP] Finished in {}s", time.elapsed().as_secs_f32());
    result
}
