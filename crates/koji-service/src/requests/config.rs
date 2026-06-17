//! Request/output-side configuration structs. The v2 arg-group wire DTOs
//! ([`super::groups`]) `resolve()` into these; downstream ops + the response
//! layer consume them. `DataFilter` carries a [`SpawnpointTth`] from `koji_core`.

use koji_core::SpawnpointTth;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone)]
pub struct DataFilter {
    pub last_seen: u32,
    pub tth: SpawnpointTth,
}

#[derive(Debug, Clone)]
pub struct OutputConfig {
    pub return_type: ReturnTypeArg,
    pub save_to_db: bool,
    pub save_to_scanner: bool,
    pub save_to_scanner_only: bool,
    pub simplify: bool,
}

/// Developer / experimental toggles + benchmark mode.
#[derive(Debug, Clone, Default)]
pub struct DevConfig {
    pub benchmark_mode: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, ToSchema)]
pub enum ReturnTypeArg {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Geometry,
    Feature,
    FeatureCollection,
    Poracle,
    Sql,
}

pub fn get_return_type(return_type: String, default_return_type: &ReturnTypeArg) -> ReturnTypeArg {
    match return_type.to_lowercase().replace("-", "_").as_str() {
        "alttext" | "alt_text" => ReturnTypeArg::AltText,
        "text" => ReturnTypeArg::Text,
        "array" => match *default_return_type {
            ReturnTypeArg::SingleArray => ReturnTypeArg::SingleArray,
            ReturnTypeArg::MultiArray => ReturnTypeArg::MultiArray,
            _ => ReturnTypeArg::SingleArray,
        },
        "singlearray" | "single_array" => ReturnTypeArg::SingleArray,
        "multiarray" | "multi_array" => ReturnTypeArg::MultiArray,
        "struct" => match *default_return_type {
            ReturnTypeArg::SingleStruct => ReturnTypeArg::SingleStruct,
            ReturnTypeArg::MultiStruct => ReturnTypeArg::MultiStruct,
            _ => ReturnTypeArg::SingleStruct,
        },
        // The bare-array `geometryvec`/`featurevec` output variants were removed
        // (locked wire decision: clients use the collection forms). The legacy
        // request strings now map to their collection replacements — `Geometry`
        // is a `GeometryCollection`, `FeatureCollection` a `FeatureCollection`.
        "geometry" | "geometryvec" | "geometry_vec" | "geometries" => ReturnTypeArg::Geometry,
        "singlestruct" | "single_struct" => ReturnTypeArg::SingleStruct,
        "multistruct" | "multi_struct" => ReturnTypeArg::MultiStruct,
        "feature" => ReturnTypeArg::Feature,
        "featurevec" | "feature_vec" => ReturnTypeArg::FeatureCollection,
        "poracle" => ReturnTypeArg::Poracle,
        "featurecollection" | "feature_collection" => ReturnTypeArg::FeatureCollection,
        "sql" => ReturnTypeArg::Sql,
        _ => default_return_type.clone(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn get_return_type_maps_aliases_and_falls_back() {
        use super::{ReturnTypeArg, get_return_type};
        assert_eq!(
            get_return_type("feature_collection".into(), &ReturnTypeArg::SingleArray),
            ReturnTypeArg::FeatureCollection
        );
        assert_eq!(
            get_return_type("alt-text".into(), &ReturnTypeArg::SingleArray),
            ReturnTypeArg::AltText
        );
        assert_eq!(
            get_return_type("nonsense".into(), &ReturnTypeArg::Feature),
            ReturnTypeArg::Feature
        );
    }
}
