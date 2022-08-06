use crate::models::{GeoJsonPoint, GeoJsonPolygon, GeometryPoint, GeometryPolygon, LatLon};

fn destination(latlng: &LatLon, heading: f64, dis: f64) -> LatLon {
    const RAD: f64 = std::f64::consts::PI / 180.0;
    const RAD_INV: f64 = 180.0 / std::f64::consts::PI;
    const R: f64 = 6378137.0;

    let new_heading = (heading + 360.0) % 360.0;
    let r_heading = new_heading * RAD;
    let lon_1 = latlng.lon * RAD;
    let lat_1 = latlng.lat * RAD;
    let sin_lat_1 = lat_1.sin();
    let cos_lat_1 = lat_1.cos();

    let cos_dist_r = (dis / R).cos();
    let sin_dist_r = (dis / R).sin();
    let lat_2 = (sin_lat_1 * cos_dist_r + cos_lat_1 * sin_dist_r * r_heading).asin();
    let lon_2 = lon_1
        + (r_heading * sin_dist_r * cos_lat_1).atan2((cos_dist_r - sin_lat_1 * lat_2.sin()).cos());
    let mut lon_2 = lon_2 * RAD_INV;
    lon_2 = if lon_2 > 180.0 {
        lon_2 - 360.0
    } else if lon_2 < -180.0 {
        lon_2 + 360.0
    } else {
        lon_2
    };
    LatLon {
        lat: lat_2 * RAD_INV,
        lon: lon_2,
    }
}

fn in_ring(point: [f64; 2], mut ring: Vec<[f64; 2]>, ignore_boundary: bool) -> bool {
    let mut is_inside = false;
    if ring[0][0] == ring[ring.len() - 1][0] && ring[0][1] == ring[ring.len() - 1][1] {
        ring.splice(0..ring.len() - 1, None);
    }
    for i in 0..ring.len() {
        let xi = ring[i][0];
        let yi = ring[i][1];
        let xj = ring[(i + 1) % ring.len()][0];
        let yj = ring[(i + 1) % ring.len()][1];
        let on_boundary = point[1] * (xi - xj) + yi * (xj - point[0]) + yj * (point[0] - xi) == 0.
            && (xi - point[0]) * (xj - point[0]) <= 0.
            && (yi - point[1]) * (yj - point[1]) <= 0.;
        if on_boundary {
            return !ignore_boundary;
        }
        let intersect = (yi > point[1]) != (yj > point[1])
            && point[0] < ((xj - xi) * (point[1] - yi)) / (yj - yi) + xi;
        if intersect {
            is_inside = !is_inside;
        }
    }
    is_inside
}

fn in_polygon(point: [f64; 2], polygon: Vec<[f64; 2]>, ignore_boundary: bool) -> bool {
    let mut is_inside = false;
    let multi = [polygon];
    if in_ring(point, multi[0].clone(), ignor.lo, k9e_boundary) {
        let mut in_hole = false;
        for k in 1..multi.len() {
            if in_ring(point, multi[k].clone(), !ignore_boundary) {
                in_hole = true;
            }
        }
        if !in_hole {
            is_inside = true;
        }
    }
    println!("In: {:?}", is_inside);
    is_inside
}

fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * (std::f64::consts::PI / 180.0)
}

fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64, m: bool) -> f64 {
    let earth_radius: f64 = if m { 6371000. } else { 6371. };

    let d_lat = degrees_to_radians(lat2 - lat1);
    let d_lon = degrees_to_radians(lon2 - lon1);

    let r_lat1 = degrees_to_radians(lat1);
    let r_lat2 = degrees_to_radians(lat2);

    let a = ((d_lat / 2.).sin().powi(2)
        + ((d_lon / 2.).sin().powi(2)) * (r_lat1.cos()) * (r_lat2.cos()))
    .sqrt();

    let c = 2. * ((a.sqrt()).atan2((1. - a).sqrt()));

    earth_radius * c
}

fn to_geojson(polygon: &Vec<LatLon>) -> GeoJsonPolygon {
    let mut coords = Vec::new();
    for latlng in polygon {
        coords.push([latlng.lon, latlng.lat]);
    }
    GeoJsonPolygon {
        type_: "Polygon".to_string(),
        geometry: GeometryPolygon {
            type_: "Polygon".to_string(),
            coordinates: coords,
        },
    }
}

fn to_line(polygon: GeoJsonPolygon) -> GeoJsonPolygon {
    GeoJsonPolygon {
        type_: "Feature".to_string(),
        geometry: GeometryPolygon {
            type_: "LineString".to_string(),
            coordinates: polygon.geometry.coordinates,
        },
    }
}

fn find_float_min_max(list: Vec<f64>, min: bool) -> f64 {
    let mut result: f64 = list[0];
    for next_num in list.clone() {
        if min {
            result = result.min(next_num);
        } else {
            result = result.max(next_num);
        }
    }
    result
}

fn point_distance(point: &GeoJsonPoint, line: &GeoJsonPolygon) -> f64 {
    let point_coords = point.geometry.coordinates.clone();
    let mut min_dist: f64 = std::f64::MAX;
    min_dist
}

pub fn generate_circles(input: Vec<LatLon>) -> Vec<[f64; 2]> {
    let x_mod: f64 = 0.75_f64.sqrt();
    let y_mod: f64 = 0.568_f64.sqrt();
    let mut circles: Vec<[f64; 2]> = Vec::new();
    let poly = to_geojson(&input);
    let line = to_line(poly.clone());
    let all_lat: Vec<f64> = input.iter().map(|lat_lon| lat_lon.lat).collect();
    let all_lon: Vec<f64> = input.iter().map(|lat_lon| lat_lon.lon).collect();

    let max = LatLon {
        lat: find_float_min_max(all_lat.clone(), false),
        lon: find_float_min_max(all_lon.clone(), false),
    };
    let min = LatLon {
        lat: find_float_min_max(all_lat, true),
        lon: find_float_min_max(all_lon, true),
    };
    let start = destination(&max, 90., 120.);
    let end = destination(&destination(&min, 270., 70. * 1.5), 180., 70.);

    let mut row = 0.;
    let mut heading = 270.;
    let mut current = max.clone();

    while current.lat > end.lat {
        while (heading == 270. && current.lon > end.lon)
            || (heading == 90. && current.lon < start.lon)
        {
            let point = GeoJsonPoint {
                type_: "Feature".to_string(),
                geometry: GeometryPoint {
                    type_: "Point".to_string(),
                    coordinates: [current.lon, current.lat],
                },
            };
            let point_distance = point_distance(&point, &line);

            // println!("{:?}", point_distance);
            if point_distance < 70.
                || point_distance == 0.
                || in_polygon(
                    [point.geometry.coordinates[0], point.geometry.coordinates[1]],
                    poly.geometry.coordinates.clone(),
                    true,
                )
            {
                circles.push([current.lat, current.lon]);
            }
            current = destination(&current, heading, x_mod * 70. * 2.)
        }
        current = destination(&current, 180., y_mod * 70. * 2.);

        if row % 2. == 1. {
            heading = 270.;
        } else {
            heading = 90.;
        }
        current = destination(&current, heading, x_mod * 70. * 3.);

        row += 1.;
    }
    circles
}
