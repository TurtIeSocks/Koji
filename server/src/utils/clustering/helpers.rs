use super::*;

pub fn sync_maps(
    circle_map: &mut HashMap<String, CircleInfo>,
    circle_key: String,
    best_neighbor_key: String,
    best_neighbor: CircleInfo,
    point_map: &mut HashMap<String, PointInfo>,
    radius: f64,
    remove_at_start: bool,
) {
    if remove_at_start {
        circle_map.remove(&circle_key);
        circle_map.remove(&best_neighbor_key);
    }
    let new_key = encode(best_neighbor.coord, PRECISION).unwrap();
    let mut unique = HashSet::new();
    let mut points = HashSet::new();
    let info = circle_map.entry(new_key.clone()).or_insert(CircleInfo {
        bbox: BBox::new(Some(
            &best_neighbor
                .points
                .iter()
                .filter_map(|x| {
                    if let Some(point) = point_map.get_mut(x) {
                        if point.coord.vincenty_inverse(&best_neighbor.coord) <= radius {
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
                .collect(),
        )),
        points,
        unique,
        ..best_neighbor
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
