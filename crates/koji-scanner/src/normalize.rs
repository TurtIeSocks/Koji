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

/// Shared filter→enumerate→format→collect pipeline behind the four public
/// normalize fns. `prefix` yields each item's id prefix (a constant for forts,
/// despawn-conditional for spawnpoints); `area`, when present, gates items by
/// polygon containment. Enumeration is post-filter, matching the original fns.
fn to_generic_data<T: HasLatLon>(
    items: Vec<T>,
    area: Option<&FeatureCollection>,
    prefix: impl Fn(&T) -> &'static str,
) -> Vec<GenericData> {
    let polys = area.map(AreaPolygons::from_collection);
    items
        .into_iter()
        .filter(|it| polys.as_ref().is_none_or(|p| p.contains(it.lat(), it.lon())))
        .enumerate()
        .map(|(i, it)| GenericData::new(format!("{}{}", prefix(&it), i), it.lat(), it.lon()))
        .collect()
}

pub fn fort(items: Vec<LatLonRow>, prefix: &'static str) -> Vec<GenericData> {
    to_generic_data(items, None, |_| prefix)
}

pub fn fort_filtered(
    items: Vec<LatLonRow>,
    area: &FeatureCollection,
    prefix: &'static str,
) -> Vec<GenericData> {
    to_generic_data(items, Some(area), |_| prefix)
}

pub fn spawnpoint(items: Vec<Spawnpoint>) -> Vec<GenericData> {
    to_generic_data(items, None, |it| {
        if it.despawn_sec.is_some() { "v" } else { "u" }
    })
}

pub fn spawnpoint_filtered(items: Vec<Spawnpoint>, area: &FeatureCollection) -> Vec<GenericData> {
    to_generic_data(items, Some(area), |it| {
        if it.despawn_sec.is_some() { "v" } else { "u" }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rows::{LatLonRow, Spawnpoint};

    #[test]
    fn fort_prefixes_and_enumerates() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 2.0 },
            LatLonRow { lat: 3.0, lon: 4.0 },
        ];
        let out = fort(items, "g");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].i, "g0");
        assert_eq!(out[1].i, "g1");
        // p is [lat, lon] — verify coordinates carry through unchanged.
        assert_eq!(out[0].p, [1.0, 2.0]);
        assert_eq!(out[1].p, [3.0, 4.0]);
    }

    #[test]
    fn spawnpoint_prefix_depends_on_despawn_sec() {
        let items = vec![
            Spawnpoint {
                lat: 1.0,
                lon: 2.0,
                despawn_sec: Some(30),
            },
            Spawnpoint {
                lat: 3.0,
                lon: 4.0,
                despawn_sec: None,
            },
        ];
        let out = spawnpoint(items);
        assert_eq!(out.len(), 2);
        // verified (Some) → "v", unverified (None) → "u"; index is post-filter.
        assert_eq!(out[0].i, "v0");
        assert_eq!(out[1].i, "u1");
    }

    #[test]
    fn area_none_keeps_all_items() {
        let items = vec![
            LatLonRow { lat: 1.0, lon: 2.0 },
            LatLonRow { lat: 3.0, lon: 4.0 },
        ];
        // area=None path (via fort) must retain every item — no polygon gate.
        assert_eq!(to_generic_data(items, None, |_| "g").len(), 2);
    }
}
