use model::api::single_vec::SingleVec;
use std::collections::HashSet;

use self::project::Plane;

mod project;
mod udc;

trait FromKey {
    fn from_key(&self) -> [f64; 2];
}

impl FromKey for String {
    fn from_key(&self) -> [f64; 2] {
        let mut iter = self.split(',');
        let lat = iter.next().unwrap().parse::<f64>().unwrap();
        let lon = iter.next().unwrap().parse::<f64>().unwrap();
        [lat, lon]
    }
}

pub fn cluster(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let output = plane.project();

    let point_map = udc::cluster(output, min_points);

    let output = {
        let mut seen_map: HashSet<String> = HashSet::new();
        let return_value: SingleVec = point_map
            .into_iter()
            .filter_map(|(key, values)| {
                if values.len() >= min_points {
                    for point in values.into_iter() {
                        seen_map.insert(point);
                    }
                    return Some(key.from_key());
                }
                None
            })
            .collect();
        return_value
    };

    plane.reverse(output)
}
