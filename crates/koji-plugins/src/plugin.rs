//! The external-process plugin runner.
//!
//! A [`Plugin`] is built from a [`PluginManifest`] plus the directory it was
//! loaded from. Running it spawns the manifest's interpreter + entrypoint as a
//! child process, writes a [`PluginInput`] as JSON to its stdin, and parses a
//! [`PluginOutput`] JSON object from its stdout. A non-zero exit status or
//! unparseable stdout is an error. The child's stderr is captured and, when
//! non-empty, logged at warn level rather than inheriting Koji's own stderr.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

use koji_core::SingleVec;

use crate::manifest::PluginManifest;
use crate::protocol::{PluginInput, PluginOutput, PluginProtocol, decode_latlng, encode_latlng};

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
    /// The stdio encoding this plugin speaks.
    protocol: PluginProtocol,
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
            protocol: manifest.protocol,
        })
    }

    /// Run the plugin once over `points` with `args`, via the JSON stdio
    /// protocol. Returns the plugin's output points, or an error on spawn
    /// failure, non-zero exit, or unparseable stdout.
    pub fn run(&self, points: SingleVec, args: &serde_json::Value) -> io::Result<SingleVec> {
        log::info!("spawning {} child process", self.name);
        let time = Instant::now();

        let mut command = Command::new(&self.interpreter);
        if let Some(entrypoint) = &self.entrypoint_path {
            command.arg(entrypoint);
        }

        // Protocol-specific stdin payload (+ extra argv for the legacy protocol).
        let payload: Vec<u8> = match self.protocol {
            PluginProtocol::Json => {
                serde_json::to_vec(&PluginInput::with_args(points, args.clone()))
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
            }
            PluginProtocol::Latlng => {
                // Legacy plugins take `--flag value` argv from the `args.raw`
                // string (the old free-form `plugin_args`).
                if let Some(raw) = args.get("raw").and_then(|v| v.as_str()) {
                    command.args(raw.split_whitespace());
                }
                encode_latlng(&points).into_bytes()
            }
        };

        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
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

        // Drain stderr on its own thread so a chatty plugin can't deadlock by
        // filling the stderr pipe while we block reading stdout. Joined before
        // `child.wait()`.
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("Could not capture stderr"))?;
        let stderr_handle = std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = stderr.read_to_string(&mut buf);
            buf
        });

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("Could not capture stdout"))?;
        let mut raw = String::new();
        stdout.read_to_string(&mut raw)?;

        // Collect the child's stderr before waiting on it. A panicked drain
        // thread (poisoned) just yields no captured stderr.
        let captured_stderr = stderr_handle.join().unwrap_or_default();
        if !captured_stderr.trim().is_empty() {
            log::warn!("[plugin {}] stderr: {}", self.name, captured_stderr.trim());
        }

        match child.wait()? {
            status if status.success() => {}
            status => {
                return Err(io::Error::other(format!(
                    "child process exited with status: {}",
                    status
                )));
            }
        }

        let points = match self.protocol {
            PluginProtocol::Json => {
                let output: PluginOutput = serde_json::from_str(raw.trim()).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "plugin `{}` returned unparseable output ({}); expected JSON {{\"points\": [[lat,lon], …]}}",
                            self.name, e
                        ),
                    )
                })?;
                output.points
            }
            PluginProtocol::Latlng => decode_latlng(&raw).map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("plugin `{}` returned unparseable output ({e})", self.name),
                )
            })?,
        };

        log::info!(
            "{} child process finished in {}s with {} points",
            self.name,
            time.elapsed().as_secs_f32(),
            points.len()
        );
        Ok(points)
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
            protocol: Default::default(),
        }
    }

    #[test]
    fn from_manifest_errors_on_missing_entrypoint() {
        let tmp = tempfile::tempdir().unwrap();
        let err = Plugin::from_manifest(&manifest("nope.sh"), tmp.path()).unwrap_err();
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

        let plugin = Plugin::from_manifest(&manifest("echo.sh"), tmp.path()).unwrap();
        let points: SingleVec = vec![[1.5, 2.5], [3.5, 4.5]];
        let out = plugin
            .run(points.clone(), &serde_json::Value::Null)
            .unwrap();
        assert_eq!(out, points);
    }

    #[test]
    fn run_latlng_protocol_round_trips() {
        // The legacy `tsp` protocol: whitespace-separated `lat,lng` in, same out.
        // This bash "plugin" reverses the token order to prove I/O is wired.
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("rev.sh");
        fs::write(
            &script,
            "#!/usr/bin/env bash\nread -a toks\nfor ((i=${#toks[@]}-1;i>=0;i--)); do echo \"${toks[i]}\"; done\n",
        )
        .unwrap();
        let m = PluginManifest {
            name: "rev".into(),
            kind: PluginKind::Routing,
            entrypoint: "rev.sh".into(),
            interpreter: Some("bash".into()),
            version: None,
            description: None,
            protocol: PluginProtocol::Latlng,
        };
        let plugin = Plugin::from_manifest(&m, tmp.path()).unwrap();
        let out = plugin
            .run(vec![[1.0, 2.0], [3.0, 4.0]], &serde_json::Value::Null)
            .unwrap();
        assert_eq!(out, vec![[3.0, 4.0], [1.0, 2.0]]);
    }

    #[test]
    fn run_errors_on_nonzero_exit() {
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("fail.sh");
        fs::write(&script, "#!/usr/bin/env bash\nexit 3\n").unwrap();
        let plugin = Plugin::from_manifest(&manifest("fail.sh"), tmp.path()).unwrap();
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
        let plugin = Plugin::from_manifest(&manifest("garbage.sh"), tmp.path()).unwrap();
        let err = plugin
            .run(vec![[0.0, 0.0]], &serde_json::Value::Null)
            .unwrap_err();
        assert!(err.to_string().contains("unparseable"));
    }
}
