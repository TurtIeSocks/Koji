use crate::utils;
use koji_core::Precision;
use geo::{Distance, Haversine, Point};
use koji_core::{PointArray, SingleVec};
use koji_plugins::{JoinFunction, Plugin};
use s2::cellid::CellID;
use s2::latlng::LatLng;
use std::collections::HashMap;
use std::time::Instant;

pub fn join(plugin: &Plugin, input: Vec<SingleVec>) -> SingleVec {
    if plugin.split_level == 0 || input.len() < 3 {
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
        let center = utils::centroid(points);
        centroids.push(center);
        point_map.insert(get_cell_id(center), points.clone());
    }
    let clusters: Vec<SingleVec> = plugin
        .run_multi::<JoinFunction>(&centroids, &serde_json::Value::Null, None)
        .unwrap_or(vec![])
        .into_iter()
        .filter_map(|c| {
            let hash = get_cell_id(c);
            point_map.remove(&hash)
        })
        .collect();

    let result = stitch_routes(clusters);
    log::info!(
        "joined {} routes in {}ms",
        result.len(),
        time.elapsed().as_millis()
    );
    result
}

/// Rotate each cluster so its endpoint nearest to the next cluster's first point
/// becomes the first point, then concatenate all clusters into one flat route.
///
/// For a circular tour the last cluster wraps back to clusters[0].
/// Empty `clusters` → empty output. Single-cluster input → rotated as if
/// its successor is itself (wraps to index 0 trivially).
pub(crate) fn stitch_routes(clusters: Vec<SingleVec>) -> SingleVec {
    if clusters.is_empty() {
        return vec![];
    }
    let mut final_routes: SingleVec = vec![];
    let last = clusters.len() - 1;
    for (i, current) in clusters.clone().iter_mut().enumerate() {
        let next: &SingleVec = if i == last {
            clusters[0].as_ref()
        } else {
            clusters[i + 1].as_ref()
        };

        let mut shortest = Precision::MAX;
        let mut shortest_current_index = 0;

        for (current_index, current_point) in current.iter().enumerate() {
            let current_point = Point::new(current_point[1], current_point[0]);
            for next_point in next.iter() {
                let next_point = Point::new(next_point[1], next_point[0]);
                let distance = Haversine.distance(current_point, next_point);
                if distance < shortest {
                    shortest = distance;
                    shortest_current_index = current_index;
                }
            }
        }
        current.rotate_left(shortest_current_index);
        final_routes.append(current);
    }
    final_routes
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── stitch_routes: empty / degenerate ─────────────────────────────────────

    #[test]
    fn stitch_routes_empty_clusters_returns_empty() {
        assert!(stitch_routes(vec![]).is_empty());
    }

    #[test]
    fn stitch_routes_preserves_total_point_count() {
        // 3 clusters, 3 points each → 9 total points out.
        let clusters: Vec<SingleVec> = vec![
            vec![[40.0, -74.0], [40.001, -74.0], [40.002, -74.0]],
            vec![[41.0, -74.0], [41.001, -74.0], [41.002, -74.0]],
            vec![[42.0, -74.0], [42.001, -74.0], [42.002, -74.0]],
        ];
        let out = stitch_routes(clusters);
        assert_eq!(out.len(), 9, "all points must be present after stitching");
    }

    #[test]
    fn stitch_routes_single_cluster_returns_all_points() {
        let cluster: Vec<SingleVec> = vec![vec![[40.0, -74.0], [40.001, -74.0], [40.002, -74.0]]];
        let out = stitch_routes(cluster);
        assert_eq!(out.len(), 3);
    }

    // ── stitch_routes: rotation correctness ───────────────────────────────────

    #[test]
    fn stitch_routes_rotates_to_minimize_gap() {
        // cluster A has 3 points; cluster B is geographically to the north.
        // The point in A closest to any point in B should become the first
        // point of A in the output.
        //
        // A = [(40.0, -74.0), (40.0, -73.9), (40.0, -73.8)]  lon increases east
        // B = [(41.0, -73.8)]  — only one point, so A's closest point is -73.8
        let a: SingleVec = vec![[40.0, -74.0], [40.0, -73.9], [40.0, -73.8]];
        let b: SingleVec = vec![[41.0, -73.8]];
        let out = stitch_routes(vec![a, b]);

        // A should be rotated so [40.0, -73.8] is first (closest to B's [41.0, -73.8]).
        // Output: [40.0,-73.8], [40.0,-74.0], [40.0,-73.9], then [41.0,-73.8]
        assert_eq!(out.len(), 4, "3 + 1 = 4 total");
        assert!(
            (out[0][1] - (-73.8)).abs() < 0.0001,
            "first output point should be the A-point closest to B; got lon={}",
            out[0][1]
        );
    }

    #[test]
    fn stitch_routes_two_clusters_no_reorder_when_already_aligned() {
        // A's first point is already the closest to B → rotate_left(0) → no change.
        let a: SingleVec = vec![[40.0, -74.0], [40.0, -75.0]];
        // B is directly north of A[0].
        let b: SingleVec = vec![[41.0, -74.0]];
        let out = stitch_routes(vec![a.clone(), b]);
        // A[0] = [40.0,-74.0] is 111 km from B; A[1] = [40.0,-75.0] is ~130 km from B.
        // So A[0] should remain first.
        assert_eq!(out.len(), 3);
        assert!(
            (out[0][0] - 40.0).abs() < 1e-6 && (out[0][1] - (-74.0)).abs() < 1e-6,
            "first point should be unchanged (already closest), got {:?}",
            out[0]
        );
    }

    #[test]
    fn stitch_routes_all_lat_lon_in_valid_range() {
        let clusters: Vec<SingleVec> = vec![
            vec![[40.0, -74.0], [40.1, -74.0]],
            vec![[41.0, -74.0], [41.1, -74.0]],
            vec![[42.0, -74.0], [42.1, -74.0]],
        ];
        let out = stitch_routes(clusters);
        for [lat, lon] in &out {
            assert!(lat.abs() < 90.0, "bad lat: {lat}");
            assert!(lon.abs() < 180.0, "bad lon: {lon}");
        }
    }
}
