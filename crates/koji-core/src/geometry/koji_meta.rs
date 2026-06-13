//! `KojiMeta` — the hybrid metadata sidecar for `KojiGeometry`. Typed fields for
//! what Koji branches on; a flattened `extra` bag carries arbitrary geojson
//! properties losslessly. Replaces `FeatureCtx` and the geojson `properties`
//! object.

use serde::{Deserialize, Serialize};

use crate::Mode;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KojiMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ancestors: Vec<String>,
    /// Lossless passthrough for any property not promoted to a typed field.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn typed_fields_and_extra_roundtrip_losslessly() {
        let json = serde_json::json!({
            "id": 7,
            "name": "Denver",
            "mode": "fort",
            "parent_id": 3,
            "color": "#ff0000",          // unknown -> extra
            "custom_flag": true          // unknown -> extra
        });
        let meta: KojiMeta = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(meta.id, Some(7));
        assert_eq!(meta.mode, Mode::Fort);
        assert_eq!(meta.parent_id, Some(3));
        assert_eq!(meta.extra.get("color").unwrap(), "#ff0000");
        assert!(meta.ancestors.is_empty());
        // round-trips back to the same object
        assert_eq!(serde_json::to_value(&meta).unwrap(), json);
    }

    #[test]
    fn empty_meta_omits_optional_keys() {
        let v = serde_json::to_value(KojiMeta::default()).unwrap();
        assert_eq!(v, serde_json::json!({ "mode": "unset" }));
    }
}
