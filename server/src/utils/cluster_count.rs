use geo::Coordinate;
use std::{collections::HashMap, time::Instant};

trait ToKey {
    fn to_key(self) -> String;
}

impl ToKey for [f64; 2] {
    fn to_key(self) -> String {
        format!("{},{}", self[0], self[1])
    }
}

trait FromKey {
    fn from_key(self) -> [f64; 2];
}

impl FromKey for String {
    fn from_key(self) -> [f64; 2] {
        let mut iter = self.split(',');
        let lat = iter.next().unwrap().parse::<f64>().unwrap();
        let lon = iter.next().unwrap().parse::<f64>().unwrap();
        [lat, lon]
    }
}

pub fn count(
    points: Vec<Coordinate>,
    clusters: Vec<[f64; 2]>,
    _radius: f64,
    min: i32,
) -> Vec<[f64; 2]> {
    let mut filtered_clusters: HashMap<String, [f64; 2]> = HashMap::new();

    // point_keys, visited
    let mut visited_map: HashMap<String, bool> = HashMap::new();

    let first_loop = Instant::now();

    let mut cluster_map: HashMap<_, _> = clusters
        .iter()
        .map(|x| (x.to_key(), Vec::<[f64; 2]>::new()))
        .collect();

    let mut point_map: HashMap<_, _> = points
        .iter()
        .map(|x| ([x.x, x.y].to_key(), Vec::<String>::new()))
        .collect();

    // Loop through the clusters
    for cluster_key in clusters.iter() {
        // Create a temp cluster map of all of the points known to be within loc_1
        let mut count: Vec<[f64; 2]> = Vec::new();
        for point in points.iter() {
            let point_key = [point.x, point.y];
            // Check the distance between loc_1 and loc_2
            if distance(*cluster_key, point_key) {
                count.push(point_key);
                // visited_map.insert(point_key.to_string(), true);

                // Create a points map to know where to find the rest
                if point_map.contains_key(&point_key.to_key()) {
                    point_map
                        .get_mut(&point_key.to_key())
                        .unwrap()
                        .push(cluster_key.to_key());
                } else {
                    point_map.insert(point_key.to_key(), vec![cluster_key.to_key()]);
                }
            }
        }
        // insert the temp cluster for loc_1
        cluster_map.insert(cluster_key.to_key(), count);
    }
    println!("First Time: {:?}", first_loop.elapsed());
    let second_loop = Instant::now();

    for (point_key, value) in point_map.iter() {
        let mut best_count: usize = min as usize - 1;
        let mut best_cluster: String = "".to_string();
        for cluster_key in value.clone().iter() {
            let filter_points: Vec<[f64; 2]> = cluster_map
                .get(cluster_key)
                .unwrap()
                .iter()
                .filter(|f| visited_map.get(&(*f).to_key()).is_none())
                .map(|f| *f)
                .collect();
            if filter_points.len() > best_count {
                best_count = filter_points.len();
                best_cluster = cluster_key.to_string();
            }
        }
        if !best_cluster.is_empty() {
            for point in cluster_map.get(&best_cluster).unwrap() {
                visited_map.insert((*point).to_key(), true);
            }
            if best_count == 1 {
                filtered_clusters.insert(point_key.to_string(), point_key.to_string().from_key());
            } else {
                filtered_clusters.insert(best_cluster.to_string(), best_cluster.from_key());
            }
        }
    }
    println!("Second Time: {:?}", second_loop.elapsed());
    filtered_clusters.values().cloned().collect()
}

fn distance(a: [f64; 2], b: [f64; 2]) -> bool {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let d = dx * dx + dy * dy;
    d < 1.5
}
