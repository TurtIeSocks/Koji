//! Auth + search wire structs for the private admin/session surface.
//!
//! `Auth` (the login password body) and `Search` (the `?query=` admin/nominatim
//! search param) were moved verbatim out of the dissolving `model` crate. They are
//! plain serde DTOs with no behavior — re-homed here so `model` has no remaining
//! koji-service importers.

use serde::{Deserialize, Serialize};

/// Login request body for `POST /private/login`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Auth {
    pub password: String,
}

/// `?query=` search parameter shared by the admin `search` endpoint and the
/// nominatim proxy.
#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}
