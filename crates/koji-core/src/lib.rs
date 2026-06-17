//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod has_lat_lon;
mod mode;
mod query_args;
pub mod s2;
mod text_utils;
mod unknown_id;

pub use has_lat_lon::HasLatLon;
pub use mode::Mode;
pub use query_args::{BoundsArg, SpawnpointTth};
pub use s2::from_array_to_cell_id;
pub use text_utils::{clean, get_mode_acronym, separate_by_comma};
pub use unknown_id::UnknownId;

pub mod geometry;
mod util;

pub use geometry::*;
pub use util::TrimPrecision;
