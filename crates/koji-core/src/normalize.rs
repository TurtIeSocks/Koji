use geo::{Contains, MultiPolygon, Point, Polygon};
use geojson::{FeatureCollection, Value};

use crate::PointStruct;

pub trait HasLatLon {
    fn lat(&self) -> f64;
    fn lon(&self) -> f64;
}

impl HasLatLon for PointStruct {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

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
