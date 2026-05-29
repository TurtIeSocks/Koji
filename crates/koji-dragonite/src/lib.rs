//! # koji-dragonite — typed client over Dragonite's `/v2/areas/*` API
//!
//! ⚠️ **wire types are provisional — reconcile with Dragonite's real `/v2/areas`
//! API before P5 wires this client.** There is no Dragonite OpenAPI/schema in
//! this repo, so the exact JSON field names and the [`ApiArea`]/[`ApiGeofence`]/
//! [`V2GeofencePatch`] shapes are unverified guesses. Every speculative field
//! and endpoint carries a `// TODO(dragonite-reconcile): ...` marker. No request
//! in this crate has been run against a live Dragonite — all HTTP is
//! runtime-unverified.
//!
//! ## What is SOUND and reusable
//! - [`JSend`] — the generic JSend envelope (a published spec Koji V2 adopts).
//! - [`Tri`] — the tri-state PATCH wrapper (absent / null / value), per the
//!   architecture's `V2GeofencePatch` semantics (§7).
//! - [`DragoniteClient`] plumbing — reqwest setup, `Bearer` + `User-Agent`
//!   headers, JSend→`Result` collapsing, page-walking.
//! - [`route_to_dragonite`] — Koji `SingleVec` (`Vec<[f64;2]>`, `[lat,lon]`) →
//!   Dragonite route array, grounded in koji-core (identity, ordering tested).
//!
//! ## What is PROVISIONAL (reconcile before P5)
//! - Endpoint paths + pagination contract in [`DragoniteClient`].
//! - All field names in [`types`] (the [`AreaMode`] enum itself is sound — it
//!   mirrors the existing `area_fence.mode` migration ENUM).
//! - [`feature_to_api_geofence`] target representation.

pub mod client;
pub mod error;
pub mod jsend;
pub mod mapping;
pub mod patch;
pub mod types;

pub use client::DragoniteClient;
pub use error::DragoniteError;
pub use jsend::{parse_jsend, JSend};
pub use mapping::{feature_to_api_geofence, route_to_dragonite};
pub use patch::Tri;
pub use types::{ApiArea, ApiGeofence, AreaMode, V2GeofencePatch};
