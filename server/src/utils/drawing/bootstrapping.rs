use crate::utils::convert::feature::split_multi;

use super::*;
use geo::{Contains, Extremes, HaversineDestination, HaversineDistance, Point, Polygon};
use geojson::Value;

fn dot(u: &Point, v: &Point) -> f64 {
    u.x() * v.x() + u.y() * v.y()
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
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
    return p.haversine_distance(&pb);
}

pub fn point_line_distance(input: Vec<Point>, point: Point) -> f64 {
    let mut distance: f64 = std::f64::MAX;
    for (i, line) in input.iter().enumerate() {
        let next = if i == input.len() - 1 {
            input[0]
        } else {
            input[i + 1]
        };
        distance = distance.min(distance_to_segment(point, *line, next));
    }
    distance
}

pub fn check(input: Feature, radius: f64) -> Vec<[f64; 2]> {
    match input.geometry.clone().unwrap().value {
        Value::MultiPolygon(_) => split_multi(input)
            .into_iter()
            .flat_map(|feat| generate_circles(feat, radius))
            .collect(),
        _ => generate_circles(input, radius),
    }
}

fn generate_circles(input: Feature, radius: f64) -> Vec<[f64; 2]> {
    let mut circles: Vec<Point> = Vec::new();

    if input.geometry.is_none() {
        return circles.iter().map(|p| [p.y(), p.x()]).collect();
    }
    let polygon = Polygon::<f64>::try_from(input).unwrap();
    let get_points = || polygon.exterior().points().collect::<Vec<Point>>();

    let x_mod: f64 = 0.75_f64.sqrt();
    let y_mod: f64 = 0.568_f64.sqrt();

    let extremes = polygon.extremes().unwrap();
    let max = Point::new(extremes.x_max.coord.x, extremes.y_max.coord.y);
    let min = Point::new(extremes.x_min.coord.x, extremes.y_min.coord.y);

    let start = max.haversine_destination(90., radius * 1.5);
    let end = min
        .haversine_destination(270., radius * 1.5)
        .haversine_destination(180., radius);

    let mut row = 0;
    let mut bearing = 270.;
    let mut current = max;

    while current.y() > end.y() {
        while (bearing == 270. && current.x() > end.x())
            || (bearing == 90. && current.x() < start.x())
        {
            let point_distance = point_line_distance(get_points(), current);
            if point_distance <= radius || point_distance == 0. || polygon.contains(&current) {
                circles.push(current);
            }
            current = current.haversine_destination(bearing, x_mod * radius * 2.)
        }
        current = current.haversine_destination(180., y_mod * radius * 2.);

        if row % 2 == 1 {
            bearing = 270.;
        } else {
            bearing = 90.;
        }
        current = current.haversine_destination(bearing, x_mod * radius * 3.);

        row += 1;
    }
    circles.iter().map(|p| [p.y(), p.x()]).collect()
}
