//! The v2 public API: a clean, best-practices surface over the P3 infra
//! (koji-jobs / events / dragonite). Calc runs through the job queue; responses
//! use the [ApiResponse](crate::utils::api_response) envelope.
//!
//! Wired under `web::scope("/api/v2")` in [`crate::start`]. `/api/v1` is left
//! exactly as-is (the legacy shim).

pub mod calc;
pub mod geofences;
pub mod jobs;
pub mod resources;
pub mod routes;
