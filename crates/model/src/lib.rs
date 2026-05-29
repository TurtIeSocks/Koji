use std::fmt::Display;

use geojson::{Feature, FeatureCollection};
use log;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};

pub mod api;
pub mod db;
pub mod error;
pub mod utils;

#[derive(Debug, Clone)]
pub enum ScannerType {
    RDM,
    Unown,
    Hybrid,
}

impl Serialize for ScannerType {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ScannerType::RDM => serializer.serialize_str("rdm"),
            ScannerType::Unown => serializer.serialize_str("unown"),
            ScannerType::Hybrid => serializer.serialize_str("hybrid"),
        }
    }
}

impl Display for ScannerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScannerType::RDM => write!(f, "rdm"),
            ScannerType::Unown => write!(f, "unown"),
            ScannerType::Hybrid => write!(f, "hybrid"),
        }
    }
}

impl PartialEq for ScannerType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ScannerType::RDM, ScannerType::RDM) => true,
            (ScannerType::Unown, ScannerType::Unown) => true,
            (ScannerType::Hybrid, ScannerType::Hybrid) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct KojiDb {
    pub koji: DatabaseConnection,
    pub scanner: DatabaseConnection,
    pub controller: DatabaseConnection,
    pub scanner_type: ScannerType,
}
