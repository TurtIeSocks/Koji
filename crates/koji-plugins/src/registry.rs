//! Plugin discovery + lookup.
//!
//! A [`PluginRegistry`] scans a plugins directory once: each immediate
//! subdirectory is expected to contain a `plugin.toml`. Parsed manifests are
//! indexed by `(PluginKind, name)`. The directory location is conventionally
//! supplied via the `KOJI_PLUGINS_DIR` env var (default `./plugins`); see
//! [`PluginRegistry::from_env`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::manifest::{PluginKind, PluginManifest};

/// Env var naming the directory plugins are discovered from.
pub const PLUGINS_DIR_ENV: &str = "KOJI_PLUGINS_DIR";
/// Default plugins directory when [`PLUGINS_DIR_ENV`] is unset.
pub const DEFAULT_PLUGINS_DIR: &str = "./plugins";
/// Manifest file name expected inside each plugin directory.
pub const MANIFEST_FILE: &str = "plugin.toml";

/// An in-memory index of the plugins found under one directory.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    /// The plugin directory each manifest was loaded from, by `(kind, name)`.
    dirs: HashMap<(PluginKind, String), PathBuf>,
    /// Parsed manifests by `(kind, name)`.
    manifests: HashMap<(PluginKind, String), PluginManifest>,
}

impl PluginRegistry {
    /// Scan `dir` for plugins. Each immediate subdirectory containing a
    /// `plugin.toml` contributes one entry. A missing `dir` yields an empty
    /// registry (no plugins is a valid state — the built-in modes still work).
    /// Subdirectories with a missing/unparseable manifest are skipped with a
    /// warning rather than failing the whole scan.
    pub fn load(dir: &Path) -> Self {
        let mut registry = PluginRegistry::default();

        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                log::debug!(
                    "[PLUGINS] no plugins loaded from {}: {}",
                    dir.display(),
                    e
                );
                return registry;
            }
        };

        for entry in entries.flatten() {
            let plugin_dir = entry.path();
            if !plugin_dir.is_dir() {
                continue;
            }
            let manifest_path = plugin_dir.join(MANIFEST_FILE);
            if !manifest_path.exists() {
                continue;
            }
            match Self::read_manifest(&manifest_path) {
                Ok(manifest) => {
                    let key = (manifest.kind, manifest.name.clone());
                    if registry.manifests.contains_key(&key) {
                        log::warn!(
                            "[PLUGINS] duplicate plugin {} {} at {}; keeping the first",
                            manifest.kind,
                            manifest.name,
                            plugin_dir.display()
                        );
                        continue;
                    }
                    log::info!(
                        "[PLUGINS] loaded {} plugin `{}` from {}",
                        manifest.kind,
                        manifest.name,
                        plugin_dir.display()
                    );
                    registry.dirs.insert(key.clone(), plugin_dir);
                    registry.manifests.insert(key, manifest);
                }
                Err(e) => {
                    log::warn!(
                        "[PLUGINS] skipping {}: {}",
                        manifest_path.display(),
                        e
                    );
                }
            }
        }

        registry
    }

    /// Convenience: load from [`PLUGINS_DIR_ENV`], falling back to
    /// [`DEFAULT_PLUGINS_DIR`].
    pub fn from_env() -> Self {
        let dir =
            std::env::var(PLUGINS_DIR_ENV).unwrap_or_else(|_| DEFAULT_PLUGINS_DIR.to_string());
        PluginRegistry::load(Path::new(&dir))
    }

    fn read_manifest(path: &Path) -> Result<PluginManifest, String> {
        let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str::<PluginManifest>(&raw).map_err(|e| e.to_string())
    }

    /// Look up a plugin manifest by kind + name.
    pub fn get(&self, kind: PluginKind, name: &str) -> Option<&PluginManifest> {
        self.manifests.get(&(kind, name.to_string()))
    }

    /// The directory a plugin's manifest was loaded from (for resolving its
    /// entrypoint).
    pub fn dir(&self, kind: PluginKind, name: &str) -> Option<&Path> {
        self.dirs.get(&(kind, name.to_string())).map(|p| p.as_path())
    }

    /// All plugin names registered under `kind`.
    pub fn names(&self, kind: PluginKind) -> Vec<String> {
        self.manifests
            .keys()
            .filter(|(k, _)| *k == kind)
            .map(|(_, name)| name.clone())
            .collect()
    }

    /// Total number of plugins indexed.
    pub fn len(&self) -> usize {
        self.manifests.len()
    }

    /// Whether the registry holds no plugins.
    pub fn is_empty(&self) -> bool {
        self.manifests.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_plugin(root: &Path, dir: &str, manifest: &str) {
        let plugin_dir = root.join(dir);
        fs::create_dir_all(&plugin_dir).unwrap();
        fs::write(plugin_dir.join(MANIFEST_FILE), manifest).unwrap();
    }

    #[test]
    fn missing_dir_is_empty_registry() {
        let registry = PluginRegistry::load(Path::new("/nonexistent/koji/plugins/xyz"));
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.names(PluginKind::Clustering).is_empty());
    }

    #[test]
    fn indexes_plugins_by_kind_and_name() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(
            tmp.path(),
            "clu",
            "name = \"my_clusterer\"\nkind = \"clustering\"\nentrypoint = \"main.py\"\n",
        );
        write_plugin(
            tmp.path(),
            "rou",
            "name = \"my_router\"\nkind = \"routing\"\nentrypoint = \"run.js\"\n",
        );

        let registry = PluginRegistry::load(tmp.path());
        assert_eq!(registry.len(), 2);

        let clusterer = registry.get(PluginKind::Clustering, "my_clusterer").unwrap();
        assert_eq!(clusterer.entrypoint, "main.py");
        assert!(registry.get(PluginKind::Routing, "my_clusterer").is_none());

        assert_eq!(
            registry.names(PluginKind::Clustering),
            vec!["my_clusterer".to_string()]
        );
        assert_eq!(
            registry.names(PluginKind::Routing),
            vec!["my_router".to_string()]
        );
        assert!(registry.names(PluginKind::Bootstrap).is_empty());
    }

    #[test]
    fn dir_points_at_the_plugin_directory() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(
            tmp.path(),
            "clu",
            "name = \"c\"\nkind = \"clustering\"\nentrypoint = \"main.py\"\n",
        );
        let registry = PluginRegistry::load(tmp.path());
        let dir = registry.dir(PluginKind::Clustering, "c").unwrap();
        assert!(dir.ends_with("clu"));
        assert!(dir.join("main.py").parent().unwrap().exists());
    }

    #[test]
    fn subdir_without_manifest_is_ignored() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("not_a_plugin")).unwrap();
        write_plugin(
            tmp.path(),
            "clu",
            "name = \"c\"\nkind = \"clustering\"\nentrypoint = \"main.py\"\n",
        );
        let registry = PluginRegistry::load(tmp.path());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn unparseable_manifest_is_skipped_not_fatal() {
        let tmp = tempfile::tempdir().unwrap();
        write_plugin(tmp.path(), "bad", "this = is = not = toml\n");
        write_plugin(
            tmp.path(),
            "good",
            "name = \"g\"\nkind = \"bootstrap\"\nentrypoint = \"go.sh\"\n",
        );
        let registry = PluginRegistry::load(tmp.path());
        assert_eq!(registry.len(), 1);
        assert!(registry.get(PluginKind::Bootstrap, "g").is_some());
    }
}
