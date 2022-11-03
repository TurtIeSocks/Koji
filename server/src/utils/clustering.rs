use geo::Coordinate;
use rstar::PointDistance;
use std::{
    collections::HashMap,
    // f64::{INFINITY, NEG_INFINITY},
    time::Instant,
};

struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

trait New {
    fn new(point: Coordinate) -> BoundingBox;
    fn update(&self, point: Coordinate) -> BoundingBox;
}

impl New for BoundingBox {
    fn new(point: Coordinate) -> BoundingBox {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        if point.x < min_x {
            min_x = point.x
        }
        if point.x > max_x {
            max_x = point.x
        }
        if point.y < min_y {
            min_y = point.y
        }
        if point.y > max_y {
            max_y = point.y
        }
        BoundingBox {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
    fn update(&self, point: Coordinate) -> BoundingBox {
        let max_x = if point.x > self.max_x {
            point.x
        } else {
            self.max_x
        };
        let min_x = if point.x < self.min_x {
            point.x
        } else {
            self.min_x
        };
        let max_y = if point.y > self.max_y {
            point.y
        } else {
            self.max_y
        };
        let min_y = if point.y < self.min_y {
            point.y
        } else {
            self.min_y
        };
        BoundingBox {
            min_x,
            min_y,
            max_x,
            max_y,
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
    fn from_key(self) -> Coordinate;
}

impl FromKey for String {
    fn from_key(self) -> Coordinate {
        let mut iter = self.split(',');
        let x = iter.next().unwrap().parse::<f64>().unwrap();
        let y = iter.next().unwrap().parse::<f64>().unwrap();
        Coordinate { x, y }
    }
}

pub fn udc(points: Vec<Coordinate>) -> Vec<Coordinate> {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = sqrt2 * 1.5 - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = sqrt2 * 1.5 + 1.;

    let time = Instant::now();
    let mut pre_clusters: HashMap<String, (BoundingBox, bool)> = HashMap::new();
    let mut clusters = Vec::<Coordinate>::new();

    let val = |x: f64| (x / sqrt2).floor();

    for p in points.iter() {
        let vertical_times_sqrt2 = val(p.x);
        let horizontal_times_sqrt2 = val(p.y);
        let new_coord = Coordinate {
            x: val(p.x),
            y: val(p.y),
        };

        let mut pair = pre_clusters.get(&new_coord.to_key());

        if pair.is_some() {
            pre_clusters
                .entry(p.to_key())
                .and_modify(|saved| saved.0 = saved.0.update(*p));
            continue;
        }

        if p.x >= vertical_times_sqrt2 + sqrt2_x_one_point_five_minus_one {
            pair = pre_clusters.get(
                &Coordinate {
                    x: val(p.x) + 1.,
                    y: val(p.y),
                }
                .to_key(),
            );
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (val(p.x) + 1.) + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                pre_clusters
                    .entry(p.to_key())
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.x >= vertical_times_sqrt2 - sqrt2_x_one_point_five_plus_one {
            pair = pre_clusters.get(
                &Coordinate {
                    x: val(p.x) - 1.,
                    y: val(p.y),
                }
                .to_key(),
            );
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (val(p.x) - 1.) + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                pre_clusters
                    .entry(p.to_key())
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.y >= horizontal_times_sqrt2 + sqrt2_x_one_point_five_minus_one {
            pair = pre_clusters.get(
                &Coordinate {
                    x: val(p.x),
                    y: val(p.y) + 1.,
                }
                .to_key(),
            );
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (val(p.y) - 1.) + additive_factor,
                }) <= 1.
            {
                pre_clusters
                    .entry(p.to_key())
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }

        if p.y >= horizontal_times_sqrt2 - sqrt2_x_one_point_five_plus_one {
            pair = pre_clusters.get(
                &Coordinate {
                    x: val(p.x),
                    y: val(p.y) + 1.,
                }
                .to_key(),
            );
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (val(p.y) + 1.) + additive_factor,
                }) <= 1.
            {
                pre_clusters
                    .entry(p.to_key())
                    .and_modify(|saved| saved.0 = saved.0.update(*p));
                continue;
            }
        }
        pre_clusters
            .entry(p.to_key())
            .or_insert((BoundingBox::new(*p), true));
    }

    for (key, value) in pre_clusters.iter() {
        if value.1 {
            clusters.push(key.clone().from_key());
        }
    }

    println!("Clustering Time: {:?}", time.elapsed());
    clusters
}
