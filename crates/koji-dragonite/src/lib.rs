//! # koji-dragonite — typed client over Dragonite's `/v2/areas/*` API
//!
//! Reconciled against Dragonite's real V2 area contract (`routes/areas.go`,
//! `routes/v2_areas.go`, `routes/v2_geofence*.go`, `routes/v2_envelope.go`):
//!
//! - **Endpoints** ([`DragoniteClient`]): `GET|POST /v2/areas/`,
//!   `GET|PATCH|DELETE /v2/areas/{id}`, zero-based `?page`/`?per_page` (max
//!   1000) pagination with a `V2Meta` block, `?q=` name filter. `DELETE` returns
//!   `204`.
//! - **Envelope** ([`envelope`]): the V2 `{status:"ok"|"error", data, meta,
//!   error{code,message,field}}` shape — *not* JSend.
//! - **Area shape** ([`types`]): the full [`ApiArea`] with per-mode blocks; the
//!   geofence field is a single tri-state nullable GeoJSON `Feature`
//!   ([`V2GeofencePatch`]) at the area root (base) and inside each mode block.
//! - **Conversions** ([`mapping`]): Koji `SingleVec` route → `[{lat,lon}]`,
//!   `Feature` fence → geofence patch, and per-[`AreaMode`] PATCH builders.
//!
//! Mutations target an area by its stored `dragonite_area_id` (architecture §9);
//! the [`Tri`] wrapper gives PATCH the absent/null/value semantics (§7) so a
//! patch carries only the fields it intends to change.

pub mod client;
pub mod envelope;
pub mod error;
pub mod mapping;
pub mod patch;
pub mod types;

pub use client::DragoniteClient;
pub use envelope::{parse_v2, parse_v2_with_meta, V2ApiError, V2Envelope, V2Meta};
pub use error::DragoniteError;
pub use mapping::{
    area_geofence_patch, area_route_patch, feature_to_geofence, route_to_api_locations,
};
pub use patch::Tri;
pub use types::{
    ApiArea, ApiAreaFortMode, ApiAreaPokemonMode, ApiAreaQuestMode, ApiAreaRarePokemonMode,
    ApiLocation, AreaMode, V2GeofencePatch,
};
