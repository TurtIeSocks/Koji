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

trait BBTraits {
    fn new(point: Coordinate) -> BoundingBox;
    fn update(&self, point: Coordinate) -> BoundingBox;
}

impl BBTraits for BoundingBox {
    fn new(point: Coordinate) -> BoundingBox {
        BoundingBox {
            min_x: if point.x < f64::INFINITY {
                point.x
            } else {
                f64::INFINITY
            },
            min_y: if point.y < f64::INFINITY {
                point.y
            } else {
                f64::INFINITY
            },
            max_x: if point.x > f64::NEG_INFINITY {
                point.x
            } else {
                f64::NEG_INFINITY
            },
            max_y: if point.y > f64::NEG_INFINITY {
                point.y
            } else {
                f64::NEG_INFINITY
            },
        }
    }
    fn update(&self, point: Coordinate) -> BoundingBox {
        BoundingBox {
            min_x: if point.x < self.min_x {
                point.x
            } else {
                self.min_x
            },
            min_y: if point.y < self.min_y {
                point.y
            } else {
                self.min_y
            },
            max_x: if point.x > self.max_x {
                point.x
            } else {
                self.max_x
            },
            max_y: if point.y > self.max_y {
                point.y
            } else {
                self.max_y
            },
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
            point_map_2
                .entry(format!("{},{}", v, h))
                .and_modify(|saved| saved.1 = false);
            point_map_2
                .entry(point.0.to_string())
                .and_modify(|saved| saved.1 = false);
            return true;
        }
    }
    false
}

pub fn udc(points: Vec<Coordinate>) -> Vec<[f64; 2]> {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = (sqrt2 * 1.5) - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = (sqrt2 * 1.5) + 1.;

    let time = Instant::now();
    let mut point_map: HashMap<String, (BoundingBox, bool)> = HashMap::new();

    let mut clusters = Vec::<[f64; 2]>::new();

    for p in points.iter() {
        let v = (p.x / sqrt2).floor();
        let h = (p.y / sqrt2).floor();
        let vertical_times_sqrt2 = v * sqrt2;
        let horizontal_times_sqrt2 = h * sqrt2;

        let mut pair = point_map.get(&format!("{},{}", v, h));

        if pair.is_some() {
            point_map
                .entry(p.to_key())
                .and_modify(|saved| saved.0 = saved.0.update(*p));
            continue;
        }

        if p.x >= (vertical_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = point_map.get(&format!("{},{}", v + 1., h));
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v + 1.) + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                point_map
                    .entry(format!("{},{}", v + 1., h))
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.x <= (vertical_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = point_map.get(&format!("{},{}", v - 1., h));
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v - 1.) + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                point_map
                    .entry(format!("{},{}", v - 1., h))
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.y <= (horizontal_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = point_map.get(&format!("{},{}", v, h - 1.));
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h - 1.) + additive_factor,
                }) <= 1.
            {
                point_map
                    .entry(format!("{},{}", v, h - 1.))
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.y >= (horizontal_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = point_map.get(&format!("{},{}", v, h + 1.));
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h + 1.) + additive_factor,
                }) <= 1.
            {
                point_map
                    .entry(format!("{},{}", v, h + 1.))
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }
        point_map
            .entry(format!("{},{}", v, h))
            .or_insert((BoundingBox::new(*p), true));
    }

    let mut point_map_2 = point_map.clone();

    for point in point_map.iter() {
        let [v, h] = point.0.clone().from_key();

        if !point.1 .1 {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v,
            h - 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v,
            h + 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v + 1.,
            h,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v - 1.,
            h,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v - 1.,
            h - 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v + 1.,
            h - 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v + 1.,
            h + 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
        if try_to_merge(
            &point_map,
            point,
            v - 1.,
            h + 1.,
            &mut clusters,
            &mut point_map_2,
        ) {
            continue;
        }
    }
    for (key, value) in point_map_2.iter() {
        if value.1 {
            let [x, y] = key.clone().from_key();
            clusters.push([x * sqrt2 + additive_factor, y * sqrt2 + additive_factor]);
        }
    }
    println!("Clusters Made: {:?}", clusters.len());
    println!("Clustering Time: {:?}", time.elapsed().as_secs_f64());

    clusters
}
