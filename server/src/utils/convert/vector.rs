use super::*;
use crate::models::scanner::{GenericData, LatLon};
// use geojson::Value;
use num_traits::Float;
use std::str::FromStr;

pub fn from_struct<T>(coords: Vec<LatLon<T>>) -> Vec<[T; 2]>
where
    T: Float,
{
    coords.iter().map(|p| [p.lat, p.lon]).collect()
}

pub fn from_generic_data<T>(coords: Vec<GenericData<T>>) -> Vec<[T; 2]>
where
    T: Float,
{
    coords.iter().map(|p| p.p).collect()
}

pub fn from_text<T>(area_data: &str, rdm_text: bool) -> Vec<[T; 2]>
where
    T: FromStr + Float,
{
    let mut points: Vec<[T; 2]> = Vec::new();
    let coords: Vec<&str> = area_data.split(if rdm_text { "\n" } else { "," }).collect();
    for coord in coords {
        let lat_lon: Vec<&str> = if rdm_text {
            coord.split(",").collect()
        } else {
            coord.split_whitespace().collect()
        };
        if lat_lon.is_empty() {
            continue;
        }
        let lat = lat_lon[0].parse::<T>();
        let lat = match lat {
            Ok(lat) => lat,
            Err(_) => {
                println!("Error parsing lat: {}", lat_lon[0]);
                continue;
            }
        };
        let lon = lat_lon[1].parse::<T>();
        let lon: T = match lon {
            Ok(lon) => lon,
            Err(_) => {
                println!("Error parsing lon: {}", lat_lon[1]);
                continue;
            }
        };
        points.push([lat, lon]);
    }
    ensure_first_last(points)
}

// pub fn from_collection(fc: FeatureCollection) -> Vec<Vec<[f64; 2]>> {
//     let mut return_value = Vec::<Vec<[f64; 2]>>::new();

//     for feature in fc.features.into_iter() {
//         if feature.geometry.is_some() {
//             return_value.push(from_feature(feature));
//         }
//     }
//     return_value
// }

// pub fn from_feature(feature: Feature) -> Vec<[f64; 2]> {
//     let mut temp_arr = Vec::<[f64; 2]>::new();
//     match feature.geometry.unwrap().value {
//         Value::MultiPolygon(geometry) => {
//             for poly in geometry.into_iter() {
//                 for point in poly.into_iter() {
//                     for p in point.into_iter() {
//                         if p.len() == 2 {
//                             temp_arr.push([p[1], p[0]]);
//                         }
//                     }
//                 }
//             }
//         }
//         Value::Polygon(geometry) => {
//             for poly in geometry.into_iter() {
//                 for point in poly.into_iter() {
//                     if point.len() == 2 {
//                         temp_arr.push([point[1], point[0]]);
//                     }
//                 }
//             }
//         }
//         Value::MultiPoint(geometry) => {
//             for point in geometry.into_iter() {
//                 if point.len() == 2 {
//                     temp_arr.push([point[1], point[0]]);
//                 }
//             }
//         }
//         Value::Point(geometry) => {
//             if geometry.len() == 2 {
//                 temp_arr.push([geometry[1], geometry[0]]);
//             }
//         }
//         _ => {}
//     }
//     temp_arr
// }
