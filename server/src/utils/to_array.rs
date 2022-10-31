use crate::models::scanner::{GenericData, LatLon};

pub fn coord_to_array(coords: Vec<LatLon>) -> Vec<[f64; 2]> {
    coords.iter().map(|p| [p.lat, p.lon]).collect()
}

pub fn data_to_array(coords: Vec<GenericData>) -> Vec<[f64; 2]> {
    coords.iter().map(|p| p.p).collect()
}
