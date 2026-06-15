use serde::{Deserialize, Serialize};

/// An entity id that may arrive as either a string or a number on the wire.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnknownId {
    String(String),
    Number(u32),
}

impl std::fmt::Display for UnknownId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnknownId::Number(id) => write!(f, "{}", id),
            UnknownId::String(id) => f.write_str(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_number_variant() {
        let id = UnknownId::Number(42);
        assert_eq!(id.to_string(), "42");
    }

    #[test]
    fn display_string_variant() {
        let id = UnknownId::String("abc-123".to_string());
        assert_eq!(id.to_string(), "abc-123");
    }

    #[test]
    fn display_number_zero() {
        let id = UnknownId::Number(0);
        assert_eq!(id.to_string(), "0");
    }

    #[test]
    fn serde_deserializes_number_from_json() {
        let id: UnknownId = serde_json::from_value(serde_json::json!(99)).unwrap();
        assert!(matches!(id, UnknownId::Number(99)));
    }

    #[test]
    fn serde_deserializes_string_from_json() {
        let id: UnknownId = serde_json::from_value(serde_json::json!("zone-7")).unwrap();
        assert!(matches!(id, UnknownId::String(ref s) if s == "zone-7"));
    }

    #[test]
    fn serde_serializes_number_as_number() {
        let id = UnknownId::Number(5);
        assert_eq!(serde_json::to_value(&id).unwrap(), serde_json::json!(5));
    }

    #[test]
    fn serde_serializes_string_as_string() {
        let id = UnknownId::String("test".to_string());
        assert_eq!(
            serde_json::to_value(&id).unwrap(),
            serde_json::json!("test")
        );
    }

    #[test]
    fn clone_works_for_both_variants() {
        let n = UnknownId::Number(7);
        let s = UnknownId::String("x".to_string());
        let _n2 = n.clone();
        let _s2 = s.clone();
    }
}
