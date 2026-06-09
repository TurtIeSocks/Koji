//! The JSON stdio protocol (v1) spoken between Koji and an external plugin.
//!
//! Koji writes a [`PluginInput`] as a single JSON object to the plugin's stdin
//! and reads a [`PluginOutput`] JSON object from its stdout. A non-zero exit
//! status or unparseable stdout is treated as an error by the runner. The input
//! carries an explicit `protocol_version` so a plugin can detect the contract it
//! is being spoken to:
//!
//! ```json
//! {"protocol_version": 1, "points": [[lat, lon], …], "args": { … }}
//! ```
//!
//! `SingleVec` is `Vec<[f64; 2]>` in `[lat, lon]` order, so the JSON point
//! arrays map onto it directly.

use koji_core::SingleVec;
use serde::{Deserialize, Serialize};

/// The version of the JSON plugin protocol Koji currently speaks. Emitted as
/// [`PluginInput::protocol_version`] so plugins can detect the contract.
pub const PROTOCOL_VERSION: u32 = 1;

/// Default for [`PluginInput::protocol_version`] when older inbound JSON omits
/// the field — such payloads predate versioning and are therefore v1.
fn default_protocol_version() -> u32 {
    PROTOCOL_VERSION
}

/// The stdio encoding a plugin speaks. Declared per-plugin in `plugin.toml`
/// (`protocol = "json" | "latlng"`); defaults to `json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginProtocol {
    /// The canonical v1 JSON protocol ([`PluginInput`]/[`PluginOutput`]).
    #[default]
    Json,
    /// Legacy line protocol of the bundled OR-Tools `tsp` router: whitespace-
    /// separated `lat,lng` tokens on stdin, `lat,lng` tokens (any whitespace) on
    /// stdout, and the `args.raw` string split into `--flag value` argv. Lets a
    /// pre-existing binary plugin run unchanged under the manifest/registry
    /// system (it predates the JSON protocol).
    Latlng,
}

/// Encode points as the legacy `"lat,lng lat,lng …"` stdin string (`[lat, lon]`
/// order, space-separated).
pub fn encode_latlng(points: &SingleVec) -> String {
    points
        .iter()
        .map(|p| format!("{},{}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parse legacy `lat,lng` whitespace-separated tokens from plugin stdout into a
/// [`SingleVec`]. Malformed tokens are skipped.
pub fn decode_latlng(raw: &str) -> SingleVec {
    raw.split_whitespace()
        .filter_map(|tok| {
            let (lat, lon) = tok.split_once(',')?;
            Some([lat.trim().parse().ok()?, lon.trim().parse().ok()?])
        })
        .collect()
}

/// Sent to the plugin's stdin: the protocol version, the points to operate on,
/// plus a free-form `args` object carrying the mode-specific parameters.
///
/// ```json
/// {"protocol_version": 1, "points": [[lat, lon], …], "args": { … }}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
    /// The JSON protocol version this payload speaks. Always [`PROTOCOL_VERSION`]
    /// on output; defaults to it on input when older JSON omits the field.
    #[serde(default = "default_protocol_version")]
    pub protocol_version: u32,
    pub points: SingleVec,
    /// Mode parameters, passed through verbatim. Defaults to JSON `null` when
    /// absent so plugins may omit it.
    #[serde(default)]
    pub args: serde_json::Value,
}

impl PluginInput {
    /// Build an input with no extra args (`args` = JSON null).
    pub fn new(points: SingleVec) -> Self {
        PluginInput {
            protocol_version: PROTOCOL_VERSION,
            points,
            args: serde_json::Value::Null,
        }
    }

    /// Build an input carrying the given args object.
    pub fn with_args(points: SingleVec, args: serde_json::Value) -> Self {
        PluginInput {
            protocol_version: PROTOCOL_VERSION,
            points,
            args,
        }
    }
}

/// Read from the plugin's stdout: the resulting points.
///
/// ```json
/// {"points": [[lat, lon], …]}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOutput {
    pub points: SingleVec,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_round_trips() {
        let input = PluginInput::with_args(
            vec![[1.0, 2.0], [3.0, 4.0]],
            serde_json::json!({ "radius": 70, "min_points": 5 }),
        );
        let json = serde_json::to_string(&input).unwrap();
        let back: PluginInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.protocol_version, PROTOCOL_VERSION);
        assert_eq!(back.points, input.points);
        assert_eq!(back.args, input.args);
    }

    #[test]
    fn input_serializes_with_protocol_version() {
        let input = PluginInput::new(vec![[1.0, 2.0]]);
        let value: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&input).unwrap()).unwrap();
        assert_eq!(value["protocol_version"], serde_json::json!(PROTOCOL_VERSION));
    }

    #[test]
    fn input_args_default_to_null_when_absent() {
        let json = r#"{"points": [[1.0, 2.0]]}"#;
        let input: PluginInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.points, vec![[1.0, 2.0]]);
        assert!(input.args.is_null());
    }

    #[test]
    fn input_protocol_version_defaults_when_absent() {
        // Inbound JSON predating versioning omits the field; it must still
        // deserialize and default to v1.
        let json = r#"{"points": [[1.0, 2.0]], "args": {"radius": 70}}"#;
        let input: PluginInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.protocol_version, PROTOCOL_VERSION);
        assert_eq!(input.points, vec![[1.0, 2.0]]);
        assert_eq!(input.args, serde_json::json!({ "radius": 70 }));
    }

    #[test]
    fn output_round_trips() {
        let output = PluginOutput {
            points: vec![[5.0, 6.0]],
        };
        let json = serde_json::to_string(&output).unwrap();
        let back: PluginOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.points, output.points);
    }

    #[test]
    fn output_parses_from_plugin_shaped_json() {
        let json = r#"{"points": [[37.7749, -122.4194], [40.7128, -74.0060]]}"#;
        let output: PluginOutput = serde_json::from_str(json).unwrap();
        assert_eq!(output.points.len(), 2);
        assert_eq!(output.points[0], [37.7749, -122.4194]);
    }
}
