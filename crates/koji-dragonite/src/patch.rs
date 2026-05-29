//! Tri-state field wrapper for PATCH bodies.
//!
//! This is the SOUND, reusable part: a JSON PATCH needs to distinguish three
//! intents per field, and Rust's `Option<T>` only encodes two. The semantics
//! here are dictated by the architecture (§7 "respects the tri-state
//! `V2GeofencePatch`: absent=no-op, null=clear, value=set") and are not
//! speculative — only the *set of fields* a real Dragonite patch carries is
//! provisional (see [`crate::types`]).
//!
//! | variant        | serialized as | meaning      |
//! |----------------|---------------|--------------|
//! | [`Tri::Absent`]| (field omitted)| no-op / leave unchanged |
//! | [`Tri::Null`]  | `null`        | clear / unset |
//! | [`Tri::Value`] | the value     | set to value |

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A field that may be absent, explicitly null, or set to a value.
///
/// Use together with `#[serde(skip_serializing_if = "Tri::is_absent", default)]`
/// on the struct field so that [`Tri::Absent`] omits the key entirely. The
/// custom [`Serialize`] impl handles the `null`-vs-value cases on write; the
/// [`Deserialize`] impl + `#[serde(default)]` recovers all three on read
/// (missing key → `Absent` via the default, JSON `null` → `Null`, value →
/// `Value`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Tri<T> {
    /// Field omitted from the body — Dragonite leaves it unchanged.
    #[default]
    Absent,
    /// Field present and explicitly `null` — Dragonite clears it.
    Null,
    /// Field present with a value — Dragonite sets it.
    Value(T),
}

impl<T> Tri<T> {
    /// `true` when this field should be omitted from the serialized body.
    /// Wire it to `skip_serializing_if` so absent fields disappear.
    pub fn is_absent(&self) -> bool {
        matches!(self, Tri::Absent)
    }

    /// `true` when the field is present (either `null` or a value).
    pub fn is_present(&self) -> bool {
        !self.is_absent()
    }
}

impl<T: Serialize> Serialize for Tri<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            // `Absent` is normally skipped by `skip_serializing_if`. If it ever
            // reaches here (e.g. serialized standalone), emit `null` as the
            // least-surprising fallback rather than erroring.
            Tri::Absent | Tri::Null => serializer.serialize_none(),
            Tri::Value(v) => v.serialize(serializer),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Tri<T> {
    /// Recovers `Null` vs `Value`. The `Absent` case is unreachable here —
    /// serde only invokes a field's `Deserialize` when the key is *present*, so
    /// a missing key must be handled by `#[serde(default)]` (→ `Tri::Absent`).
    /// A present JSON `null` deserializes through `Option` to `None` → `Null`;
    /// any other value → `Value`.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(v) => Tri::Value(v),
            None => Tri::Null,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Body {
        #[serde(skip_serializing_if = "Tri::is_absent", default)]
        name: Tri<String>,
    }

    #[test]
    fn absent_skips_the_field_entirely() {
        let body = Body { name: Tri::Absent };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, "{}", "absent must omit the key");
    }

    #[test]
    fn null_emits_explicit_null() {
        let body = Body { name: Tri::Null };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"name":null}"#, "null must serialize as null");
    }

    #[test]
    fn value_emits_the_value() {
        let body = Body {
            name: Tri::Value("alpha".into()),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert_eq!(json, r#"{"name":"alpha"}"#, "value must serialize as value");
    }

    #[test]
    fn default_is_absent() {
        assert!(Tri::<String>::default().is_absent());
    }

    #[test]
    fn deserialize_missing_key_is_absent() {
        let body: Body = serde_json::from_str("{}").unwrap();
        assert!(body.name.is_absent(), "missing key must default to Absent");
    }

    #[test]
    fn deserialize_explicit_null_is_null() {
        let body: Body = serde_json::from_str(r#"{"name":null}"#).unwrap();
        assert_eq!(body.name, Tri::Null, "JSON null must deserialize to Null");
    }

    #[test]
    fn deserialize_value_is_value() {
        let body: Body = serde_json::from_str(r#"{"name":"alpha"}"#).unwrap();
        assert_eq!(body.name, Tri::Value("alpha".into()));
    }

    #[test]
    fn deserialize_then_serialize_round_trips_each_state() {
        for (json, expect) in [
            ("{}", "{}"),
            (r#"{"name":null}"#, r#"{"name":null}"#),
            (r#"{"name":"x"}"#, r#"{"name":"x"}"#),
        ] {
            let body: Body = serde_json::from_str(json).unwrap();
            assert_eq!(
                serde_json::to_string(&body).unwrap(),
                expect,
                "round-trip of {json}"
            );
        }
    }

    #[test]
    fn presence_predicates() {
        assert!(Tri::<i32>::Absent.is_absent());
        assert!(!Tri::<i32>::Absent.is_present());
        assert!(Tri::<i32>::Null.is_present());
        assert!(Tri::Value(1).is_present());
    }
}
