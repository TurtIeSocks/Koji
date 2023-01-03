use super::*;

use crate::clustering::brute::helpers::Helpers;

pub fn run(
    circle_map: &mut HashMap<String, CircleInfo>,
    point_map: &mut HashMap<String, PointInfo>,
    radius: f64,
    min_points: usize,
) {
    let time = Instant::now();
    let neighbor_distance = 0.75_f64.sqrt() * 2. * radius;

    for (circle_key, circle_info) in helpers::get_sorted(circle_map) {
        let mut best_neighbor = CircleInfo {
            coord: Coord { x: 0., y: 0. },
            bbox: BBox::new(&vec![circle_info.coord]),
            points: HashSet::new(),
            unique: HashSet::new(),
            meets_min: false,
        };
        let mut best_neighbor_key = "".to_string();
        let mut all_points: HashSet<String> = circle_info.combine();

        for bearing in [30., 90., 150., 210., 270., 330.] {
            // for neighbor_key in keys {
            if let Some(circle_info) = circle_map.get(&circle_key) {
                let point: Point = circle_info.coord.into();
                let neighbor_point = point.haversine_destination(bearing, neighbor_distance);
                let neighbor_key = encode(neighbor_point.into(), PRECISION).unwrap();

                // If the circle map already has the neighbor entry
                if let Some(found_neighbor) = circle_map.get(&neighbor_key) {
                    // LL of the circle and its neighbor
                    let lower_left = Coord {
                        x: circle_info.bbox.min_x.min(found_neighbor.bbox.min_x),
                        y: circle_info.bbox.min_y.min(found_neighbor.bbox.min_y),
                    };
                    // UR of the circle and its neighbor
                    let upper_right = Coord {
                        x: circle_info.bbox.max_x.max(found_neighbor.bbox.max_x),
                        y: circle_info.bbox.max_y.max(found_neighbor.bbox.max_y),
                    };
                    let distance = lower_left.vincenty_inverse(&upper_right);

                    all_points.extend(found_neighbor.combine());
                    // Checks whether the LL and UR points are within the circle circumference
                    if distance <= radius * 2. {
                        // New coord from the midpoint of the LL and UR points
                        let new_coord = lower_left.midpoint(&upper_right);

                        // Combine the points from the circle and its neighbor, ensuring uniqueness
                        let mut new_points = circle_info.combine();
                        new_points.extend(found_neighbor.combine());

                        if new_points.len() > best_neighbor.combine().len()
                            && new_points.len() >= min_points
                        {
                            best_neighbor_key = neighbor_key;
                            best_neighbor.coord = new_coord;
                            best_neighbor.meets_min = true;
                        }
                    } else if distance <= radius * 2. + 10. {
                        // New coord from the midpoint of the LL and UR points
                        let new_coord = lower_left.midpoint(&upper_right);

                        // Combine the points from the circle and its neighbor, ensuring uniqueness
                        let mut new_points = circle_info.combine();
                        new_points.extend(found_neighbor.combine());

                        if new_points.len() > best_neighbor.combine().len()
                            && new_points.len() >= min_points
                        {
                            if new_points.iter().all(|p| {
                                let point_info = point_map.get(p).unwrap();
                                point_info.coord.vincenty_inverse(&new_coord) <= radius
                            }) {
                                best_neighbor_key = neighbor_key;
                                best_neighbor.coord = new_coord;
                                best_neighbor.meets_min = true;
                            }
                        }
                    }
                }
            }
        }
        if !best_neighbor_key.is_empty() {
            let mut best_neighbor = CircleInfo {
                points: all_points,
                ..best_neighbor
            };
            helpers::sync_maps(
                circle_map,
                circle_key,
                best_neighbor_key,
                &mut best_neighbor,
                point_map,
                radius,
                false,
                min_points,
            );
        }
    }
    println!(
        "Stage 2 time: {}s | Circles: {}",
        time.elapsed().as_secs_f32(),
        circle_map.len()
    );
}
