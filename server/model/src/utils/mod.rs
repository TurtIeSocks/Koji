use super::*;

use std::env;

use geojson::Value;
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, Order};

use crate::{
    api::{args::ApiQueryArgs, EnsurePoints, GetBbox, ToSingleVec},
    db::sea_orm_active_enums::{Category, Type},
};

pub mod json;
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
                    string = format!("{}{} (lon BETWEEN {} AND {}\nAND lat BETWEEN {} AND {}\nAND ST_CONTAINS(ST_GeomFromGeoJSON('{}', 2, 0), POINT(lon, lat)))",
                        string,
                        if i == 0 { "" } else { " OR" },
                        bbox[0], bbox[2], bbox[1], bbox[3], geo.to_string()
                    );
                }
                _ => {}
            }
        }
    }
    string
}

pub fn get_enum(instance_type: Option<String>) -> Type {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => Type::AutoQuest,
            "CirclePokemon" | "circle_pokemon" => Type::CirclePokemon,
            "CircleSmartPokemon" | "circle_smart_pokemon" => Type::CircleSmartPokemon,
            "CircleRaid" | "circle_raid" => Type::CircleRaid,
            "CircleSmartRaid" | "circle_smart_raid" => Type::CircleSmartRaid,
            "PokemonIv" | "pokemon_iv" => Type::PokemonIv,
            "Leveling" | "leveling" => Type::Leveling,
            "CircleQuest" | "circle_quest" => Type::CircleQuest,
            "AutoTth" | "auto_tth" => Type::AutoTth,
            "AutoPokemon" | "auto_pokemon" => Type::AutoPokemon,
            _ => Type::Unset,
        },
        None => Type::Unset,
    }
}

pub fn get_enum_by_geometry(enum_val: &Value) -> Type {
    match enum_val {
        Value::Point(_) => Type::Leveling,
        Value::MultiPoint(_) => Type::CircleSmartPokemon,
        Value::Polygon(_) => Type::PokemonIv,
        Value::MultiPolygon(_) => Type::AutoQuest,
        _ => {
            log::warn!("Invalid Geometry Type: {}", enum_val.type_name());
            Type::Unset
        }
    }
}

pub fn get_category_enum(category: String) -> Category {
    match category.to_lowercase().as_str() {
        "database" => Category::Database,
        "boolean" => Category::Boolean,
        "number" => Category::Number,
        "object" => Category::Object,
        "array" => Category::Array,
        "color" => Category::Color,
        _ => Category::String,
    }
}

pub fn get_mode_acronym(instance_type: Option<&String>) -> String {
    match instance_type {
        Some(instance_type) => match instance_type.as_str() {
            "AutoQuest" | "auto_quest" => "AQ",
            "CirclePokemon" | "circle_pokemon" => "CP",
            "CircleSmartPokemon" | "circle_smart_pokemon" => "CSP",
            "CircleRaid" | "circle_raid" => "CR",
            "CircleSmartRaid" | "circle_smart_raid" => "CSR",
            "PokemonIv" | "pokemon_iv" => "IV",
            "Leveling" | "leveling" => "L",
            "CircleQuest" | "circle_quest" => "CQ",
            "AutoTth" | "auto_tth" => "ATTH",
            "AutoPokemon" | "auto_pokemon" => "AP",
            _ => "U",
        },
        None => "U",
    }
    .to_string()
}

pub fn get_enum_by_geometry_string(input: Option<String>) -> Option<Type> {
    if let Some(input) = input {
        match input.to_lowercase().as_str() {
            "point" => Some(Type::Leveling),
            "multipoint" => Some(Type::CirclePokemon),
            "multipolygon" => Some(Type::AutoQuest),
            _ => None,
        }
    } else {
        None
    }
}

pub async fn get_database_struct() -> KojiDb {
    let koji_db_url = env::var("KOJI_DB_URL").expect("Need KOJI_DB_URL env var to run migrations");
    let scanner_db_url = if env::var("DATABASE_URL").is_ok() {
        log::warn!("[WARNING] `DATABASE_URL` is deprecated in favor of `SCANNER_DB_URL`");
        env::var("DATABASE_URL")
    } else {
        env::var("SCANNER_DB_URL")
    }
    .expect("Need SCANNER_DB_URL env var");

    let max_connections: u32 = if let Ok(parsed) = env::var("MAX_CONNECTIONS")
        .unwrap_or("100".to_string())
        .parse()
    {
        parsed
    } else {
        100
    };

    let unown_db_url =
        env::var("UNOWN_DB_URL").unwrap_or(env::var("UNOWN_DB").unwrap_or("".to_string()));

    KojiDb {
        data_db: {
            let mut opt = ConnectOptions::new(scanner_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Scanner DB: {}", err),
            }
        },
        koji_db: {
            let mut opt = ConnectOptions::new(koji_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => db,
                Err(err) => panic!("Cannot connect to Kōji DB: {}", err),
            }
        },
        unown_db: if unown_db_url.is_empty() {
            None
        } else {
            let mut opt = ConnectOptions::new(unown_db_url);
            opt.max_connections(max_connections);
            opt.sqlx_logging_level(LevelFilter::Debug);
            match Database::connect(opt).await {
                Ok(db) => Some(db),
                Err(err) => panic!("Cannot connect to Unown DB: {}", err),
            }
        },
    }
}

pub fn parse_order(order_by: &String) -> Order {
    if order_by.to_lowercase().eq("asc") {
        Order::Asc
    } else {
        Order::Desc
    }
}

pub fn json_related_sort(json: &mut Vec<serde_json::Value>, sort_by: &String, order: String) {
    json.sort_by(|a, b| {
        let a = a[sort_by].as_array().unwrap().len();
        let b = b[sort_by].as_array().unwrap().len();
        if order == "asc" {
            a.cmp(&b)
        } else {
            b.cmp(&a)
        }
    });
}

pub fn clean(param: &String) -> String {
    if param.starts_with("\"") && param.ends_with("\"") {
        return param[1..param.len() - 1].to_string();
    }
    param.to_string()
}

pub fn name_modifier(string: String, modifiers: &ApiQueryArgs, parent: Option<String>) -> String {
    let mut mutable = string.clone();
    if let Some(trimstart) = modifiers.trimstart {
        mutable = (&mutable[trimstart..]).to_string();
    }
    if let Some(trimend) = modifiers.trimend {
        mutable = (&mutable[..(mutable.len() - trimend)]).to_string();
    }
    if let Some(replacer) = modifiers.replace.as_ref() {
        mutable = mutable.replace(clean(replacer).as_str(), "");
    }
    if parent.is_some() {
        let parent = parent.unwrap();
        if let Some(replacer) = modifiers.parentreplace.as_ref() {
            mutable = mutable.replace(&parent, clean(replacer).as_str());
        }
        if let Some(parent_start) = modifiers.parentstart.as_ref() {
            mutable = format!("{}{}{}", parent, clean(parent_start), mutable,);
        }
        if let Some(parent_end) = modifiers.parentend.as_ref() {
            mutable = format!("{}{}{}", mutable, clean(parent_end), parent,);
        }
    }
    if modifiers.lowercase.is_some() {
        mutable = mutable.to_lowercase();
    }
    if modifiers.uppercase.is_some() {
        mutable = mutable.to_uppercase();
    }
    if modifiers.capfirst.is_some() {
        mutable = mutable
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if i == 0 {
                    c.to_uppercase().to_string()
                } else {
                    c.to_string()
                }
            })
            .collect();
    }
    if let Some(capitalize) = modifiers.capitalize.as_ref() {
        mutable = mutable
            .split(clean(capitalize).as_str())
            .map(|word| {
                word.chars()
                    .enumerate()
                    .map(|(i, c)| {
                        if i == 0 {
                            c.to_uppercase().to_string()
                        } else {
                            c.to_string()
                        }
                    })
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join(clean(capitalize).as_str());
    }
    if let Some(replacer) = modifiers.underscore.as_ref() {
        mutable = mutable.replace("_", clean(replacer).as_str());
    }
    if let Some(replacer) = modifiers.dash.as_ref() {
        mutable = mutable.replace("-", clean(replacer).as_str());
    }
    if let Some(replacer) = modifiers.space.as_ref() {
        mutable = mutable.replace(" ", clean(replacer).as_str());
    }
    mutable = mutable.trim().to_string();
    if mutable == "" {
        log::warn!(
            "Empty string detected for {} {:?}, returning the standard name",
            string,
            modifiers
        );
        string
    } else {
        mutable
    }
}
