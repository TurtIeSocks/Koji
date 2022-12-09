use super::*;

pub fn run(
    point_map: &HashMap<String, PointInfo>,
    point_seen_map: &mut HashSet<String>,
    circle_map: &mut HashMap<String, CircleInfo>,
    radius: f64,
    min_points: usize,
) {
    let time = Instant::now();
    let mut new_point_map: HashMap<String, PointInfo> = HashMap::new();

    let initial_size = circle_map.len();

    point_map.into_iter().for_each(|(key, info)| {
        if !point_seen_map.contains(key) {
            new_point_map.insert(
                key.to_string(),
                PointInfo {
                    points: info.points,
                    coord: info.coord,
                    circles: HashSet::<String>::new(),
                },
            );
        }
    });

    for (point_key, point_info) in new_point_map.clone().into_iter() {
        let bbox = BBox::new(&vec![point_info.coord]);
        let mut best_merge = CircleInfo {
            coord: point_info.coord,
            bbox: BBox::default(),
            points: HashSet::new(),
            unique: HashSet::new(),
            meets_min: false,
        };
        let mut best_merge_key = "".to_string();

        for (circle_key, circle_info) in circle_map.clone().into_iter() {
            let lower_left = Coord {
                x: bbox.min_x.min(circle_info.bbox.min_x),
                y: bbox.min_y.min(circle_info.bbox.min_y),
            };
            // UR of the circle and its neighbor
            let upper_right = Coord {
                x: bbox.max_x.max(circle_info.bbox.max_x),
                y: bbox.max_y.max(circle_info.bbox.max_y),
            };
            let distance = lower_left.vincenty_inverse(&upper_right);

            // Checks whether the LL and UR points are within the circle circumference
            if distance <= radius * 2. {
                // New coord from the midpoint of the LL and UR points
                let coord = lower_left.midpoint(&upper_right);
                // Combine the points from the circle and its neighbor, ensuring uniqueness
                let new_points = circle_info.combine().len() + 1;

                if new_points > best_merge.combine().len() && new_points >= min_points {
                    let mut unique = circle_info.unique;
                    unique.insert(point_key.clone());
                    best_merge = CircleInfo {
                        coord,
                        unique,
                        meets_min: true,
                        ..circle_info
                    };
                    best_merge_key = circle_key;
                }
            } else if distance <= radius * 2. + 10. {
                // New coord from the midpoint of the LL and UR points
                let coord = lower_left.midpoint(&upper_right);
                // Combine the points from the circle and its neighbor, ensuring uniqueness
                let new_points = circle_info.combine().len() + 1;

                if new_points > best_merge.combine().len() && new_points >= min_points {
                    let mut unique = circle_info.unique;
                    unique.insert(point_key.clone());
                    best_merge = CircleInfo {
                        coord,
                        unique,
                        meets_min: true,
                        ..circle_info
                    };
                    best_merge_key = circle_key;
                }
            }
        }
        if best_merge_key.is_empty() {
            if min_points == 1 {
                let mut unique = HashSet::new();
                unique.insert(point_key.clone());
                circle_map.insert(
                    point_key.clone(),
                    CircleInfo {
                        coord: point_info.coord,
                        bbox: BBox::new(&vec![point_info.coord]),
                        points: HashSet::new(),
                        unique,
                        meets_min: true,
                    },
                );
                point_seen_map.insert(point_key);
            }
        } else {
            let new_key = encode(best_merge.coord, PRECISION).unwrap();
            circle_map.remove(&best_merge_key);
            circle_map.insert(new_key, best_merge);
            point_seen_map.insert(point_key);
        }
    }
    if circle_map.len() != initial_size {
        println!(
            "Added {} circles to cover the missing points, if this number gets too high, file an issue on GitHub",
            circle_map.len().abs_diff(initial_size)
        );
    }
    println!(
        "Stage 4 time: {}s | Circles: {}",
        time.elapsed().as_secs_f32(),
        circle_map.len()
    );
}
