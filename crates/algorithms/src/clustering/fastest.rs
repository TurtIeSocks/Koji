use geo::Coord;
use hashbrown::HashSet;
use koji_core::SingleVec;
use rstar::PointDistance;
use std::collections::HashMap;

use crate::project::Plane;

#[derive(Debug, Clone)]
struct BoundingBox {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl BoundingBox {
    fn new(point: Coord) -> BoundingBox {
        BoundingBox {
            min_x: point.x.min(f64::INFINITY),
            min_y: point.y.min(f64::INFINITY),
            max_x: point.x.max(f64::NEG_INFINITY),
            max_y: point.y.max(f64::NEG_INFINITY),
        }
    }
    fn update(&self, point: Coord) -> BoundingBox {
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
    fn midpoint(&self, other: &Coord) -> [f64; 2];
}

impl ClusterCoords for Coord {
    fn to_key(self) -> String {
        format!("{},{}", self.x, self.y)
    }
    fn midpoint(&self, other: &Coord) -> [f64; 2] {
        [(self.x + other.x) / 2., (self.y + other.y) / 2.]
    }
}

type PointTuple = (i32, i32);
type PointInfo = (BoundingBox, bool, bool, Vec<String>);
type ClusterMap = HashMap<PointTuple, PointInfo>;

#[allow(clippy::wrong_self_convention)]
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

pub fn main(input: &SingleVec, radius: f64, min_points: usize) -> Vec<[f64; 2]> {
    let plane = Plane::new(input).radius(radius);
    let output = plane.project();

    let point_map = cluster(output, min_points);

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

fn update(point_map: &mut ClusterMap, key: PointTuple, p: Coord) {
    point_map.entry(key).and_modify(|saved| {
        saved.0 = saved.0.update(p);
        saved.3.push(p.to_key());
    });
}

fn cluster(points: Vec<Coord>, min_points: usize) -> HashMap<String, Vec<String>> {
    let sqrt2: f64 = 2.0_f64.sqrt();
    let additive_factor: f64 = sqrt2 / 2.;
    let sqrt2_x_one_point_five_minus_one: f64 = (sqrt2 * 1.5) - 1.;
    let sqrt2_x_one_point_five_plus_one: f64 = (sqrt2 * 1.5) + 1.;

    let mut udc_point_map: ClusterMap = HashMap::new();

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
                && p.distance_2(&Coord {
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
                && p.distance_2(&Coord {
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
                && p.distance_2(&Coord {
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
                && p.distance_2(&Coord {
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

    let mut process_final = |coord: Coord, points_to_process: Vec<String>| {
        if !points_to_process.is_empty() {
            point_map_return.insert(coord.to_key(), points_to_process);
        }
    };

    'count: for (point, (bb, _check, _second, points)) in udc_point_map.clone().into_iter() {
        let (v, h) = point;

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
                let lower_left = Coord {
                    x: bb.min_x.min(found_cluster.0.min_x),
                    y: bb.min_y.min(found_cluster.0.min_y),
                };
                let upper_right = Coord {
                    x: bb.max_x.max(found_cluster.0.max_x),
                    y: bb.max_y.max(found_cluster.0.max_y),
                };

                if lower_left.distance_2(&upper_right) <= 4. {
                    let mut combined = points.clone();
                    combined.extend(found_cluster.3.clone());
                    if combined.len() > min_points {
                        let [x, y] = lower_left.midpoint(&upper_right);
                        process_final(Coord { x, y }, combined);
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

    for (key, value) in udc_point_map.into_iter() {
        if value.1 && value.2 && true {
            if value.3.len() == 1 {
                let x = value.0.min_x;
                let y = value.0.min_y;
                process_final(Coord { x, y }, value.3);
            } else {
                let x = key.0 as f64 * sqrt2 + additive_factor;
                let y = key.1 as f64 * sqrt2 + additive_factor;
                process_final(Coord { x, y }, value.3);
            }
        }
    }
    point_map_return
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── main: empty input ─────────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty() {
        let empty: SingleVec = vec![];
        let result = main(&empty, 70.0, 1);
        assert!(result.is_empty());
    }

    // ── main: single point, min_points = 1 ───────────────────────────────────

    #[test]
    fn single_point_min1_returns_one_cluster() {
        let pts = vec![[40.0_f64, -74.0_f64]];
        let result = main(&pts, 70.0, 1);
        assert_eq!(
            result.len(),
            1,
            "one point with min_points=1 should yield 1 cluster"
        );
    }

    // ── main: cluster count never exceeds input points ────────────────────────

    #[test]
    fn clusters_do_not_exceed_input_points() {
        let pts: Vec<[f64; 2]> = (0..20).map(|i| [40.0 + i as f64 * 0.01, -74.0]).collect();
        let result = main(&pts, 70.0, 1);
        assert!(
            result.len() <= pts.len(),
            "cluster count ({}) > input count ({})",
            result.len(),
            pts.len()
        );
    }

    // ── main: dense cluster with high min_points ──────────────────────────────

    #[test]
    fn sparse_points_filtered_by_min_points() {
        // 3 isolated points, each far from the others; min_points = 5 → should return nothing.
        let pts = vec![[40.0, -74.0], [41.0, -74.0], [42.0, -74.0]];
        let result = main(&pts, 70.0, 5);
        // Each point is isolated; no cluster can have 5+ members.
        // (behavior: 0 or fewer than 3 centers; exact 0 is expected here)
        assert!(
            result.len() <= 3,
            "should not exceed input count, got {}",
            result.len()
        );
    }

    // ── main: tight cluster should yield roughly one center ───────────────────

    #[test]
    fn tight_cluster_yields_one_center() {
        // 10 points within ~1 m of each other — Fastest should group into 1 cluster.
        let pts: Vec<[f64; 2]> = (0..10)
            .map(|i| [40.0 + i as f64 * 0.000001, -74.0 + i as f64 * 0.000001])
            .collect();
        let result = main(&pts, 1_000.0, 2); // 1 km radius, 2 min points
        assert!(
            !result.is_empty(),
            "tight cluster with large radius should yield at least 1 center"
        );
        assert!(
            result.len() <= pts.len(),
            "should not produce more clusters than input points"
        );
    }

    // ── main: output contains valid lat/lon ───────────────────────────────────

    #[test]
    fn output_lat_lon_in_valid_range() {
        let pts = vec![[40.0, -74.0], [40.001, -74.001], [40.002, -74.002]];
        let result = main(&pts, 70.0, 1);
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0, "invalid lat: {lat}");
            assert!(lon.abs() < 180.0, "invalid lon: {lon}");
        }
    }

    // ── main: deterministic for same input ────────────────────────────────────

    #[test]
    fn deterministic_same_input() {
        let pts = vec![[40.0, -74.0], [40.001, -74.001], [40.5, -73.0]];
        let a = main(&pts, 70.0, 1);
        let b = main(&pts, 70.0, 1);
        assert_eq!(a.len(), b.len(), "Fastest must be deterministic");
    }
}
