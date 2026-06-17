//! The v2 public API: a clean, best-practices surface over the P3 infra
//! (koji-jobs / events / dragonite). Calc runs through the job queue; responses
//! use the [ApiResponse](crate::utils::api_response) envelope.
//!
//! Wired under `web::scope("/api/v2")` in [`crate::start`]. `/api/v1` is left
//! exactly as-is (the legacy shim).

pub(crate) mod auth;
pub(crate) mod calc;
pub(crate) mod config;
pub(crate) mod geofences;
pub(crate) mod geometry;
pub(crate) mod jobs;
pub(crate) mod nominatim;
pub(crate) mod plugins;
pub(crate) mod resources;
pub(crate) mod routes;
pub(crate) mod s2;
pub(crate) mod golbat_data;
