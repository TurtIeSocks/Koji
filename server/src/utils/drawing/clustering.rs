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

// #[derive(Debug, Clone)]
// struct NeighborMatrix {
//     pub s: bool,
//     pub n: bool,
//     pub e: bool,
//     pub w: bool,
//     pub sw: bool,
//     pub se: bool,
//     pub ne: bool,
//     pub nw: bool,
// }

// impl Index<&'_ str> for NeighborMatrix {
//     type Output = bool;
//     fn index(&self, s: &str) -> &bool {
//         match s {
//             "s" => &self.s,
//             "n" => &self.n,
//             "e" => &self.e,
//             "w" => &self.w,
//             "sw" => &self.sw,
//             "se" => &self.se,
//             "ne" => &self.ne,
//             "nw" => &self.nw,
//             _ => panic!("unknown field: {}", s),
//         }
//     }
// }

// impl IndexMut<&'_ str> for NeighborMatrix {
//     fn index_mut(&mut self, s: &str) -> &mut bool {
//         match s {
//             "s" => &mut self.s,
//             "n" => &mut self.n,
//             "e" => &mut self.e,
//             "w" => &mut self.w,
//             "sw" => &mut self.sw,
//             "se" => &mut self.se,
//             "ne" => &mut self.ne,
//             "nw" => &mut self.nw,
//             _ => panic!("unknown field: {}", s),
//         }
//     }
// }

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
type PointInfo = (BoundingBox, bool, Vec<Coordinate>);
type ClusterMap = HashMap<PointTuple, PointInfo>;
type ClusterReturn = Vec<[f64; 2]>;

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

fn update(point_map: &mut ClusterMap, key: PointTuple, p: &Coordinate) {
    point_map.entry(key).and_modify(|saved| {
        saved.0 = saved.0.update(*p);
        saved.2.push(*p);
    });
}

pub fn udc(points: Vec<Coordinate>, _min: i32) -> (ClusterReturn, [f64; 2]) {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = (sqrt2 * 1.5) - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = (sqrt2 * 1.5) + 1.;

    let time = Instant::now();
    let mut udc_point_map: ClusterMap = HashMap::new();
    // let mut visited_map: HashMap<_, _> =
    //     points.iter().map(|coord| (coord.to_key(), false)).collect();

    let mut clusters = ClusterReturn::new();

    for p in points.iter() {
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
        udc_point_map.entry(key).or_insert((
            BoundingBox::new(*p),
            true,
            // NeighborMatrix {
            //     s: true,
            //     n: true,
            //     e: true,
            //     w: true,
            //     sw: true,
            //     se: true,
            //     ne: true,
            //     nw: true,
            // },
            vec![*p],
        ));
    }
    let mut point_map_2 = udc_point_map.clone();
    let mut biggest: i16 = 0;
    let mut best_coord = [0., 0.];

    'count: for (point, (bb, _check, points)) in udc_point_map.iter() {
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
        .iter()
        {
            let found_cluster = point_map_2.get(&(*v, *h));
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
                    let new_count = found_cluster.2.len() as i16 + points.len() as i16;
                    let new_coord = lower_left.midpoint(&upper_right);
                    if new_count > biggest {
                        biggest = new_count;
                        best_coord = new_coord;
                    }
                    clusters.push(new_coord);
                    point_map_2
                        .entry((*v, *h))
                        .and_modify(|saved| saved.1 = false);
                    point_map_2
                        .entry(*point)
                        .and_modify(|saved| saved.1 = false);
                    continue 'count;
                }
            }
        }
    }
    for (key, value) in point_map_2.iter() {
        if value.1 {
            if value.2.len() == 1 {
                clusters.push([value.0.min_x, value.0.min_y]);
            } else {
                let lat = key.0 as f64 * sqrt2 + additive_factor;
                let lon = key.1 as f64 * sqrt2 + additive_factor;
                if value.2.len() as i16 > biggest {
                    best_coord = [lat, lon];
                }
                clusters.push([lat, lon]);
            }
        }
    }
    println!("Clusters Made: {:?}", clusters.len());
    println!("Clustering Time: {:?}", time.elapsed().as_secs_f64());
    (clusters, best_coord)
}

// fn greedy_cluster(point_map: ClusterMap, mut point_map_2: ClusterMap) {
//     'point: for point in point_map.iter() {
//         let (v, h) = point.0.clone();

//         let mut best_pos = [v as f64, h as f64];
//         let mut best_count = point.1 .2;
//         for neighbor in [
//             (v - 1, h - 1),
//             (v - 1, h),
//             (v - 1, h + 1),
//             (v, h - 1),
//             (v, h + 1),
//             (v + 1, h - 1),
//             (v + 1, h),
//             (v + 1, h + 1),
//         ]
//         .iter()
//         {
//             let found_cluster = point_map.get(neighbor);
//             if found_cluster.is_some() {
//                 let found_cluster = found_cluster.unwrap();
//                 if found_cluster.1 {
//                     let lower_left = Coordinate {
//                         x: point.1 .0.min_x.min(found_cluster.0.min_x),
//                         y: point.1 .0.min_y.min(found_cluster.0.min_y),
//                     };
//                     let upper_right = Coordinate {
//                         x: point.1 .0.max_x.max(found_cluster.0.max_x),
//                         y: point.1 .0.max_y.max(found_cluster.0.max_y),
//                     };
//                     if lower_left.distance_2(&upper_right) <= 4. {
//                         let new_total = found_cluster.2 + point.1 .2;
//                         if new_total > best_count {
//                             best_pos = lower_left.midpoint(&upper_right);
//                             best_count = new_total;
//                         }
//                         continue 'point;
//                     } else if found_cluster.2 > best_count {
//                         best_pos = [neighbor.0 as f64, neighbor.1 as f64];
//                         best_count = found_cluster.2;
//                     }
//                 }
//             }
//         }

//         println!("{:?}, {}", best_pos, best_count);
//     }
// }
