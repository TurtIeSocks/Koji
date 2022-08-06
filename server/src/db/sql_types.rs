#![allow(non_camel_case_types)]

use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, DbEnum, Serialize, Deserialize)]
#[DieselType = "Enum"]
pub enum InstanceType {
    auto_quest,
    circle_pokemon,
    circle_smart_pokemon,
    circle_raid,
    circle_smart_raid,
    pokemon_iv,
    leveling,
}
