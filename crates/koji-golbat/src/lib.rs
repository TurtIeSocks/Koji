//! koji-golbat — read-only access to the golbat data (golbat) database.
//! Depends only on koji-core; queries take a `&DatabaseConnection` (the
//! caller's `golbat` connection) and return `sea_orm::DbErr` directly.

pub mod entities;
mod normalize;
mod rows;
mod util;

pub use normalize::{count_in_area, fort, fort_filtered, spawnpoint, spawnpoint_filtered};
pub use rows::{GenericData, GenericDataToVec, LatLonRow, Spawnpoint, Total};
pub use util::sql_raw_bbox;
