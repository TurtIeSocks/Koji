use geo::{Contains, MultiPolygon, Point, Polygon};
use geojson::{FeatureCollection, Value};
use serde_json::json;

use super::*;

use crate::{
    api::text::TextHelpers,
    db::{sea_orm_active_enums::Type, AreaRef, Spawnpoint},
};

pub fn fort(items: api::single_struct::SingleStruct, prefix: &str) -> Vec<db::GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| db::GenericData::new(format!("{}{}", prefix, i), item.lat, item.lon))
        .collect()
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

pub fn spawnpoint_filtered(items: Vec<Spawnpoint>, area: &FeatureCollection) -> Vec<db::GenericData> {
    let polygons: Vec<(Vec<Polygon<f64>>, Vec<MultiPolygon<f64>>)> = area
        .features
        .iter()
        .filter_map(|feature| {
            feature.geometry.as_ref().map(|geometry| {
                let mut polys = Vec::new();
                let mut multi_polys = Vec::new();
                match &geometry.value {
                    Value::Polygon(_) => {
                        if let Ok(poly) = Polygon::try_from(geometry) {
                            polys.push(poly);
                        }
                    }
                    Value::MultiPolygon(_) => {
                        if let Ok(mp) = MultiPolygon::try_from(geometry) {
                            multi_polys.push(mp);
                        }
                    }
                    _ => {}
                }
                (polys, multi_polys)
            })
        })
        .collect();

    items
        .into_iter()
        .filter(|item| {
            let point = Point::new(item.lon, item.lat);
            polygons.iter().any(|(polys, multi_polys)| {
                polys.iter().any(|poly| poly.contains(&point))
                    || multi_polys.iter().any(|mp| mp.contains(&point))
            })
        })
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

pub fn count_spawnpoints_in_area(items: &[Spawnpoint], area: &FeatureCollection) -> i32 {
    let polygons: Vec<(Vec<Polygon<f64>>, Vec<MultiPolygon<f64>>)> = area
        .features
        .iter()
        .filter_map(|feature| {
            feature.geometry.as_ref().map(|geometry| {
                let mut polys = Vec::new();
                let mut multi_polys = Vec::new();
                match &geometry.value {
                    Value::Polygon(_) => {
                        if let Ok(poly) = Polygon::try_from(geometry) {
                            polys.push(poly);
                        }
                    }
                    Value::MultiPolygon(_) => {
                        if let Ok(mp) = MultiPolygon::try_from(geometry) {
                            multi_polys.push(mp);
                        }
                    }
                    _ => {}
                }
                (polys, multi_polys)
            })
        })
        .collect();

    items
        .iter()
        .filter(|item| {
            let point = Point::new(item.lon, item.lat);
            polygons.iter().any(|(polys, multi_polys)| {
                polys.iter().any(|poly| poly.contains(&point))
                    || multi_polys.iter().any(|mp| mp.contains(&point))
            })
        })
        .count() as i32
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
