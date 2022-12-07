use super::*;

pub fn sync_maps(
    circle_map: &mut HashMap<String, CircleInfo>,
    circle_key: String,
    best_neighbor_key: String,
    best_neighbor: &mut CircleInfo,
    point_map: &mut HashMap<String, PointInfo>,
    radius: f64,
    remove_at_start: bool,
    min_points: usize,
) {
    let new_key = encode(best_neighbor.coord, PRECISION).unwrap();
    if remove_at_start {
        circle_map.remove(&best_neighbor_key);
        circle_map.remove(&circle_key);
    }
    let mut unique = HashSet::new();
    let mut points = HashSet::new();
    let mut bbox_points: Vec<Coord> = best_neighbor
        .points
        .iter()
        .filter_map(|x| {
            if let Some(point) = point_map.get_mut(x) {
                let distance = point.coord.vincenty_inverse(&best_neighbor.coord);
                if distance <= radius {
                    point.circles.remove(&circle_key);
                    point.circles.remove(&best_neighbor_key);
                    point.circles.insert(new_key.clone());
                    if point.circles.len() == 1 {
                        unique.insert(x.to_string());
                    } else {
                        points.insert(x.to_string());
                    }
                    return Some(point.coord);
                }
                None
            } else {
                None
            }
        })
        .collect();
    for (key, info) in point_map.clone().into_iter() {
        if key[..APPROX_PRECISION] == new_key[..APPROX_PRECISION] {
            if info.coord.vincenty_inverse(&best_neighbor.coord) <= radius {
                if info.circles.is_empty() {
                    unique.insert(key);
                } else {
                    points.insert(key);
                }
                bbox_points.push(info.coord);
            }
        }
    }
    let info = circle_map.entry(new_key.clone()).or_insert(CircleInfo {
        bbox: BBox::new(Some(&bbox_points)),
        points: points.clone(),
        unique: unique.clone(),
        coord: best_neighbor.coord,
        meets_min: (points.len() + unique.len()) >= min_points,
    });
    for key in info.combine() {
        if let Some(point) = point_map.get(&key) {
            for cir in point.circles.iter() {
                if cir == &new_key {
                    continue;
                }
                if let Some(circle) = circle_map.get_mut(cir) {
                    if circle.unique.contains(&key) {
                        circle.unique.remove(&key);
                        circle.points.insert(key.to_string());
                    }
                }
            }
        }
    }
    if !remove_at_start {
        circle_map.remove(&circle_key);
        circle_map.remove(&best_neighbor_key);
    }
}

pub fn get_sorted<T>(map: &HashMap<String, T>) -> Vec<(String, T)>
where
    T: Clone,
{
    let mut vec: Vec<&String> = map.keys().collect();
    vec.sort();
    vec.into_iter()
        .map(|k| (k.clone(), map.get(k).unwrap().clone()))
        .collect()
}

// pub fn centroid(coords: &Vec<Coord>) -> Coord {
//     let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);

//     for loc in coords.iter() {
//         let lat = loc.y.to_radians();
//         let lon = loc.x.to_radians();

//         x += lat.cos() * lon.cos();
//         y += lat.cos() * lon.sin();
//         z += lat.sin();
//     }

//     let number_of_locations = coords.len() as f64;
//     x /= number_of_locations;
//     y /= number_of_locations;
//     z /= number_of_locations;

//     let hyp = (x * x + y * y).sqrt();
//     let lon = y.atan2(x);
//     let lat = z.atan2(hyp);

//     Coord {
//         y: lat.to_degrees(),
//         x: lon.to_degrees(),
//     }
// }
