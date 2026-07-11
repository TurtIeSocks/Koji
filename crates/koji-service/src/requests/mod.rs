//! HTTP wire layer for the v2 request surface.
//!
//! Holds the inbound geojson/data-point input enums ([`inputs`]), the shared
//! resolution helpers + default constants ([`resolve`], crate-private), and the
//! `Option`-field "arg-group" wire structs ([`groups`]) that `resolve()` into
//! the pure-domain `koji-core` config structs.
//!
//! The arg-groups are camelCase, `#[serde(default)]` wire DTOs; their `resolve()`
//! bodies replicate the legacy fat-`Args` `init()` defaults exactly (the parity
//! contract lives in `docs/superpowers/specs/2026-06-14-args-restructure-design.md`
//! §2). koji-core configs stay serde-free domain types.

mod config;
mod groups;
mod inputs;
mod ops;
pub(crate) mod resolve;

pub use config::{
    DataFilter, DevConfig, OutputConfig, ReturnTypeArg, get_return_type, negotiate_return_type,
};
pub use groups::*;
pub use inputs::*;
pub use ops::*;
