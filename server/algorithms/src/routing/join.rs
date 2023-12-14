use crate::plugin::{JoinFunction, Plugin};
use crate::utils;
use geo::{HaversineDistance, Point};
use model::api::{point_array::PointArray, single_vec::SingleVec};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use std::collections::HashMap;
use std::time::Instant;

pub fn join(plugin: &Plugin, input: Vec<SingleVec>) -> SingleVec {
    if plugin.split_level == 0 {
        return input.into_iter().flatten().collect();
    }
    let time = Instant::now();
    let mut point_map = HashMap::<u64, SingleVec>::new();

    let get_cell_id = |point: PointArray| {
        CellID::from(LatLng::from_degrees(point[0], point[1]))
            .parent(plugin.split_level)
            .0
    };

    let mut centroids = vec![];
    for points in input.iter() {
        let center = utils::centroid(&points);
        centroids.push(center);
        point_map.insert(get_cell_id(center), points.clone());
    }
    let clusters: Vec<SingleVec> = plugin
        .run_multi::<JoinFunction>(&centroids, None)
        .unwrap_or(vec![])
        .into_iter()
        .filter_map(|c| {
            let hash = get_cell_id(c);
            point_map.remove(&hash)
        })
        .collect();

    let mut final_routes: SingleVec = vec![];

    let last = clusters.len() - 1;
    for (i, current) in clusters.clone().iter_mut().enumerate() {
        let next: &SingleVec = if i == last {
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
        "joined {} routes in {}ms",
        final_routes.len(),
        time.elapsed().as_millis()
    );
    final_routes
}
