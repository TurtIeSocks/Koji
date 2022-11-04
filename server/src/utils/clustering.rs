use geo::Coordinate;
use rstar::PointDistance;
use std::{collections::HashMap, time::Instant};

#[derive(Debug, Clone)]
struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

trait NewAndUpdate {
    fn new(point: Coordinate) -> BoundingBox;
    fn update(&self, point: Coordinate) -> BoundingBox;
}

impl NewAndUpdate for BoundingBox {
    fn new(point: Coordinate) -> BoundingBox {
        BoundingBox {
            min_x: point.x.min(f64::INFINITY),
            min_y: point.y.min(f64::INFINITY),
            max_x: point.x.max(f64::NEG_INFINITY),
            max_y: point.y.max(f64::NEG_INFINITY),
        }
    }
    fn update(&self, point: Coordinate) -> BoundingBox {
        BoundingBox {
            min_x: self.min_x.min(point.x),
            min_y: self.min_y.min(point.y),
            max_x: self.max_x.max(point.x),
            max_y: self.max_y.max(point.y),
        }
    }
}

trait ToKey {
    fn to_key(self) -> String;
}

impl ToKey for Coordinate {
    fn to_key(self) -> String {
        format!("{},{}", self.x, self.y)
    }
}

trait FromKey {
    fn from_key(self) -> [f64; 2];
}

impl FromKey for String {
    fn from_key(self) -> [f64; 2] {
        let mut iter = self.split(',');
        let x = iter.next().unwrap().parse::<f64>().unwrap();
        let y = iter.next().unwrap().parse::<f64>().unwrap();
        [x, y]
    }
}

trait Midpoint {
    fn midpoint(&self, other: &Coordinate) -> [f64; 2];
}

impl Midpoint for Coordinate {
    fn midpoint(&self, other: &Coordinate) -> [f64; 2] {
        [(self.x + other.x) / 2., (self.y + other.y) / 2.]
    }
}

fn try_to_merge(
    point_map: &HashMap<String, (BoundingBox, bool)>,
    point: (&String, &(BoundingBox, bool)),
    v: f64,
    h: f64,
    clusters: &mut Vec<[f64; 2]>,
    point_map_2: &mut HashMap<String, (BoundingBox, bool)>,
) -> bool {
    let found_cluster = point_map.get(&format!("{},{}", v, h));
    if found_cluster.is_none() {
        return false;
    }
    let found_cluster = found_cluster.unwrap();

    if found_cluster.1 {
        let min_x = if point.1 .0.min_x < found_cluster.0.min_x {
            point.1 .0.min_x
        } else {
            found_cluster.0.min_x
        };
        let min_y = if point.1 .0.min_y < found_cluster.0.min_y {
            point.1 .0.min_y
        } else {
            found_cluster.0.min_y
        };
        let max_x = if point.1 .0.max_x > found_cluster.0.max_x {
            point.1 .0.max_x
        } else {
            found_cluster.0.max_x
        };
        let max_y = if point.1 .0.max_y > found_cluster.0.max_y {
            point.1 .0.max_y
        } else {
            found_cluster.0.max_y
        };

        let lower_left = Coordinate { x: min_x, y: min_y };
        let upper_right = Coordinate { x: max_x, y: max_y };

        if lower_left.distance_2(&upper_right) <= 4. {
            clusters.push(lower_left.midpoint(&upper_right));
            // point_map_2
            //     .entry(format!("{},{}", v, h))
            //     .and_modify(|saved| saved.1 = false);
            // point_map_2
            //     .entry(point.0.to_string())
            //     .and_modify(|saved| saved.1 = false);
            return true;
        }
    }
    false
}

fn update(
    point_map: &mut HashMap<(i32, i32), (BoundingBox, bool, i32)>,
    key: (i32, i32),
    p: &Coordinate,
) {
    point_map.entry(key).and_modify(|saved| {
        saved.0 = saved.0.update(*p);
        saved.2 += 1;
    });
}

pub fn udc(points: Vec<Coordinate>, min: i32) -> Vec<[f64; 2]> {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = (sqrt2 * 1.5) - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = (sqrt2 * 1.5) + 1.;

    let time = Instant::now();
    let mut point_map: HashMap<(i32, i32), (BoundingBox, bool, i32)> = HashMap::new();
    // let mut cluster_map: HashMap<(i32, i32), Vec<[f64; 2]>> = HashMap::new();

    let mut clusters = Vec::<[f64; 2]>::new();

    for p in points.iter() {
        let v = (p.x / sqrt2).floor() as i32;
        let h = (p.y / sqrt2).floor() as i32;
        let vertical_times_sqrt2 = v as f64 * sqrt2;
        let horizontal_times_sqrt2 = h as f64 * sqrt2;
        let key = (v, h);

        let mut pair = point_map.get(&key);

        if pair.is_some() {
            update(&mut point_map, key, p);
            continue;
        }

        if p.x >= (vertical_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v + 1) as f64 + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                update(&mut point_map, key, p);
                continue;
            }
        }

        if p.x <= (vertical_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v - 1) as f64 + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                update(&mut point_map, key, p);
                continue;
            }
        }

        if p.y <= (horizontal_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h - 1) as f64 + additive_factor,
                }) <= 1.
            {
                update(&mut point_map, key, p);
                continue;
            }
        }

        if p.y >= (horizontal_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h + 1) as f64 + additive_factor,
                }) <= 1.
            {
                update(&mut point_map, key, p);
                continue;
            }
        }
        point_map
            .entry(key)
            .or_insert((BoundingBox::new(*p), true, 1));
    }

    for (key, value) in point_map.iter() {
        if value.1 && value.2 >= min {
            clusters.push([
                key.0 as f64 * sqrt2 + additive_factor,
                key.1 as f64 * sqrt2 + additive_factor,
            ]);
        }
    }
    println!("Clusters Made: {:?}", clusters.len());
    println!("Clustering Time: {:?}", time.elapsed().as_secs_f64());

    clusters
}
