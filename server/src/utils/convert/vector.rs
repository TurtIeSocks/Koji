use super::*;

use geojson::{Geometry, Value};
use num_traits::Float;
use std::str::FromStr;

use crate::models::{scanner::GenericData, MultiVec, SingleStruct, SingleVec};

pub fn from_struct<T>(coords: SingleStruct<T>) -> SingleVec<T>
where
    T: Float,
{
    coords.iter().map(|p| [p.lat, p.lon]).collect()
}

pub fn from_generic_data<T>(coords: Vec<GenericData<T>>) -> SingleVec<T>
where
    T: Float,
{
    coords.iter().map(|p| p.p).collect()
}

pub fn from_text<T>(area_data: &str) -> SingleVec<T>
where
    T: FromStr + Float,
{
    let mut points: SingleVec<T> = vec![];
    let test = text_test(area_data);
    let coords: Vec<&str> = area_data.split(if test { "," } else { "\n" }).collect();
    for coord in coords {
        let lat_lon: Vec<&str> = if test {
            coord.split_whitespace().collect()
        } else {
            coord.split(",").collect()
        };
        if lat_lon.is_empty() || lat_lon.concat().is_empty() {
            continue;
        }
        let lat = lat_lon[0].trim().parse::<T>();
        let lat = match lat {
            Ok(lat) => lat,
            Err(_) => continue,
        };
        let lon = lat_lon[1].trim().parse::<T>();
        let lon: T = match lon {
            Ok(lon) => lon,
            Err(_) => continue,
        };
        points.push([lat, lon]);
    }
    ensure_first_last(points)
}

pub fn from_collection(fc: FeatureCollection) -> MultiVec {
    let mut return_value: MultiVec = vec![];

    for feature in fc.features.into_iter() {
        if let Some(geometry) = feature.geometry {
            match geometry.value {
                Value::MultiPolygon(_) => feature::split_multi(geometry)
                    .into_iter()
                    .for_each(|f| return_value.push(from_geometry(f.geometry.unwrap()))),
                Value::GeometryCollection(geometries) => geometries.into_iter().for_each(|g| {
                    let value = from_geometry(g);
                    if !value.is_empty() {
                        return_value.push(value)
                    }
                }),
                _ => return_value.push(from_geometry(geometry)),
            }
        }
    }
    return_value
}

pub fn from_geometry(geometry: Geometry) -> SingleVec {
    let mut temp_arr: SingleVec = vec![];
    match geometry.value {
        // This should be unused now but leaving it since the work has been done
        Value::MultiPolygon(geometry) => {
            for poly in geometry.into_iter() {
                for line in poly.into_iter() {
                    for point in line.into_iter() {
                        if point.len() == 2 {
                            temp_arr.push([point[1], point[0]]);
                        }
                    }
                }
            }
        }
        Value::Polygon(geometry) => {
            for line in geometry.into_iter() {
                for point in line.into_iter() {
                    if point.len() == 2 {
                        temp_arr.push([point[1], point[0]]);
                    }
                }
            }
        }
        Value::MultiPoint(geometry) => {
            for point in geometry.into_iter() {
                if point.len() == 2 {
                    temp_arr.push([point[1], point[0]]);
                }
            }
        }
        Value::Point(geometry) => {
            if geometry.len() == 2 {
                temp_arr.push([geometry[1], geometry[0]]);
            }
        }
        _ => {
            println!("Unsupported Geometry: {:?}", geometry.value.type_name())
        }
    }
    temp_arr
}
