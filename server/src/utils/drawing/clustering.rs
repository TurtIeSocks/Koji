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

impl BoundingBox {
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

trait ClusterCoords {
    fn to_key(self) -> String;
    fn midpoint(&self, other: &Coordinate) -> [f64; 2];
}

impl ClusterCoords for Coordinate {
    fn to_key(self) -> String {
        format!("{},{}", self.x, self.y)
    }
    fn midpoint(&self, other: &Coordinate) -> [f64; 2] {
        [(self.x + other.x) / 2., (self.y + other.y) / 2.]
    }
}

type PointTuple = (i32, i32);
type PointInfo = (BoundingBox, bool, bool, Vec<String>);
type ClusterMap = HashMap<PointTuple, PointInfo>;

// fn try_to_merge(
//     point: (&PointTuple, &PointInfo),
//     v: i32,
//     h: i32,
//     clusters: &mut ClusterReturn,
//     point_map_2: &mut ClusterMap,
//     _index: &str,
//     _min: i32,
//     (biggest, big_coord): (&mut i16, &mut [f64; 2]),
// ) -> bool {
//     let found_cluster = point_map_2.get(&(v, h));
//     if found_cluster.is_none() {
//         return false;
//     }
//     let found_cluster = found_cluster.unwrap();

//     if found_cluster.1 {
//         let lower_left = Coordinate {
//             x: point.1 .0.min_x.min(found_cluster.0.min_x),
//             y: point.1 .0.min_y.min(found_cluster.0.min_y),
//         };
//         let upper_right = Coordinate {
//             x: point.1 .0.max_x.max(found_cluster.0.max_x),
//             y: point.1 .0.max_y.max(found_cluster.0.max_y),
//         };

//         if lower_left.distance_2(&upper_right) <= 4. {
//             let mut new_count = found_cluster.2.len() as i16 + point.1 .2.len() as i16;
//             let mut new_coord = lower_left.midpoint(&upper_right);
//             if new_count > *biggest {
//                 biggest = *new_count;
//                 big_coord = &mut new_coord;
//             }
//             clusters.push(new_coord);
//             point_map_2
//                 .entry((v, h))
//                 .and_modify(|saved| saved.1 = false);
//             point_map_2
//                 .entry(*point.0)
//                 .and_modify(|saved| saved.1 = false);
//             return true;
//         }
//     }
//     false
// }

fn update(point_map: &mut ClusterMap, key: PointTuple, p: Coordinate) {
    point_map.entry(key).and_modify(|saved| {
        saved.0 = saved.0.update(p);
        saved.3.push(p.to_key());
    });
}

pub fn udc(points: Vec<Coordinate>, min_points: usize) -> HashMap<String, Vec<String>> {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = (sqrt2 * 1.5) - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = (sqrt2 * 1.5) + 1.;

    let time = Instant::now();
    let mut udc_point_map: ClusterMap = HashMap::new();
    // let mut seen_map: HashMap<String, bool> = HashMap::new();
    // let mut clusters = ClusterReturn::new();

    for p in points.into_iter() {
        let v = (p.x / sqrt2).floor() as i32;
        let h = (p.y / sqrt2).floor() as i32;
        let vertical_times_sqrt2 = v as f64 * sqrt2;
        let horizontal_times_sqrt2 = h as f64 * sqrt2;
        let key = (v, h);

        let mut pair = udc_point_map.get(&key);

        if pair.is_some() {
            update(&mut udc_point_map, key, p);
            continue;
        }

        if p.x >= (vertical_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = udc_point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v + 1) as f64 + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                update(&mut udc_point_map, key, p);
                continue;
            }
        }

        if p.x <= (vertical_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = udc_point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: sqrt2 * (v - 1) as f64 + additive_factor,
                    y: horizontal_times_sqrt2 + additive_factor,
                }) <= 1.
            {
                update(&mut udc_point_map, key, p);
                continue;
            }
        }

        if p.y <= (horizontal_times_sqrt2 + sqrt2_x_one_point_five_minus_one) {
            pair = udc_point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h - 1) as f64 + additive_factor,
                }) <= 1.
            {
                update(&mut udc_point_map, key, p);
                continue;
            }
        }

        if p.y >= (horizontal_times_sqrt2 - sqrt2_x_one_point_five_plus_one) {
            pair = udc_point_map.get(&key);
            if pair.is_some()
                && p.distance_2(&Coordinate {
                    x: vertical_times_sqrt2 + additive_factor,
                    y: sqrt2 * (h + 1) as f64 + additive_factor,
                }) <= 1.
            {
                update(&mut udc_point_map, key, p);
                continue;
            }
        }
        udc_point_map
            .entry(key)
            .or_insert((BoundingBox::new(p), true, true, vec![p.to_key()]));
    }
    let mut point_map_return: HashMap<String, Vec<String>> = HashMap::new();

    let mut process_final = |coord: Coordinate, points_to_process: Vec<String>| {
        if points_to_process.len() > 0 {
            // for point in points_to_process.clone().into_iter() {
            //     seen_map.insert(point, true);
            // }
            point_map_return.insert(coord.to_key(), points_to_process);
        }
    };

    'count: for (point, (bb, _check, _second, points)) in udc_point_map.clone().into_iter() {
        let (v, h) = point.clone();

        for (v, h, _index) in [
            (v, h - 1, "s"),
            (v, h + 1, "n"),
            (v + 1, h, "e"),
            (v - 1, h, "w"),
            (v - 1, h - 1, "sw"),
            (v + 1, h - 1, "se"),
            (v + 1, h + 1, "ne"),
            (v - 1, h + 1, "nw"),
        ]
        .into_iter()
        {
            let found_cluster = udc_point_map.get(&(v, h));
            if found_cluster.is_none() {
                continue;
            }
            let found_cluster = found_cluster.unwrap();

            if found_cluster.1 {
                let lower_left = Coordinate {
                    x: bb.min_x.min(found_cluster.0.min_x),
                    y: bb.min_y.min(found_cluster.0.min_y),
                };
                let upper_right = Coordinate {
                    x: bb.max_x.max(found_cluster.0.max_x),
                    y: bb.max_y.max(found_cluster.0.max_y),
                };

                if lower_left.distance_2(&upper_right) <= 4. {
                    let mut combined = points.clone();
                    combined.extend(found_cluster.3.clone());
                    if combined.len() > min_points {
                        let [x, y] = lower_left.midpoint(&upper_right);
                        process_final(Coordinate { x, y }, combined);
                        udc_point_map
                            .entry((v, h))
                            .and_modify(|saved| saved.1 = false);
                        udc_point_map
                            .entry(point)
                            .and_modify(|saved| saved.1 = false);
                        continue 'count;
                    }
                }
            }
        }
    }

    'count: for (point, (bb, _check, _second, points)) in udc_point_map.clone().into_iter() {
        let (v, h) = point.clone();

        for (v, h, _index) in [
            (v, h - 1, "s"),
            (v, h + 1, "n"),
            (v + 1, h, "e"),
            (v - 1, h, "w"),
            (v - 1, h - 1, "sw"),
            (v + 1, h - 1, "se"),
            (v + 1, h + 1, "ne"),
            (v - 1, h + 1, "nw"),
        ]
        .into_iter()
        {
            let found_cluster = udc_point_map.get(&(v, h));
            if found_cluster.is_none() {
                continue;
            }
            let found_cluster = found_cluster.unwrap();

            if found_cluster.2 {
                let lower_left = Coordinate {
                    x: bb.min_x.min(found_cluster.0.min_x),
                    y: bb.min_y.min(found_cluster.0.min_y),
                };
                let upper_right = Coordinate {
                    x: bb.max_x.max(found_cluster.0.max_x),
                    y: bb.max_y.max(found_cluster.0.max_y),
                };

                if lower_left.distance_2(&upper_right) <= 4. {
                    let mut combined = points.clone();
                    combined.extend(found_cluster.3.clone());
                    if combined.len() > min_points {
                        let [x, y] = lower_left.midpoint(&upper_right);
                        process_final(Coordinate { x, y }, combined);
                        udc_point_map
                            .entry((v, h))
                            .and_modify(|saved| saved.2 = false);
                        udc_point_map
                            .entry(point)
                            .and_modify(|saved| saved.2 = false);
                        continue 'count;
                    }
                }
            }
        }
    }

    for (key, value) in udc_point_map.into_iter() {
        if value.1 && value.2 {
            if true {
                if value.3.len() == 1 {
                    let x = value.0.min_x;
                    let y = value.0.min_y;
                    process_final(Coordinate { x, y }, value.3);
                } else {
                    let x = key.0 as f64 * sqrt2 + additive_factor;
                    let y = key.1 as f64 * sqrt2 + additive_factor;
                    process_final(Coordinate { x, y }, value.3);
                }
            }
        }
    }
    println!("Clustering Time: {:?}", time.elapsed().as_secs_f64());
    point_map_return
}
