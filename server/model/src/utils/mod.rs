use super::*;

use geojson::Value;

use crate::{api::{ToSingleVec, GetBbox, EnsurePoints}, db::sea_orm_active_enums::Type};

pub mod normalize;

pub fn sql_raw(area: &FeatureCollection) -> String {
  let mut string = "".to_string();
  for (i, feature) in area.into_iter().enumerate() {
      let bbox = if let Some(bbox) = feature.bbox.clone() {
          bbox
      } else {
          feature.clone().to_single_vec().get_bbox().unwrap()
      };
      if let Some(geometry) = feature.geometry.clone() {
          let geo = geometry.ensure_first_last();
          match geo.value {
              Value::Polygon(_) | Value::MultiPolygon(_) => {
                  string = format!(
                      "{}{} (lon BETWEEN {} AND {}\nAND lat BETWEEN {} AND {}\nAND ST_CONTAINS(ST_GeomFromGeoJSON('{}', 2, 0), POINT(lon, lat)))",
                      string,
                      if i == 0 {
                          ""
                      } else {
                          " OR"
                      },
                      bbox[0], 
                      bbox[2],
                      bbox[1],
                      bbox[3],
                      geo.to_string()
                  );
              }
              _ => {}
          }
      }
  }
  string
}

pub fn get_enum(instance_type: Option<String>) -> Option<Type> {
  match instance_type {
      Some(instance_type) => match instance_type.as_str() {
          "AutoQuest" | "auto_quest" => Some(Type::AutoQuest),
          "CirclePokemon" | "circle_pokemon" => Some(Type::CirclePokemon),
          "CircleSmartPokemon" | "circle_smart_pokemon" => Some(Type::CircleSmartPokemon),
          "CircleRaid" | "circle_raid" => Some(Type::CircleRaid),
          "CircleSmartRaid" | "circle_smart_raid" => Some(Type::CircleSmartRaid),
          "PokemonIv" | "pokemon_iv" => Some(Type::PokemonIv),
          "Leveling" | "leveling" => Some(Type::Leveling),
          _ => None,
      },
      None => None,
  }
}

pub fn get_enum_by_geometry(enum_val: &Value) -> Option<Type> {
  match enum_val {
      Value::Point(_) => Some(Type::Leveling),
      Value::MultiPoint(_) => Some(Type::CirclePokemon),
      Value::Polygon(_) => Some(Type::PokemonIv),
      Value::MultiPolygon(_) => Some(Type::AutoQuest),
      _ => {
          println!("Invalid Geometry Type");
          None
      }
  }
}
