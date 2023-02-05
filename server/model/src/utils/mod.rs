use super::*;

use geojson::Value;
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database};
use std::env;

use crate::{
    api::{EnsurePoints, GetBbox, ToSingleVec},
    db::sea_orm_active_enums::Type,
};

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
            "ManualQuest" | "manual_quest" => Some(Type::ManualQuest),
            "AutoTth" | "auto_tth" => Some(Type::AutoTth),
            "AutoPokemon" | "auto_pokemon" => Some(Type::AutoPokemon),
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
            log::warn!("Invalid Geometry Type: {}", enum_val.type_name());
            None
        }
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
            "ManualQuest" | "manual_quest" => "MQ",
            "AutoTth" | "auto_tth" => "ATTH",
            "AutoPokemon" | "auto_pokemon" => "AP",
            _ => "",
        },
        None => "",
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
        println!("[WARNING] `DATABASE_URL` is deprecated in favor of `SCANNER_DB_URL`");
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

    let unown_db_url = env::var("UNOWN_DB").unwrap_or("".to_string());

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
                Err(err) => panic!("Cannot connect to Koji DB: {}", err),
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
