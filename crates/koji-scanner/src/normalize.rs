use geo::{Contains, MultiPolygon, Point, Polygon};
use geojson::{FeatureCollection, Value};

use koji_core::HasLatLon;

use crate::rows::{GenericData, LatLonRow, Spawnpoint};

pub struct AreaPolygons {
    polys: Vec<Polygon<f64>>,
    multi_polys: Vec<MultiPolygon<f64>>,
}

impl AreaPolygons {
    pub fn from_collection(area: &FeatureCollection) -> Self {
        let mut polys = Vec::new();
        let mut multi_polys = Vec::new();

        for feature in &area.features {
            if let Some(geometry) = &feature.geometry {
                match &geometry.value {
                    Value::Polygon(_) => match Polygon::try_from(geometry) {
                        Ok(poly) => polys.push(poly),
                        Err(e) => log::warn!("Failed to convert Polygon: {}", e),
                    },
                    Value::MultiPolygon(_) => match MultiPolygon::try_from(geometry) {
                        Ok(mp) => multi_polys.push(mp),
                        Err(e) => log::warn!("Failed to convert MultiPolygon: {}", e),
                    },
                    _ => {}
                }
            }
        }

        Self { polys, multi_polys }
    }

    pub fn contains(&self, lat: f64, lon: f64) -> bool {
        let point = Point::new(lon, lat);
        self.polys.iter().any(|poly| poly.contains(&point))
            || self.multi_polys.iter().any(|mp| mp.contains(&point))
    }
}

pub fn count_in_area<T: HasLatLon>(items: &[T], area: &FeatureCollection) -> i32 {
    let polygons = AreaPolygons::from_collection(area);
    items
        .iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .count() as i32
}

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
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
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
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
                item.lat,
                item.lon,
            )
        })
        .collect()
}
