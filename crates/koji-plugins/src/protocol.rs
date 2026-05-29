//! The JSON stdio protocol (v1) spoken between Koji and an external plugin.
//!
//! Koji writes a [`PluginInput`] as a single JSON object to the plugin's stdin
//! and reads a [`PluginOutput`] JSON object from its stdout. A non-zero exit
//! status or unparseable stdout is treated as an error by the runner.
//!
//! `SingleVec` is `Vec<[f64; 2]>` in `[lat, lon]` order, so the JSON point
//! arrays map onto it directly.

use koji_core::SingleVec;
use serde::{Deserialize, Serialize};

/// Sent to the plugin's stdin: the points to operate on plus a free-form `args`
/// object carrying the mode-specific parameters.
///
/// ```json
/// {"points": [[lat, lon], …], "args": { … }}
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInput {
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
            points,
            args: serde_json::Value::Null,
        }
    }

    /// Build an input carrying the given args object.
    pub fn with_args(points: SingleVec, args: serde_json::Value) -> Self {
        PluginInput { points, args }
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
        assert_eq!(back.points, input.points);
        assert_eq!(back.args, input.args);
    }

    #[test]
    fn input_args_default_to_null_when_absent() {
        let json = r#"{"points": [[1.0, 2.0]]}"#;
        let input: PluginInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.points, vec![[1.0, 2.0]]);
        assert!(input.args.is_null());
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
