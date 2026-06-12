//! koji-plugins — the external-process algorithm plugin system.
//!
//! Koji's clustering / routing / bootstrap pipelines can dispatch a `Custom`
//! mode to an external program. This crate formalizes that mechanism:
//!
//! - **Manifest** ([`PluginManifest`] + [`PluginKind`]): each plugin lives in
//!   its own directory with a `plugin.toml` describing its name, kind,
//!   entrypoint, and (optionally) interpreter/version/description.
//! - **Protocol** ([`PluginInput`] / [`PluginOutput`]): Koji speaks a JSON
//!   stdio protocol to the plugin — `{points, args}` in, `{points}` out.
//! - **Registry** ([`PluginRegistry`]): discovers plugins under a directory
//!   (conventionally `KOJI_PLUGINS_DIR`, default `./plugins`) and indexes them
//!   by `(kind, name)`.
//! - **Runner** ([`Plugin`]): spawns a plugin as a child process, with an
//!   optional parallel-over-S2-cells fan-out ([`Plugin::run_multi`]).
//!
//! The crate depends only on `koji-core` (for `SingleVec` + `create_cell_map`),
//! keeping the workspace graph acyclic: `koji-core ← koji-plugins ← algorithms`.

pub mod global;
mod manifest;
mod plugin;
mod protocol;
mod registry;

pub use global::{current, install};
pub use manifest::{PluginKind, PluginManifest};
pub use plugin::{JoinFunction, Plugin};
pub use protocol::{PluginInput, PluginOutput};
pub use registry::{DEFAULT_PLUGINS_DIR, MANIFEST_FILE, Overlay, PLUGINS_DIR_ENV, PluginRegistry};
