use web_time::Instant;

use crate::{routing, stats::Stats};

use geo::{Contains, Destination, Distance, Extremes, Haversine, Point, Polygon};
use geojson::{Feature, Geometry, Value};
use koji_core::{Precision, SingleVec};

use crate::routing::RoutingConfig;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// Split a `MultiPolygon` geometry into one `Geometry` per polygon, geo-native
/// (no `To*` matrix). Matches the matrix `ToGeometryVec for Geometry`'s
/// `MultiPolygon` arm: each polygon becomes its own `Value::Polygon` geometry,
/// carrying the parent's bbox. A non-MultiPolygon geometry passes through as a
/// single-element vec (the matrix's `_ =>` arm). Only the MultiPolygon case is
/// reached here, but the fallthrough keeps the behavior total.
fn split_multipolygon(geometry: Geometry) -> Vec<Geometry> {
    match geometry.value {
        Value::MultiPolygon(polygons) => polygons
            .into_iter()
            .map(|polygon| Geometry {
                bbox: geometry.bbox.clone(),
                value: Value::Polygon(polygon),
                foreign_members: None,
            })
            .collect(),
        _ => vec![geometry],
    }
}

#[derive(Debug)]
pub struct BootstrapRadius<'a> {
    feature: &'a Feature,
    result: SingleVec,
    radius: Precision,
    pub stats: Stats,
}

impl<'a> BootstrapRadius<'a> {
    pub fn new(feature: &'a Feature, radius: Precision) -> Self {
        let mut new_bootstrap = Self {
            feature,
            result: vec![],
            radius,
            stats: Stats::new("BootstrapRadius".to_string(), 0),
        };

        let time = Instant::now();
        new_bootstrap.result = new_bootstrap.run();
        new_bootstrap.stats.set_cluster_time(time);
        new_bootstrap
            .stats
            .cluster_stats(radius, &vec![], &new_bootstrap.result);

        new_bootstrap
    }

    pub fn sort(&mut self, routing: &RoutingConfig) {
        self.result = routing::main(
            &vec![],
            self.result.clone(),
            self.radius,
            routing,
            &mut self.stats,
        );
    }

    pub fn result(self) -> SingleVec {
        self.result
    }

    pub fn feature(self) -> Feature {
        // Koji-native MultiPoint projection (matches the old matrix
        // `SingleVec::to_feature(CirclePokemon)` geometry); no To* matrix.
        let mut new_feature = koji_core::single_vec_to_multipoint_feature(&self.result);

        if let Some(name) = self.feature.property("__name") {
            new_feature.set_property("__name", name.clone());
        }
        if let Some(geofence_id) = self.feature.property("__id") {
            new_feature.set_property("__geofence_id", geofence_id.clone());
        }
        new_feature.set_property("__mode", "CirclePokemon");
        new_feature
    }

    fn run(&self) -> SingleVec {
        self.flatten_circles()
            .into_iter()
            .map(|p| [p.y(), p.x()])
            .collect()
    }

    fn flatten_circles(&self) -> Vec<Point> {
        if let Some(geometry) = self.feature.geometry.clone() {
            match geometry.value {
                Value::MultiPolygon(_) => split_multipolygon(geometry)
                    .par_iter()
                    .flat_map(|geo| self.generate_circles(geo))
                    .collect(),
                _ => self.generate_circles(&geometry),
            }
        } else {
            vec![]
        }
    }

    fn generate_circles(&self, geometry: &Geometry) -> Vec<Point> {
        let mut circles: Vec<Point> = vec![];

        let polygon = Polygon::<Precision>::try_from(geometry).unwrap();
        let external_points = polygon.exterior().points().collect::<Vec<Point>>();
        let internal_points: Vec<_> = polygon
            .interiors()
            .iter()
            .map(|interior| interior.points().collect::<Vec<Point>>())
            .collect();

        let x_mod = 0.75_f64.sqrt();
        let y_mod = 0.568_f64.sqrt();

        let extremes = polygon.extremes().unwrap();
        let max = Point::new(extremes.x_max.coord.x, extremes.y_max.coord.y);
        let min = Point::new(extremes.x_min.coord.x, extremes.y_min.coord.y);

        let start = Haversine.destination(max, 90.0, self.radius * 1.5);
        let end = Haversine.destination(min, 270., self.radius * 1.5);
        let end = Haversine.destination(end, 180., self.radius);

        let mut row = 0;
        let mut bearing = 270.;
        let mut current = max;

        while current.y() > end.y() {
            while (bearing == 270. && current.x() > end.x())
                || (bearing == 90. && current.x() < start.x())
            {
                if polygon.contains(&current)
                    || point_line_distance(&external_points, &current) <= self.radius
                    || internal_points
                        .par_iter()
                        .any(|internal| point_line_distance(internal, &current) <= self.radius)
                {
                    circles.push(current);
                }
                current = Haversine.destination(current, bearing, x_mod * self.radius * 2.)
            }
            current = Haversine.destination(current, 180., y_mod * self.radius * 2.);

            if row % 2 == 1 {
                bearing = 270.;
            } else {
                bearing = 90.;
            }
            current = Haversine.destination(current, bearing, x_mod * self.radius * 3.);

            row += 1;
        }
        circles
    }
}

fn dot(u: &Point, v: &Point) -> Precision {
    u.x() * v.x() + u.y() * v.y()
}

fn distance_to_segment(p: &Point, a: &Point, b: &Point) -> Precision {
    let v = Point::new(b.x() - a.x(), b.y() - a.y());
    let w = Point::new(p.x() - a.x(), p.y() - a.y());
    let c1 = dot(&w, &v);
    if c1 <= 0.0 {
        return Haversine.distance(*p, *a);
    }
    let c2 = dot(&v, &v);
    if c2 <= c1 {
        return Haversine.distance(*p, *b);
    }
    let b2 = c1 / c2;
    let pb = Point::new(a.x() + b2 * v.x(), a.y() + b2 * v.y());
    Haversine.distance(*p, pb)
}

fn point_line_distance(input: &[Point], point: &Point) -> Precision {
    let mut distance = Precision::MAX;
    for (i, line) in input.iter().enumerate() {
        let next = if i == input.len() - 1 {
            input[0]
        } else {
            input[i + 1]
        };
        distance = distance.min(distance_to_segment(point, line, &next));
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, Value};

    /// Build a simple rectangular geojson Feature as a Polygon.
    /// coords: lon,lat (geojson convention).
    fn rect_feature(min_lon: f64, min_lat: f64, max_lon: f64, max_lat: f64) -> Feature {
        let ring = vec![
            vec![min_lon, min_lat],
            vec![max_lon, min_lat],
            vec![max_lon, max_lat],
            vec![min_lon, max_lat],
            vec![min_lon, min_lat], // closed
        ];
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(Value::Polygon(vec![ring]))),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // ── BootstrapRadius: basic coverage ───────────────────────────────────────

    #[test]
    fn small_rect_at_70m_produces_circles() {
        // ~700×700 m rectangle; radius 70 m → should tile with several circles.
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let br = BootstrapRadius::new(&feature, 70.0);
        let pts = br.result();
        assert!(!pts.is_empty(), "expected some circle centers, got 0");
    }

    #[test]
    fn all_output_points_have_valid_lat_lon() {
        let feature = rect_feature(-74.003, 39.997, -73.997, 40.003);
        let br = BootstrapRadius::new(&feature, 70.0);
        for [lat, lon] in br.result() {
            assert!(lat.abs() < 90.0, "bad lat: {lat}");
            assert!(lon.abs() < 180.0, "bad lon: {lon}");
        }
    }

    #[test]
    fn feature_no_geometry_returns_empty() {
        let feature = Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        };
        let br = BootstrapRadius::new(&feature, 70.0);
        assert!(br.result().is_empty());
    }

    // ── split_multipolygon ────────────────────────────────────────────────────

    #[test]
    fn split_multipolygon_polygon_passthrough() {
        let ring = vec![vec![
            vec![0.0_f64, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 0.0],
        ]];
        let geo = Geometry::new(Value::Polygon(ring.clone()));
        let result = split_multipolygon(geo);
        assert_eq!(result.len(), 1);
        // Single polygon passthrough.
        assert!(matches!(result[0].value, Value::Polygon(_)));
    }

    #[test]
    fn split_multipolygon_splits_multi() {
        let ring1 = vec![vec![
            vec![0.0_f64, 0.0],
            vec![1.0, 0.0],
            vec![0.5, 1.0],
            vec![0.0, 0.0],
        ]];
        let ring2 = vec![vec![
            vec![2.0_f64, 0.0],
            vec![3.0, 0.0],
            vec![2.5, 1.0],
            vec![2.0, 0.0],
        ]];
        let geo = Geometry::new(Value::MultiPolygon(vec![ring1, ring2]));
        let result = split_multipolygon(geo);
        assert_eq!(result.len(), 2);
        for g in &result {
            assert!(matches!(g.value, Value::Polygon(_)));
        }
    }

    // ── dot / distance helpers ────────────────────────────────────────────────

    #[test]
    fn dot_of_perpendicular_is_zero() {
        let u = Point::new(1.0, 0.0);
        let v = Point::new(0.0, 1.0);
        assert!((dot(&u, &v)).abs() < 1e-10);
    }

    #[test]
    fn distance_to_segment_at_endpoint() {
        // distance from a to the segment a-b should be ~0.
        let a = Point::new(-74.0, 40.0);
        let b = Point::new(-73.0, 40.0);
        let d = distance_to_segment(&a, &a, &b);
        assert!(d < 1.0, "distance to self endpoint should be ~0, got {d}");
    }
}
