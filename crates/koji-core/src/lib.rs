//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod category;
mod enum_map;
mod mode;
mod normalize;
mod query_args;
pub mod s2;
mod text_utils;
mod unknown_id;

pub use category::Category;
pub use enum_map::get_category_enum;
pub use mode::Mode;
pub use normalize::HasLatLon;
pub use query_args::{
    AdminReq, AdminReqParsed, ApiQueryArgs, BoundsArg, FeatureRenderSpec, Filters, OutputSpec,
    PropertySelection, SpawnpointTth,
};
pub use s2::{create_cell_map, from_array_to_cell_id};
pub use text_utils::{NameModifier, clean, get_mode_acronym, json_related_sort, separate_by_comma};
pub use unknown_id::UnknownId;

pub mod geometry;
mod util;

pub use geometry::*;
pub use util::{TrimPrecision, sql_raw};

/// Generates 1:1 `From` impls both directions between two enums whose variant
/// idents match exactly. Invoke from a crate that owns at least one of the two
/// enums (orphan rule).
///
/// ```ignore
/// koji_core::enum_bridge!(db::Type, koji_core::Mode, [CirclePokemon, ...]);
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
