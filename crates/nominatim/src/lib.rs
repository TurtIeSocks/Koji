#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::struct_excessive_bools)]

/*
  Borrowing for now while I need the changes
  https://crates.io/crates/nominatim-rs
*/

pub mod client;
pub mod error;
pub mod lookup;
pub mod reverse;
pub mod search;
pub mod serde_utils;
pub mod types;
pub mod util;

pub use client::Client;
pub use lookup::LookupQueryBuilder;
pub use reverse::ReverseQueryBuilder;
pub use reverse::Zoom;
pub use search::LocationQuery;
pub use search::SearchQueryBuilder;
pub use types::Response;
