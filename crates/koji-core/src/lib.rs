//! koji-core — pure domain types shared across the Koji workspace.
//! No database, HTTP, or async dependencies.

mod category;
mod mode;
mod normalize;
mod query_args;
pub mod s2;
mod text_utils;
mod unknown_id;

pub use category::Category;
pub use mode::Mode;
pub use normalize::HasLatLon;
pub use query_args::{
    AdminReq, AdminReqParsed, ApiQueryArgs, BoundsArg, FeatureRenderSpec, Filters, OutputSpec,
    PropertySelection, SpawnpointTth,
};
pub use s2::{create_cell_map, from_array_to_cell_id};
pub use text_utils::{NameModifier, clean, get_mode_acronym, separate_by_comma};
pub use unknown_id::UnknownId;

pub mod geometry;
mod util;

pub use geometry::*;
pub use util::{TrimPrecision, sql_raw};
