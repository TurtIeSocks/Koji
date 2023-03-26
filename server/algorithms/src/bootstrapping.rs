use super::*;

use geo::{Contains, Extremes, HaversineDestination, HaversineDistance, Point, Polygon};
use geojson::{Feature, Geometry, Value};
use model::api::{single_vec::SingleVec, stats::Stats, GetBbox, ToFeatureVec, ToSingleVec};

fn dot(u: &Point, v: &Point) -> f64 {
    u.x() * v.x() + u.y() * v.y()
}

fn distance_to_segment(p: &Point, a: &Point, b: &Point) -> f64 {
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

pub fn point_line_distance(input: &Vec<Point>, point: &Point) -> f64 {
    let mut distance: f64 = std::f64::MAX;
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

fn flatten_circles(feature: Feature, radius: f64, stats: &mut Stats) -> Vec<Point> {
    if feature.geometry.is_none() {
        return vec![];
    }
    let geometry = feature.geometry.unwrap();
    let circles = match geometry.value {
        Value::MultiPolygon(_) => geometry
            .to_feature_vec()
            .into_iter()
            .flat_map(|feat| {
                if let Some(geo) = feat.geometry {
                    generate_circles(geo, radius)
                } else {
                    vec![]
                }
            })
            .collect(),
        _ => generate_circles(geometry, radius),
    };
    stats.total_clusters += circles.len();
    circles
}

pub fn as_vec(feature: Feature, radius: f64, stats: &mut Stats) -> SingleVec {
    flatten_circles(feature, radius, stats)
        .iter()
        .map(|p| [p.y(), p.x()])
        .collect()
}

pub fn as_geojson(feature: Feature, radius: f64, stats: &mut Stats) -> Feature {
    // let mut multiline_feature: Vec<Vec<Vec<f64>>> = vec![];
    let mut multipoint_feature: Vec<Vec<f64>> = vec![];
    let circles = flatten_circles(feature.clone(), radius, stats);

    for (i, point) in circles.iter().enumerate() {
        multipoint_feature.push(vec![point.x(), point.y()]);
        let point2 = if i == circles.len() {
            circles[i + 1]
        } else {
            circles[0]
        };
        let distance = point.haversine_distance(&point2);
        if distance > stats.longest_distance {
            stats.longest_distance = distance;
        }
        stats.total_distance += distance;
    }
    let geo_collection = Geometry {
        value: Value::MultiPoint(multipoint_feature),
        bbox: None,
        foreign_members: None,
    };
    let mut new_feature = Feature {
        bbox: None,
        geometry: Some(geo_collection),
        ..Feature::default()
    };
    if let Some(name) = feature.property("__name") {
        if let Some(name) = name.as_str() {
            new_feature.set_property("__name", name);
        }
    }
    if let Some(geofence_id) = feature.property("__id") {
        if let Some(geofence_id) = geofence_id.as_str() {
            new_feature.set_property("__geofence_id", geofence_id);
        }
    }
    new_feature.set_property("__mode", "CirclePokemon");
    new_feature.bbox = feature.to_single_vec().get_bbox();
    new_feature
}

fn generate_circles(geometry: Geometry, radius: f64) -> Vec<Point> {
    let mut circles: Vec<Point> = vec![];

    let polygon = Polygon::<f64>::try_from(geometry).unwrap();
    let external_points = polygon.exterior().points().collect::<Vec<Point>>();
    let internal_points = polygon
        .interiors()
        .iter()
        .map(|interior| interior.points().collect::<Vec<Point>>());

    let x_mod = 0.75_f64.sqrt();
    let y_mod = 0.568_f64.sqrt();

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
            if polygon.contains(&current)
                || point_line_distance(&external_points, &current) <= radius
                || internal_points
                    .clone()
                    .any(|internal| point_line_distance(&internal, &current) <= radius)
            {
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
    circles
}
