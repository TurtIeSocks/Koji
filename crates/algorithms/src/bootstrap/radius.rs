use web_time::Instant;

use crate::{routing, stats::Stats};

use geo::{Contains, Destination, Distance, Extremes, Haversine, Point, Polygon};
use geojson::{Feature, Geometry, GeometryValue};
use koji_core::{Precision, SingleVec};

use crate::routing::RoutingConfig;
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

/// Split a `MultiPolygon` geometry into one `Geometry` per polygon, geo-native
/// (no `To*` matrix). Matches the matrix `ToGeometryVec for Geometry`'s
/// `MultiPolygon` arm: each polygon becomes its own `GeometryValue::Polygon` geometry,
/// carrying the parent's bbox. A non-MultiPolygon geometry passes through as a
/// single-element vec (the matrix's `_ =>` arm). Only the MultiPolygon case is
/// reached here, but the fallthrough keeps the behavior total.
fn split_multipolygon(geometry: Geometry) -> Vec<Geometry> {
    match geometry.value {
        GeometryValue::MultiPolygon {
            coordinates: polygons,
        } => polygons
            .into_iter()
            .map(|polygon| Geometry {
                bbox: geometry.bbox.clone(),
                value: GeometryValue::Polygon {
                    coordinates: polygon,
                },
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
                GeometryValue::MultiPolygon { .. } => split_multipolygon(geometry)
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
        let mut rows: Vec<Vec<Point>> = vec![];

        let polygon = Polygon::<Precision>::try_from(geometry).unwrap();
        let external_points = polygon.exterior().points().collect::<Vec<Point>>();
        let internal_points: Vec<_> = polygon
            .interiors()
            .iter()
            .map(|interior| interior.points().collect::<Vec<Point>>())
            .collect();

        let x_mod = 0.75_f64.sqrt();
        // 0.5625 = 0.75²: row pitch = 2·√0.5625·r = 1.5r exactly, the pointy-top
        // hex covering pitch. (Was 0.568 → 1.5073r, over-spacing rows ~0.5% and
        // leaving thin uncovered slivers between every trio of circles.)
        let y_mod = 0.5625_f64.sqrt();

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
            let mut current_row: Vec<Point> = vec![];
            while (bearing == 270. && current.x() > end.x())
                || (bearing == 90. && current.x() < start.x())
            {
                if polygon.contains(&current)
                    || point_line_distance(&external_points, &current) <= self.radius
                    || internal_points
                        .par_iter()
                        .any(|internal| point_line_distance(internal, &current) <= self.radius)
                {
                    current_row.push(current);
                }
                current = Haversine.destination(current, bearing, x_mod * self.radius * 2.)
            }
            if !current_row.is_empty() {
                rows.push(current_row);
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

        comb_order(rows)
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

/// Squared planar distance in lon/lat degrees. Only used for *local* nearest-end
/// comparisons inside `comb_order`, where monotonicity with true distance is all
/// that matters (no geodesic needed at row scale).
fn sq_dist(a: Point, b: Point) -> Precision {
    let dx = a.x() - b.x();
    let dy = a.y() - b.y();
    dx * dx + dy * dy
}

/// Reorder `rows` (emitted top→bottom, each the kept centers of one lattice row)
/// into a comb / boustrophedon-loop visiting order: down the even-indexed rows,
/// back up the odd-indexed rows, orienting each row to enter at the end nearest
/// the previous row's exit. The tour ends in the row adjacent to where it began,
/// so the closing leg (last→first, the device's loop-back) is ~one row pitch
/// instead of the full polygon height. Pure permutation of the input points.
fn comb_order(rows: Vec<Vec<Point>>) -> Vec<Point> {
    let rows: Vec<Vec<Point>> = rows.into_iter().filter(|r| !r.is_empty()).collect();
    let n = rows.len();
    if n <= 1 {
        return rows.into_iter().flatten().collect();
    }

    // Visiting order of row indices: evens ascending, then odds descending.
    // e.g. 6 rows -> [0,2,4, 5,3,1]; 5 rows -> [0,2,4, 3,1].
    let mut order: Vec<usize> = (0..n).step_by(2).collect();
    order.extend((0..n).filter(|i| i % 2 == 1).rev());

    // Emit greedily: orient each row so its entry end is nearest the previous
    // row's exit. Try both orientations of the first row and keep whichever
    // yields the shorter closing leg (last→first).
    let emit = |reverse_first: bool| -> Vec<Point> {
        let mut out: Vec<Point> = Vec::with_capacity(rows.iter().map(|r| r.len()).sum());
        for &idx in &order {
            let mut row = rows[idx].clone();
            match out.last().copied() {
                None if reverse_first => row.reverse(),
                None => {}
                Some(prev) => {
                    let back = *row.last().unwrap();
                    let front = row[0];
                    if sq_dist(prev, back) < sq_dist(prev, front) {
                        row.reverse();
                    }
                }
            }
            out.extend(row);
        }
        out
    };

    let a = emit(false);
    let b = emit(true);
    match (a.first(), a.last(), b.first(), b.last()) {
        (Some(&a0), Some(&an), Some(&b0), Some(&bn)) if sq_dist(bn, b0) < sq_dist(an, a0) => b,
        _ => a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, Geometry, GeometryValue};

    /// Build a simple rectangular geojson Feature as a Polygon.
    /// coords: lon,lat (geojson convention).
    fn rect_feature(
        min_lon: Precision,
        min_lat: Precision,
        max_lon: Precision,
        max_lat: Precision,
    ) -> Feature {
        let ring = vec![
            geojson::Position::from([min_lon, min_lat]),
            geojson::Position::from([max_lon, min_lat]),
            geojson::Position::from([max_lon, max_lat]),
            geojson::Position::from([min_lon, max_lat]),
            geojson::Position::from([min_lon, min_lat]), // closed
        ];
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(GeometryValue::Polygon {
                coordinates: vec![ring],
            })),
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
            geojson::Position::from([0.0_f64, 0.0]),
            geojson::Position::from([1.0, 0.0]),
            geojson::Position::from([1.0, 1.0]),
            geojson::Position::from([0.0, 0.0]),
        ]];
        let geo = Geometry::new(GeometryValue::Polygon {
            coordinates: ring.clone(),
        });
        let result = split_multipolygon(geo);
        assert_eq!(result.len(), 1);
        // Single polygon passthrough.
        assert!(matches!(result[0].value, GeometryValue::Polygon { .. }));
    }

    #[test]
    fn split_multipolygon_splits_multi() {
        let ring1 = vec![vec![
            geojson::Position::from([0.0_f64, 0.0]),
            geojson::Position::from([1.0, 0.0]),
            geojson::Position::from([0.5, 1.0]),
            geojson::Position::from([0.0, 0.0]),
        ]];
        let ring2 = vec![vec![
            geojson::Position::from([2.0_f64, 0.0]),
            geojson::Position::from([3.0, 0.0]),
            geojson::Position::from([2.5, 1.0]),
            geojson::Position::from([2.0, 0.0]),
        ]];
        let geo = Geometry::new(GeometryValue::MultiPolygon {
            coordinates: vec![ring1, ring2],
        });
        let result = split_multipolygon(geo);
        assert_eq!(result.len(), 2);
        for g in &result {
            assert!(matches!(g.value, GeometryValue::Polygon { .. }));
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

    // ── comb_order ────────────────────────────────────────────────────────────

    /// row r sits at latitude (n_rows - r) so row 0 is the top (highest lat);
    /// columns 0..n_cols at increasing longitude, stored left→right.
    fn build_grid_rows(n_rows: usize, n_cols: usize) -> Vec<Vec<Point>> {
        (0..n_rows)
            .map(|r| {
                (0..n_cols)
                    .map(|c| Point::new(c as Precision, (n_rows - r) as Precision))
                    .collect()
            })
            .collect()
    }

    /// Multiset key for permutation comparison (coords are small, scale to int).
    fn sorted_xy(pts: &[Point]) -> Vec<(i64, i64)> {
        let mut v: Vec<(i64, i64)> = pts
            .iter()
            .map(|p| {
                (
                    (p.x() * 1000.0).round() as i64,
                    (p.y() * 1000.0).round() as i64,
                )
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn comb_order_is_permutation() {
        let rows = build_grid_rows(4, 3);
        let flat: Vec<Point> = rows.iter().flatten().copied().collect();
        let out = comb_order(rows);
        assert_eq!(out.len(), 12, "comb_order must not add or drop points");
        assert_eq!(sorted_xy(&out), sorted_xy(&flat), "same multiset as input");
    }

    #[test]
    fn comb_order_empty_is_empty() {
        assert!(comb_order(vec![]).is_empty());
    }

    #[test]
    fn comb_order_single_row_unchanged() {
        let rows = build_grid_rows(1, 4);
        let out = comb_order(rows.clone());
        assert_eq!(out, rows[0], "single row passes through in order");
    }

    #[test]
    fn comb_order_starts_top_ends_in_adjacent_row() {
        // 5 rows: top lat = 5, second-from-top lat = 4.
        let out = comb_order(build_grid_rows(5, 3));
        assert_eq!(out.first().unwrap().y(), 5.0, "tour starts in the top row");
        assert_eq!(
            out.last().unwrap().y(),
            4.0,
            "comb returns adjacent to the start (row 1), not the far bottom row"
        );
    }

    #[test]
    fn comb_order_filters_empty_rows() {
        let mut rows = build_grid_rows(3, 2); // lats 3, 2, 1
        rows.insert(1, vec![]); // empty row must not shift even/odd parity
        let out = comb_order(rows);
        assert_eq!(out.len(), 6, "empty rows contribute nothing");
        for p in &out {
            assert!(
                [1.0, 2.0, 3.0].contains(&p.y()),
                "no phantom points: {:?}",
                p
            );
        }
    }

    #[test]
    fn comb_closing_leg_is_short_on_tall_polygon() {
        // Tall, skinny rectangle: ~2 km of latitude, ~150 m of longitude.
        // The snake order loops back across the full ~2 km height; the comb ends
        // one row from the start, so the closing leg is a few row pitches.
        let feature = rect_feature(-74.0009, 39.990, -73.9991, 40.010);
        let radius = 70.0;
        let pts = BootstrapRadius::new(&feature, radius).result();
        assert!(
            pts.len() > 4,
            "expected a multi-row fill, got {}",
            pts.len()
        );

        let first = Point::new(pts.first().unwrap()[1], pts.first().unwrap()[0]);
        let last = Point::new(pts.last().unwrap()[1], pts.last().unwrap()[0]);
        let closing = Haversine.distance(first, last);
        assert!(
            closing < 4.0 * radius,
            "comb closing leg {closing:.0} m should be a few row pitches, not the ~2 km height"
        );
    }
}
