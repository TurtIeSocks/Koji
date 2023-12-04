use std::time::Instant;

use crate::{routing, stats::Stats};

use geo::{Contains, Extremes, HaversineDestination, HaversineDistance, Point, Polygon};
use geojson::{Feature, Geometry, Value};
use model::{
    api::{single_vec::SingleVec, sort_by::SortBy, Precision, ToFeature, ToGeometryVec},
    db::sea_orm_active_enums::Type,
};
use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

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

    pub fn sort(&mut self, sort_by: &SortBy, route_split_level: u64) {
        let time = Instant::now();
        self.result = routing::main(
            &vec![],
            self.result.clone(),
            sort_by,
            route_split_level,
            self.radius,
            &mut self.stats,
        );
        self.stats.set_route_time(time);
    }

    pub fn result(self) -> SingleVec {
        self.result
    }

    pub fn feature(self) -> Feature {
        let mut new_feature = self.result.to_feature(Some(Type::CirclePokemon));

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
                Value::MultiPolygon(_) => geometry
                    .to_geometry_vec()
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
            .into_iter()
            .map(|interior| interior.points().collect::<Vec<Point>>())
            .collect();

        let x_mod = 0.75_f64.sqrt();
        let y_mod = 0.568_f64.sqrt();

        let extremes = polygon.extremes().unwrap();
        let max = Point::new(extremes.x_max.coord.x, extremes.y_max.coord.y);
        let min = Point::new(extremes.x_min.coord.x, extremes.y_min.coord.y);

        let start = max.haversine_destination(90., self.radius * 1.5);
        let end = min
            .haversine_destination(270., self.radius * 1.5)
            .haversine_destination(180., self.radius);

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
                        .any(|internal| point_line_distance(&internal, &current) <= self.radius)
                {
                    circles.push(current);
                }
                current = current.haversine_destination(bearing, x_mod * self.radius * 2.)
            }
            current = current.haversine_destination(180., y_mod * self.radius * 2.);

            if row % 2 == 1 {
                bearing = 270.;
            } else {
                bearing = 90.;
            }
            current = current.haversine_destination(bearing, x_mod * self.radius * 3.);

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
        return p.haversine_distance(&a);
    }
    let c2 = dot(&v, &v);
    if c2 <= c1 {
        return p.haversine_distance(&b);
    }
    let b2 = c1 / c2;
    let pb = Point::new(a.x() + b2 * v.x(), a.y() + b2 * v.y());
    p.haversine_distance(&pb)
}

fn point_line_distance(input: &Vec<Point>, point: &Point) -> Precision {
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
