//! The external-process plugin runner.
//!
//! A [`Plugin`] is built from a [`PluginManifest`] plus the directory it was
//! loaded from. Running it spawns the manifest's interpreter + entrypoint as a
//! child process, writes a [`PluginInput`] as JSON to its stdin, and parses a
//! [`PluginOutput`] JSON object from its stdout. A non-zero exit status or
//! unparseable stdout is an error.
//!
//! [`Plugin::run_multi`] preserves the original parallel-over-S2-cells fan-out:
//! when `split_level > 0` the points are bucketed by [`create_cell_map`] and
//! each bucket is run as an independent child process in parallel via rayon.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use koji_core::SingleVec;
use koji_core::create_cell_map;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::manifest::PluginManifest;
use crate::protocol::{PluginInput, PluginOutput};

/// Joins the per-cell outputs of a parallel [`Plugin::run_multi`] run back into
/// a single point list. Receives the plugin (for `split_level` and re-runs) and
/// the per-cell results.
pub type JoinFunction = fn(&Plugin, Vec<SingleVec>) -> SingleVec;

/// A ready-to-run external plugin.
#[derive(Debug, Clone)]
pub struct Plugin {
    /// The command to invoke (interpreter, or the entrypoint itself when it is
    /// directly executable).
    interpreter: String,
    /// Absolute path to the entrypoint script/binary, when an interpreter runs
    /// it. `None` when `interpreter` *is* the entrypoint.
    entrypoint_path: Option<PathBuf>,
    /// The plugin name (for logging).
    pub name: String,
    /// S2 split level for the parallel fan-out in [`Plugin::run_multi`].
    pub split_level: u64,
}

impl Plugin {
    /// Build a runnable plugin from its manifest and the directory it lives in.
    ///
    /// The entrypoint is resolved relative to `plugin_dir`. The interpreter is
    /// the manifest's explicit value or one inferred from the entrypoint
    /// extension; when neither yields one, the entrypoint is invoked directly.
    /// Errors if the resolved entrypoint does not exist.
    pub fn from_manifest(
        manifest: &PluginManifest,
        plugin_dir: &std::path::Path,
        split_level: u64,
    ) -> io::Result<Self> {
        let entrypoint_path = plugin_dir.join(&manifest.entrypoint);
        if !entrypoint_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "plugin `{}` entrypoint {} does not exist",
                    manifest.name,
                    entrypoint_path.display()
                ),
            ));
        }

        let (interpreter, entrypoint_path) = match manifest.resolved_interpreter() {
            Some(interpreter) => (interpreter, Some(entrypoint_path)),
            // No interpreter: run the entrypoint directly.
            None => (entrypoint_path.display().to_string(), None),
        };

        Ok(Plugin {
            interpreter,
            entrypoint_path,
            name: manifest.name.clone(),
            split_level,
        })
    }

    /// Run the plugin over `points`, fanning out across S2 cells when
    /// `split_level > 0`.
    ///
    /// With `split_level == 0` the full point set is sent to a single child.
    /// Otherwise points are bucketed by [`create_cell_map`] and each bucket runs
    /// as an independent child process in parallel; failed buckets are dropped
    /// (logged inside [`Plugin::run`]). The same `args` are forwarded to every
    /// child. With a `joiner` the per-cell results are combined by it; without
    /// one they are flattened in arbitrary order.
    pub fn run_multi<T>(
        &self,
        points: &SingleVec,
        args: &serde_json::Value,
        joiner: Option<T>,
    ) -> io::Result<SingleVec>
    where
        T: Fn(&Self, Vec<SingleVec>) -> SingleVec,
    {
        let handlers = if self.split_level == 0 {
            vec![self.run(points.clone(), args)?]
        } else {
            create_cell_map(points, self.split_level)
                .into_values()
                .collect::<Vec<SingleVec>>()
                .into_par_iter()
                .filter_map(|cell_points| self.run(cell_points, args).ok())
                .collect()
        };

        if let Some(joiner) = joiner {
            Ok(joiner(self, handlers))
        } else {
            Ok(handlers.into_iter().flatten().collect())
        }
    }

    /// Run the plugin once over `points` with `args`, via the JSON stdio
    /// protocol. Returns the plugin's output points, or an error on spawn
    /// failure, non-zero exit, or unparseable stdout.
    pub fn run(&self, points: SingleVec, args: &serde_json::Value) -> io::Result<SingleVec> {
        log::info!("spawning {} child process", self.name);
        let time = Instant::now();

        let payload = serde_json::to_vec(&PluginInput::with_args(points, args.clone()))
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut command = Command::new(&self.interpreter);
        if let Some(entrypoint) = &self.entrypoint_path {
            command.arg(entrypoint);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Failed to open stdin"))?;
        std::thread::spawn(move || {
            if let Err(err) = stdin.write_all(&payload).and_then(|_| stdin.flush()) {
                log::error!("failed to write plugin stdin: {}", err);
            }
        });

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Could not capture stdout"))?;
        let mut raw = String::new();
        stdout.read_to_string(&mut raw)?;

        match child.wait()? {
            status if status.success() => {}
            status => {
                return Err(io::Error::other(format!(
                    "child process exited with status: {}",
                    status
                )));
            }
        }

        let output: PluginOutput = serde_json::from_str(raw.trim()).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "plugin `{}` returned unparseable output ({}); expected JSON {{\"points\": [[lat,lon], …]}}",
                    self.name, e
                ),
            )
        })?;

        log::info!(
            "{} child process finished in {}s with {} points",
            self.name,
            time.elapsed().as_secs_f32(),
            output.points.len()
        );
        Ok(output.points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::PluginKind;
    use std::fs;

    fn manifest(entrypoint: &str) -> PluginManifest {
        PluginManifest {
            name: "echo".into(),
            kind: PluginKind::Clustering,
            entrypoint: entrypoint.into(),
            interpreter: Some("bash".into()),
            version: None,
            description: None,
        }
    }

    #[test]
    fn from_manifest_errors_on_missing_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Plugin::from_manifest(&manifest("nope.sh"), tmp.path(), 0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn run_echoes_points_through_a_bash_plugin() {
        // A trivial "plugin" that reads PluginInput on stdin and emits the same
        // points back as PluginOutput. Exercises the real JSON stdio protocol.
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("echo.sh");
        fs::write(
            &script,
            "#!/usr/bin/env bash\nin=$(cat)\npoints=$(printf '%s' \"$in\" | sed -E 's/.*\"points\"[[:space:]]*:[[:space:]]*(\\[.*\\])[^]]*$/\\1/')\nprintf '{\"points\": %s}' \"$points\"\n",
        )
        .unwrap();

        let plugin = Plugin::from_manifest(&manifest("echo.sh"), tmp.path(), 0).unwrap();
        let points: SingleVec = vec![[1.5, 2.5], [3.5, 4.5]];
        let out = plugin
            .run(points.clone(), &serde_json::Value::Null)
            .unwrap();
        assert_eq!(out, points);
    }

    #[test]
    fn run_errors_on_nonzero_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("fail.sh");
        fs::write(&script, "#!/usr/bin/env bash\nexit 3\n").unwrap();
        let plugin = Plugin::from_manifest(&manifest("fail.sh"), tmp.path(), 0).unwrap();
        let err = plugin
            .run(vec![[0.0, 0.0]], &serde_json::Value::Null)
            .unwrap_err();
        assert!(err.to_string().contains("exited with status"));
    }

    #[test]
    fn run_errors_on_unparseable_output() {
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("garbage.sh");
        fs::write(
            &script,
            "#!/usr/bin/env bash\ncat >/dev/null\necho not-json\n",
        )
        .unwrap();
        let plugin = Plugin::from_manifest(&manifest("garbage.sh"), tmp.path(), 0).unwrap();
        let err = plugin
            .run(vec![[0.0, 0.0]], &serde_json::Value::Null)
            .unwrap_err();
        assert!(err.to_string().contains("unparseable"));
    }
}
