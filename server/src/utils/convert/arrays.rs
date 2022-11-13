use std::str::FromStr;

use crate::models::scanner::{GenericData, LatLon, MultiPolygonData, SinglePolygonData};
use num_traits::Float;
use serde;

fn ensure_first_last<T>(points: Vec<[T; 2]>) -> Vec<[T; 2]>
where
    T: Float,
{
    let mut points = points;
    if points[0] != points[points.len() - 1] {
        points.push(points[0]);
    }
    points
}

pub fn coord_to_array<T>(coords: Vec<LatLon<T>>) -> Vec<[T; 2]>
where
    T: Float,
{
    coords.iter().map(|p| [p.lat, p.lon]).collect()
}

pub fn data_to_array<T>(coords: Vec<GenericData<T>>) -> Vec<[T; 2]>
where
    T: Float,
{
    coords.iter().map(|p| p.p).collect()
}

pub fn parse_single_polygon<T>(instance_data: &str) -> Vec<Vec<[T; 2]>>
where
    T: Float + serde::de::DeserializeOwned,
{
    let instance_data: SinglePolygonData<T> =
        serde_json::from_str(instance_data).expect("JSON was not well-formatted");
    vec![ensure_first_last(coord_to_array::<T>(instance_data.area))]
}

pub fn parse_multi_polygon<T>(instance_data: &str) -> Vec<Vec<[T; 2]>>
where
    T: Float + serde::de::DeserializeOwned,
{
    let instance_data: MultiPolygonData<T> =
        serde_json::from_str(instance_data).expect("JSON was not well-formatted");
    instance_data
        .area
        .into_iter()
        .map(|p| ensure_first_last(coord_to_array(p)))
        .collect()
}

pub fn parse_flat_text<T>(area_data: &str) -> Vec<Vec<[T; 2]>>
where
    T: FromStr + Float,
{
    let mut points: Vec<[T; 2]> = Vec::new();
    let coords: Vec<&str> = area_data.split(",").collect();
    for coord in coords {
        let lat_lon: Vec<&str> = coord.split_whitespace().collect();
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
    vec![ensure_first_last(points)]
}
