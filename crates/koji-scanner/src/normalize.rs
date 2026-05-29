use geojson::FeatureCollection;

use koji_core::{AreaPolygons, HasLatLon};

use crate::rows::{GenericData, LatLonRow, Spawnpoint};

impl HasLatLon for Spawnpoint {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

impl HasLatLon for LatLonRow {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

pub fn fort(items: Vec<LatLonRow>, prefix: &str) -> Vec<GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn fort_filtered(
    items: Vec<LatLonRow>,
    area: &FeatureCollection,
    prefix: &str,
) -> Vec<GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn spawnpoint(items: Vec<Spawnpoint>) -> Vec<GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            GenericData::new(
                format!("{}{}", if item.despawn_sec.is_some() { "v" } else { "u" }, i),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub fn spawnpoint_filtered(items: Vec<Spawnpoint>, area: &FeatureCollection) -> Vec<GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| {
            GenericData::new(
                format!("{}{}", if item.despawn_sec.is_some() { "v" } else { "u" }, i),
                item.lat,
                item.lon,
            )
        })
        .collect()
}
