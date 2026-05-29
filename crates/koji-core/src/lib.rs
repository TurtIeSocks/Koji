//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod calc_mode;
mod category;
mod cluster_mode;
mod fence_type;
mod sort_by;
mod unknown_id;

pub use calc_mode::CalculationMode;
pub use category::Category;
pub use cluster_mode::ClusterMode;
pub use fence_type::FenceType;
pub use sort_by::SortBy;
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
