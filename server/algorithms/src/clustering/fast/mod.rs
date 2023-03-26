use model::api::{single_vec::SingleVec, stats::Stats, Precision};
use std::{collections::HashSet, time::Instant};

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

pub fn cluster(
    input: SingleVec,
    radius: f64,
    min_points: usize,
    stats: &mut Stats,
) -> Vec<[f64; 2]> {
    let time = Instant::now();
    let plane = Plane::new(input).radius(radius);
    let output = plane.project();

    let point_map = udc::cluster(output, min_points);

    let mut best_clusters = vec![];
    let output = {
        let mut seen_map: HashSet<String> = HashSet::new();
        let return_value: SingleVec = point_map
            .into_iter()
            .filter_map(|(key, values)| {
                if values.len() >= min_points {
                    if values.len() >= stats.best_cluster_point_count {
                        if values.len() != stats.best_cluster_point_count {
                            best_clusters = vec![];
                            stats.best_cluster_point_count = values.len();
                        }
                        best_clusters.push(key.from_key());
                    }
                    for point in values.into_iter() {
                        seen_map.insert(point);
                    }
                    return Some(key.from_key());
                }
                None
            })
            .collect();
        stats.points_covered = seen_map.len();
        stats.total_clusters = return_value.len();
        return_value
    };

    stats.best_clusters = plane.reverse(best_clusters);
    stats.distance(&output);
    stats.set_cluster_time(time.elapsed().as_secs_f32() as Precision);

    plane.reverse(output)
}
