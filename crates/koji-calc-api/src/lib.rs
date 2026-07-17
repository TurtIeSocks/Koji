//! The shared calc API surface: request types + pure compute cores.
//!
//! Holds the inbound geojson/data-point input enums ([`inputs`]), the shared
//! resolution helpers + default constants ([`resolve`]), the `Option`-field
//! "arg-group" wire structs ([`groups`]) that `resolve()` into the pure-domain
//! `koji-core` config structs, the per-op request types ([`ops`]) dispatched by
//! the serde-tagged [`CalcRequest`], and the pure compute cores ([`compute`])
//! those requests resolve into.
//!
//! The arg-groups are camelCase, `#[serde(default)]` wire DTOs; their `resolve()`
//! bodies replicate the legacy fat-`Args` `init()` defaults exactly (the parity
//! contract lives in `docs/superpowers/specs/2026-06-14-args-restructure-design.md`
//! §2). koji-core configs stay serde-free domain types.
//!
//! Extracted from `koji-service` so the server and a wasm demo-mode consumer
//! share identical calc request semantics. Feature gates:
//! - `schema` (default): utoipa `ToSchema` derives for the server's OpenAPI doc.
//! - `native` (default): forwards `algorithms/native`; gates [`compute::run_bootstrap`]
//!   (the bootstrap algorithm is native-only). wasm consumers turn both off with
//!   `default-features = false`.

pub mod compute;
pub mod config;
pub mod groups;
pub mod inputs;
pub mod ops;
pub mod resolve;

pub use config::{
    DataFilter, DevConfig, OutputConfig, ReturnTypeArg, get_return_type, negotiate_return_type,
};
pub use groups::*;
pub use inputs::*;
pub use ops::*;
