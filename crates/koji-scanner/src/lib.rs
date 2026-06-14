//! koji-scanner — read-only access to the golbat data (scanner) database.
//! Depends only on koji-core; queries take a `&DatabaseConnection` (the
//! caller's `scanner` connection) and return `sea_orm::DbErr` directly.

pub mod entities;
mod normalize;
pub mod prelude;
mod rows;
mod util;

pub use normalize::{AreaPolygons, count_in_area, fort, fort_filtered, spawnpoint, spawnpoint_filtered};
pub use rows::{GenericData, GenericDataToVec, LatLonRow, Spawnpoint, Total};
pub use util::sql_raw_bbox;
