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
                key.clone(),
                PointInfo {
                    coord: info.coord,
                    circles: HashSet::<String>::new(),
                },
            );
        }
    });

    for (point_key, point_info) in new_point_map.clone().into_iter() {
        if point_seen_map.contains(&point_key) {
            continue;
        }
        let mut points: HashSet<String> = HashSet::new();
        for (point_key_2, point_info_2) in new_point_map.clone().into_iter() {
            if point_seen_map.contains(&point_key_2) {
                continue;
            }
            if point_info.coord.vincenty_inverse(&point_info_2.coord) <= radius {
                points.insert(point_key_2);
            }
        }
        if points.len() >= min_points {
            for point in points.clone().into_iter() {
                point_seen_map.insert(point);
            }
            circle_map.insert(
                point_key,
                CircleInfo {
                    coord: point_info.coord,
                    bbox: BBox::new(None),
                    points,
                    unique: HashSet::new(),
                },
            );
        }
    }
    if circle_map.len() - initial_size > 0 {
        println!(
            "Added {} missing points, if this number gets too high, file an issue on GitHub\nTODO: FIX",
            circle_map.len() - initial_size
        );
    }
    println!(
        "Stage 4 time: {}s | Circles: {}",
        time.elapsed().as_secs_f32(),
        circle_map.len()
    );
}
