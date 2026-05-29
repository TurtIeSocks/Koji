use geo::{Contains, MultiPolygon, Point, Polygon};
use geojson::{FeatureCollection, Value};
use serde_json::json;

use super::*;

use crate::{
    api::text::TextHelpers,
    db::{AreaRef, Spawnpoint, sea_orm_active_enums::Type},
};

pub trait HasLatLon {
    fn lat(&self) -> f64;
    fn lon(&self) -> f64;
}

impl HasLatLon for Spawnpoint {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

impl HasLatLon for api::point_struct::PointStruct {
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

pub fn fort(items: api::single_struct::SingleStruct, prefix: &str) -> Vec<db::GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn fort_filtered(
    items: api::single_struct::SingleStruct,
    area: &FeatureCollection,
    prefix: &str,
) -> Vec<db::GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
}

pub fn count_in_area<T: HasLatLon>(items: &[T], area: &FeatureCollection) -> i32 {
    let polygons = AreaPolygons::from_collection(area);
    items
        .iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .count() as i32
}

pub fn spawnpoint(items: Vec<db::Spawnpoint>) -> Vec<db::GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            db::GenericData::new(
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

pub fn spawnpoint_filtered(
    items: Vec<Spawnpoint>,
    area: &FeatureCollection,
) -> Vec<db::GenericData> {
    let polygons = AreaPolygons::from_collection(area);
    items
        .into_iter()
        .filter(|item| polygons.contains(item.lat(), item.lon()))
        .enumerate()
        .map(|(i, item)| {
            db::GenericData::new(
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

pub fn instance(instance: db::instance::Model) -> Feature {
    instance
        .data
        .parse_scanner_instance(Some(instance.name), Some(instance.r#type))
}

pub fn area(areas: Vec<db::area::Model>) -> Vec<Feature> {
    let mut normalized = Vec::<Feature>::new();

    let mut to_feature = |fence: Option<String>, name: &String, mode: Type| {
        if let Some(fence) = fence {
            if !fence.is_empty() {
                normalized.push(fence.parse_scanner_instance(Some(name.to_string()), Some(mode)));
            }
        }
    };
    for area in areas.into_iter() {
        to_feature(area.geofence, &area.name, Type::AutoQuest);
        to_feature(area.fort_mode_route, &area.name, Type::CircleRaid);
        to_feature(area.quest_mode_route, &area.name, Type::CircleQuest);
        to_feature(area.pokemon_mode_route, &area.name, Type::CirclePokemon);
    }
    normalized
}

pub fn area_ref(areas: Vec<AreaRef>) -> Vec<sea_orm::JsonValue> {
    let mut normalized = Vec::<sea_orm::JsonValue>::new();

    for area in areas.into_iter() {
        if area.has_geofence {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "auto_quest",
                "geo_type": "MultiPolygon",
            }));
        }
        if area.has_fort {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_raid",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_pokemon {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_pokemon",
                "geo_type": "MultiPoint",
            }));
        }
        if area.has_quest {
            normalized.push(json!({
                "id": area.id,
                "name": area.name,
                "mode": "circle_quest",
                "geo_type": "MultiPoint",
            }));
        }
    }
    normalized
}
