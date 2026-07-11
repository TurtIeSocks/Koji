//! `KojiMeta` — the hybrid metadata sidecar for `KojiGeometry`. Typed fields for
//! what Koji branches on; a flattened `extra` bag carries arbitrary geojson
//! properties losslessly. Replaces the old feature-context struct and the geojson
//! `properties` object.

use serde::{Deserialize, Deserializer, Serialize};

use crate::Mode;

/// Lenient deserializer for the `mode` field. Inbound geojson `properties` may
/// carry EITHER a canonical wire string (`pokemon`/`fort`/`quest`/`unset`) OR a
/// legacy 12-value RDM mode string (e.g. `"circle_raid"`). Both map to the
/// collapsed `Mode` via `Mode::from_legacy` (the single source of truth);
/// unknown strings collapse to `Mode::Unset`. This prevents a legacy value from
/// failing the whole `KojiMeta` deserialize and silently dropping every sibling
/// property (the S5c data-loss regression). The serialize path is unaffected —
/// `Mode` still serializes to its canonical lowercase string.
fn deserialize_mode_lenient<'de, D>(deserializer: D) -> Result<Mode, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Mode::from_legacy(&s))
}

/// Lenient deserializer for `id`/`parent_id`. The wire demonstrably carries
/// string ids; a strict `Option<u32>` made one string id fail the WHOLE
/// `KojiMeta` deserialize, and the caller's `unwrap_or_default()` then silently
/// discarded every sibling property (same S5c data-loss class the lenient
/// `mode` deserializer exists for). Accept integers and numeric strings;
/// anything else degrades to `None` instead of poisoning the struct.
fn deserialize_u32_lenient<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(match &v {
        serde_json::Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()),
        serde_json::Value::String(s) => s.parse::<u32>().ok(),
        _ => None,
    })
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KojiMeta {
    #[serde(
        default,
        deserialize_with = "deserialize_u32_lenient",
        skip_serializing_if = "Option::is_none"
    )]
    pub id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_mode_lenient")]
    pub mode: Mode,
    #[serde(
        default,
        deserialize_with = "deserialize_u32_lenient",
        skip_serializing_if = "Option::is_none"
    )]
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

    /// String ids must not poison the struct: `{"id": "5"}` parses as `Some(5)`
    /// and — critically — every sibling property survives. (Regression: a strict
    /// `Option<u32>` failed the whole deserialize; the `unwrap_or_default()`
    /// caller then nuked name/parent_id/extra.)
    #[test]
    fn string_and_garbage_ids_do_not_drop_sibling_props() {
        let meta: KojiMeta = serde_json::from_value(serde_json::json!({
            "id": "5", "name": "Denver", "color": "#f00"
        }))
        .unwrap();
        assert_eq!(meta.id, Some(5));
        assert_eq!(meta.name.as_deref(), Some("Denver"));
        assert_eq!(meta.extra.get("color").unwrap(), "#f00");

        // Non-numeric id degrades to None; siblings still survive.
        let meta: KojiMeta = serde_json::from_value(serde_json::json!({
            "id": "not-a-number", "parent_id": -3, "name": "x"
        }))
        .unwrap();
        assert_eq!(meta.id, None);
        assert_eq!(meta.parent_id, None);
        assert_eq!(meta.name.as_deref(), Some("x"));

        // Numeric string parent_id parses too.
        let meta: KojiMeta =
            serde_json::from_value(serde_json::json!({ "parent_id": "12" })).unwrap();
        assert_eq!(meta.parent_id, Some(12));
    }

    /// Inbound geojson `properties` may carry a legacy 12-value RDM mode string
    /// (e.g. `"circle_raid"`). The `mode` field must deserialize leniently — map
    /// the legacy value to the collapsed `Mode` — instead of failing the whole
    /// `KojiMeta` deserialize and discarding every sibling property (the S5c
    /// data-loss regression). Canonical strings and an absent key must still work,
    /// and serialize must stay CANONICAL.
    #[test]
    fn legacy_mode_string_deserializes_leniently_without_dropping_props() {
        // Legacy 12-value string + a sibling property: both must survive.
        let meta: KojiMeta =
            serde_json::from_value(serde_json::json!({ "mode": "circle_raid", "name": "x" }))
                .unwrap();
        assert_eq!(meta.mode, Mode::Fort);
        assert_eq!(meta.name.as_deref(), Some("x"));

        // Canonical string still deserializes.
        let meta: KojiMeta =
            serde_json::from_value(serde_json::json!({ "mode": "pokemon" })).unwrap();
        assert_eq!(meta.mode, Mode::Pokemon);

        // Absent key still defaults to Unset.
        let meta: KojiMeta = serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(meta.mode, Mode::Unset);

        // Genuinely-unknown string collapses to Unset (no error).
        let meta: KojiMeta =
            serde_json::from_value(serde_json::json!({ "mode": "totally_bogus" })).unwrap();
        assert_eq!(meta.mode, Mode::Unset);

        // Serialize stays canonical regardless of how it was parsed in.
        let meta: KojiMeta =
            serde_json::from_value(serde_json::json!({ "mode": "circle_raid" })).unwrap();
        assert_eq!(
            serde_json::to_value(&meta).unwrap(),
            serde_json::json!({ "mode": "fort" })
        );
    }
}
