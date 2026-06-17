//! Thin bridge between the algorithm dispatch and the `koji-plugins` registry.
//!
//! The `Custom(name)` arms of the clustering / routing / bootstrap pipelines
//! resolve their plugin through a [`PluginRegistry`] loaded from
//! `KOJI_PLUGINS_DIR` (default `./plugins`). The registry is built on demand —
//! only when a `Custom` mode is actually requested or `all_*_options()` is asked
//! for the available modes — so the filesystem is not scanned on the hot path of
//! the built-in modes.

use koji_core::SingleVec;
use koji_plugins::{Plugin, PluginKind};
use serde_json::{Value, json};

/// The names of all registered (enabled) plugins of a given kind (for
/// `all_*_options()`). Sourced from the installable process-global registry
/// ([`koji_plugins::current`]), which `koji-service` rebuilds from disk ∪ the DB
/// overlay; plugins disabled via the overlay are excluded.
pub(crate) fn plugin_names(kind: PluginKind) -> Vec<String> {
    koji_plugins::current().names(kind)
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

/// Merge the overlay default args (from the DB `plugin_config` table, surfaced
/// via the registry) with the per-request args; **request keys win**. A `null`
/// request falls back to the defaults wholesale; if either operand is a
/// non-object the request wins (so a non-object request is never silently
/// dropped, and a non-object default never shadows a request).
pub(crate) fn merge_args(default: Option<&Value>, request: Value) -> Value {
    match (default, &request) {
        (Some(Value::Object(d)), Value::Object(r)) => {
            let mut merged = d.clone();
            for (k, v) in r {
                merged.insert(k.clone(), v.clone());
            }
            Value::Object(merged)
        }
        (Some(d), Value::Null) => d.clone(),
        _ => request,
    }
}

/// Build the final `args` for a `Custom(name)` dispatch: overlay the registry's
/// default args for `(kind, name)` under the legacy request args. Centralizes
/// the [`merge_args`] + [`args_to_value`] dance so every `Custom` call site (the
/// clustering / routing / bootstrap pipelines) merges identically.
pub(crate) fn merged_args(kind: PluginKind, name: &str, plugin_args: &str) -> Value {
    merge_args(
        koji_plugins::current().args_default(kind, name),
        args_to_value(plugin_args),
    )
}

/// Resolve a `Custom(name)` plugin from the registry and build a runnable
/// [`Plugin`]. Returns `None` (after logging) when the name is unknown or the
/// plugin's entrypoint is missing — callers fall back to a safe default rather
/// than panicking.
pub(crate) fn resolve(kind: PluginKind, name: &str) -> Option<Plugin> {
    let registry = koji_plugins::current();
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
    match Plugin::from_manifest(manifest, dir) {
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
    let plugin = resolve(kind, name)?;
    match plugin.run(points, &merged_args(kind, name, plugin_args)) {
        Ok(points) => Some(points),
        Err(e) => {
            log::error!("[PLUGINS] error running {kind} plugin `{name}`: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_args_request_overrides_default() {
        let default = serde_json::json!({"k": 8, "iters": 50});
        let request = serde_json::json!({"k": 12});
        assert_eq!(
            merge_args(Some(&default), request),
            serde_json::json!({"k": 12, "iters": 50})
        );
    }

    #[test]
    fn merge_args_null_request_falls_back_to_default() {
        let default = serde_json::json!({"k": 8});
        assert_eq!(
            merge_args(Some(&default), serde_json::Value::Null),
            serde_json::json!({"k": 8})
        );
    }

    #[test]
    fn merge_args_no_default_returns_request() {
        let request = serde_json::json!({"k": 12});
        assert_eq!(merge_args(None, request.clone()), request);
    }
}
