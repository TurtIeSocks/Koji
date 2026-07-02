//! Plugin manifest types — the `plugin.toml` schema.

use std::fmt::Display;

use serde::Deserialize;

/// The category a plugin slots into. Mirrors the three built-in algorithm
/// families. Replaces the old `Folder` enum that keyed off in-tree directory
/// names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Clustering,
    Routing,
    Bootstrap,
}

impl Display for PluginKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginKind::Clustering => write!(f, "clustering"),
            PluginKind::Routing => write!(f, "routing"),
            PluginKind::Bootstrap => write!(f, "bootstrap"),
        }
    }
}

/// A single plugin's `plugin.toml`, one per plugin directory.
///
/// ```toml
/// name = "my_clusterer"
/// kind = "clustering"        # clustering | routing | bootstrap
/// entrypoint = "main.py"     # relative to the plugin dir
/// interpreter = "python3"    # optional; inferred from the entrypoint extension
/// version = "0.1.0"          # optional
/// description = "…"          # optional
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    /// The mode name users select (e.g. the `Custom(name)` string).
    pub name: String,
    /// Which algorithm family this plugin serves.
    pub kind: PluginKind,
    /// Executable/script to run, relative to the plugin directory.
    pub entrypoint: String,
    /// Interpreter to invoke (e.g. `python3`, `node`). When omitted it is
    /// inferred from the entrypoint extension; a bare executable runs directly.
    #[serde(default)]
    pub interpreter: Option<String>,
    /// Optional semantic version, surfaced to clients.
    #[serde(default)]
    pub version: Option<String>,
    /// Optional human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// The stdio protocol the plugin speaks (`json` default, or `latlng` for
    /// external plugins that speak the legacy whitespace `lat,lng` protocol).
    #[serde(default)]
    pub protocol: crate::protocol::PluginProtocol,
}

impl PluginManifest {
    /// Resolve the interpreter to invoke: the explicit `interpreter` field if
    /// present, otherwise inferred from the entrypoint's file extension. Returns
    /// `None` for an unrecognized extension with no explicit interpreter, in
    /// which case the entrypoint is expected to be directly executable.
    pub fn resolved_interpreter(&self) -> Option<String> {
        if let Some(interpreter) = &self.interpreter {
            return Some(interpreter.clone());
        }
        match self.entrypoint.rsplit('.').next() {
            Some("py") => Some("python3".to_string()),
            Some("js") => Some("node".to_string()),
            Some("sh") => Some("bash".to_string()),
            Some("ts") => Some("ts-node".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_manifest() {
        let toml = r#"
            name = "my_clusterer"
            kind = "clustering"
            entrypoint = "main.py"
            interpreter = "python3"
            version = "0.1.0"
            description = "demo"
        "#;
        let m: PluginManifest = toml::from_str(toml).unwrap();
        assert_eq!(m.name, "my_clusterer");
        assert_eq!(m.kind, PluginKind::Clustering);
        assert_eq!(m.entrypoint, "main.py");
        assert_eq!(m.interpreter.as_deref(), Some("python3"));
        assert_eq!(m.version.as_deref(), Some("0.1.0"));
        assert_eq!(m.description.as_deref(), Some("demo"));
    }

    #[test]
    fn parses_minimal_manifest_with_optionals_absent() {
        let toml = r#"
            name = "router"
            kind = "routing"
            entrypoint = "run.js"
        "#;
        let m: PluginManifest = toml::from_str(toml).unwrap();
        assert_eq!(m.kind, PluginKind::Routing);
        assert!(m.interpreter.is_none());
        assert!(m.version.is_none());
        assert!(m.description.is_none());
    }

    #[test]
    fn kind_is_lowercase() {
        for (s, k) in [
            ("clustering", PluginKind::Clustering),
            ("routing", PluginKind::Routing),
            ("bootstrap", PluginKind::Bootstrap),
        ] {
            let toml = format!("name = \"x\"\nkind = \"{s}\"\nentrypoint = \"e\"\n");
            let m: PluginManifest = toml::from_str(&toml).unwrap();
            assert_eq!(m.kind, k);
        }
    }

    #[test]
    fn rejects_unknown_kind() {
        let toml = "name = \"x\"\nkind = \"teleport\"\nentrypoint = \"e\"\n";
        assert!(toml::from_str::<PluginManifest>(toml).is_err());
    }

    #[test]
    fn interpreter_inferred_from_extension() {
        let make = |entry: &str| PluginManifest {
            name: "x".into(),
            kind: PluginKind::Clustering,
            entrypoint: entry.into(),
            interpreter: None,
            version: None,
            description: None,
            protocol: Default::default(),
        };
        assert_eq!(
            make("a.py").resolved_interpreter().as_deref(),
            Some("python3")
        );
        assert_eq!(make("a.js").resolved_interpreter().as_deref(), Some("node"));
        assert_eq!(make("a.sh").resolved_interpreter().as_deref(), Some("bash"));
        assert_eq!(
            make("a.ts").resolved_interpreter().as_deref(),
            Some("ts-node")
        );
        assert!(make("a.bin").resolved_interpreter().is_none());
    }

    #[test]
    fn explicit_interpreter_overrides_inference() {
        let m = PluginManifest {
            name: "x".into(),
            kind: PluginKind::Clustering,
            entrypoint: "a.py".into(),
            interpreter: Some("pypy3".into()),
            version: None,
            description: None,
            protocol: Default::default(),
        };
        assert_eq!(m.resolved_interpreter().as_deref(), Some("pypy3"));
    }
}
