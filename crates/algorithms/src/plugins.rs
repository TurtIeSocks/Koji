//! Thin bridge between the algorithm dispatch and the `koji-plugins` registry.
//!
//! The `Custom(name)` arms of the clustering / routing / bootstrap pipelines
//! resolve their plugin through a [`PluginRegistry`] loaded from
//! `KOJI_PLUGINS_DIR` (default `./plugins`). The registry is built on demand —
//! only when a `Custom` mode is actually requested or `all_*_options()` is asked
//! for the available modes — so the filesystem is not scanned on the hot path of
//! the built-in modes.

use std::sync::LazyLock;

use koji_core::SingleVec;
use koji_plugins::{Plugin, PluginKind, PluginRegistry};
use serde_json::{Value, json};

/// Process-wide plugin registry, scanned from `KOJI_PLUGINS_DIR` (default
/// `./plugins`) on first use and cached for the life of the process.
///
/// Tradeoff: the filesystem is scanned exactly once. Plugins added *after* the
/// first plugin use (the first `Custom` mode dispatch or `all_*_options()` call)
/// are not picked up — acceptable because plugins are deploy-time artifacts, not
/// hot-reloaded. Previously the registry was rebuilt on every call, so a single
/// route calc re-scanned the directory ~3×.
///
/// `KOJI_PLUGINS_DIR` is read inside [`PluginRegistry::from_env`], so it must be
/// set before the first plugin use (it is, at server startup).
static REGISTRY: LazyLock<PluginRegistry> = LazyLock::new(PluginRegistry::from_env);

/// The process-wide cached plugin registry (see [`REGISTRY`]).
pub(crate) fn registry() -> &'static PluginRegistry {
    &REGISTRY
}

/// The names of all registered plugins of a given kind (for `all_*_options()`).
pub(crate) fn plugin_names(kind: PluginKind) -> Vec<String> {
    registry().names(kind)
}

/// Wrap the legacy free-form `plugin_args` string into the JSON `args` object of
/// the stdio protocol. Empty input maps to JSON `null`; otherwise it travels as
/// `{"raw": "<plugin_args>"}` so plugins receive it verbatim.
pub(crate) fn args_to_value(plugin_args: &str) -> Value {
    if plugin_args.trim().is_empty() {
        Value::Null
    } else {
        json!({ "raw": plugin_args })
    }
}

/// Resolve a `Custom(name)` plugin from the registry and build a runnable
/// [`Plugin`]. Returns `None` (after logging) when the name is unknown or the
/// plugin's entrypoint is missing — callers fall back to a safe default rather
/// than panicking.
pub(crate) fn resolve(kind: PluginKind, name: &str, split_level: u64) -> Option<Plugin> {
    let registry = registry();
    let manifest = match registry.get(kind, name) {
        Some(manifest) => manifest,
        None => {
            log::error!(
                "[PLUGINS] no {kind} plugin named `{name}` (set KOJI_PLUGINS_DIR and add a {}/<dir>/plugin.toml)",
                koji_plugins::DEFAULT_PLUGINS_DIR
            );
            return None;
        }
    };
    let dir = registry.dir(kind, name)?;
    match Plugin::from_manifest(manifest, dir, split_level) {
        Ok(plugin) => Some(plugin),
        Err(e) => {
            log::error!("[PLUGINS] failed to load {kind} plugin `{name}`: {e}");
            None
        }
    }
}

/// Resolve, run (single-shot), and return the plugin's output points, or `None`
/// on any failure (unknown plugin, spawn error, bad output) — all logged.
pub(crate) fn run_once(
    kind: PluginKind,
    name: &str,
    points: SingleVec,
    plugin_args: &str,
) -> Option<SingleVec> {
    let plugin = resolve(kind, name, 0)?;
    match plugin.run(points, &args_to_value(plugin_args)) {
        Ok(points) => Some(points),
        Err(e) => {
            log::error!("[PLUGINS] error running {kind} plugin `{name}`: {e}");
            None
        }
    }
}
