use super::*;

pub fn run(
    point_map: &mut HashMap<String, PointInfo>,
    circle_map: &mut HashMap<String, CircleInfo>,
    radius: f64,
) {
    let time = Instant::now();

    for (circle_key, circle_info) in helpers::get_sorted(&circle_map) {
        let theoretical_bbox = BBox::new(Some(&circle_info.get_points(point_map, CiKeys::Unique)));

        let approx_key = circle_key[..(APPROX_PRECISION - 2)].to_string();

        let mut keys = circle_map
            .clone()
            .into_keys()
            .filter_map(|neighbor_key| {
                if neighbor_key[..(APPROX_PRECISION - 2)] == approx_key {
                    Some(neighbor_key)
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        keys.sort();
        let mut best_neighbor = CircleInfo {
            coord: Coordinate { x: 0., y: 0. },
            bbox: BBox::new(Some(&vec![circle_info.coord])),
            points: HashSet::new(),
            unique: HashSet::new(),
        };
        let mut best_neighbor_key = "".to_string();

        for neighbor_key in keys {
            if neighbor_key == circle_key.to_string() {
                continue;
            }
            if let Some(found_neighbor) = circle_map.get(&neighbor_key) {
                // LL of the circle and its neighbor
                let lower_left = Coordinate {
                    x: theoretical_bbox.min_x.min(found_neighbor.bbox.min_x),
                    y: theoretical_bbox.min_y.min(found_neighbor.bbox.min_y),
                };
                // UR of the circle and its neighbor
                let upper_right = Coordinate {
                    x: theoretical_bbox.max_x.max(found_neighbor.bbox.max_x),
                    y: theoretical_bbox.max_y.max(found_neighbor.bbox.max_y),
                };
                let distance = lower_left.vincenty_inverse(&upper_right);

                // Checks whether the LL and UR points are within the circle circumference
                if distance <= radius * 2. {
                    // New coord from the midpoint of the LL and UR points
                    let new_coord = lower_left.midpoint(&upper_right);

                    // Combine the points from the circle and its neighbor, ensuring uniqueness
                    let mut new_points = circle_info.combine();
                    new_points.extend(found_neighbor.combine());

                    if new_points.len() > best_neighbor.combine().len() {
                        best_neighbor_key = neighbor_key;
                        best_neighbor.points = new_points;
                        best_neighbor.coord = new_coord;
                    }
                } else if distance <= radius * 2. + 10. {
                    // New coord from the midpoint of the LL and UR points
                    let new_coord = lower_left.midpoint(&upper_right);

                    // Combine the points from the circle and its neighbor, ensuring uniqueness
                    let mut new_points = circle_info.combine();
                    new_points.extend(found_neighbor.combine());

                    if new_points.len() > best_neighbor.combine().len() {
                        if new_points.iter().all(|p| {
                            let point_info = point_map.get(p).unwrap();
                            point_info.coord.vincenty_inverse(&new_coord) <= radius
                        }) {
                            best_neighbor_key = neighbor_key;
                            best_neighbor.points = new_points;
                            best_neighbor.coord = new_coord;
                        }
                    }
                }
            }
        }
        if !best_neighbor_key.is_empty() {
            helpers::sync_maps(
                circle_map,
                circle_key,
                best_neighbor_key,
                best_neighbor,
                point_map,
                radius,
                true,
            );
        }
    }
    println!(
        "Stage 3 time: {}s | Circles: {}",
        time.elapsed().as_secs_f32(),
        circle_map.len()
    );
}
