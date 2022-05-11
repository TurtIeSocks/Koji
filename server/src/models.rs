use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Spawnpoint {
    pub id: u64,
    pub lat: f64,
    pub lon: f64,
    pub updated: u32,
    pub last_seen: u32,
    pub despawn_sec: Option<u16>,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Pokestop {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    // pub name: Option<String>,
    // pub url: Option<String>,
    // pub last_modified_timestamp: Option<u32>,
    // pub updated: u32,
    // pub enabled: Option<u8>,
    // pub cell_id: Option<u64>,
    // pub deleted: u8,
    // pub first_seen_timestamp: u32,
    // pub sponsor_id: Option<u16>,
    // pub partner_id: Option<String>,
    // pub ar_scan_eligible: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize, Queryable)]
pub struct Gym {
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    // pub name: Option<String>,
    // pub url: Option<String>,
    // pub last_modified_timestamp: Option<u32>,
    // pub updated: u32,
    // pub enabled: Option<u8>,
    // pub ex_raid_eligible: Option<u8>,
    // pub cell_id: Option<u64>,
    // pub deleted: u8,
    // pub first_seen_timestamp: u32,
    // pub sponsor_id: Option<u16>,
    // pub ar_scan_eligible: Option<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Body {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}