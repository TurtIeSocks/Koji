//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod calc_mode;
mod category;
mod cluster_mode;
mod config;
mod enum_map;
mod feature_ctx;
mod fence_type;
mod geo_formats;
mod mode;
mod normalize;
mod query_args;
mod return_type;
pub mod s2;
mod sort_by;
mod text_utils;
mod unknown_id;

pub use calc_mode::CalculationMode;
pub use category::Category;
pub use cluster_mode::ClusterMode;
pub use config::{
    AreaInput, BootstrapConfig, ClusteringConfig, DataFilter, DevConfig, OutputConfig,
    RoutingConfig, S2Config,
};
pub use enum_map::{
    get_category_enum, get_enum, get_enum_by_geometry, get_enum_by_geometry_string,
};
pub use feature_ctx::FeatureCtx;
pub use fence_type::FenceType;
pub use geo_formats::GeoFormats;
pub use mode::Mode;
pub use normalize::{AreaPolygons, HasLatLon, count_in_area};
pub use query_args::{AdminReq, AdminReqParsed, ApiQueryArgs, BoundsArg, SpawnpointTth};
pub use return_type::ReturnTypeArg;
pub use s2::{create_cell_map, from_array_to_cell_id};
pub use sort_by::SortBy;
pub use text_utils::{
    clean, get_mode_acronym, json_related_sort, name_modifier, separate_by_comma,
};
pub use unknown_id::UnknownId;

pub mod geometry;
mod util;

pub use geometry::*;
pub use util::{TrimPrecision, sql_raw, sql_raw_bbox};

/// Generates 1:1 `From` impls both directions between two enums whose variant
/// idents match exactly. Invoke from a crate that owns at least one of the two
/// enums (orphan rule).
///
/// ```ignore
/// koji_core::enum_bridge!(db::Type, koji_core::FenceType, [CirclePokemon, ...]);
/// ```
#[macro_export]
macro_rules! enum_bridge {
    ($a:ty, $b:ty, [$($variant:ident),+ $(,)?]) => {
        impl ::core::convert::From<$a> for $b {
            fn from(value: $a) -> Self {
                match value { $(<$a>::$variant => <$b>::$variant,)+ }
            }
        }
        impl ::core::convert::From<$b> for $a {
            fn from(value: $b) -> Self {
                match value { $(<$b>::$variant => <$a>::$variant,)+ }
            }
        }
    };
}
