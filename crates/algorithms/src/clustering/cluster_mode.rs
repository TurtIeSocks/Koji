use macros::StrEnum;

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
pub enum ClusterMode {
    #[str("honeycomb")]
    Honeycomb,
    #[str("fastest")]
    Fastest,
    #[str("fast")]
    Fast,
    #[str("balanced")]
    Balanced,
    #[str("better")]
    Better,
    #[str("best")]
    Best,
    #[str(default)]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── from_str_opt: canonical names ─────────────────────────────────────────

    #[test]
    fn canonical_names_recognized() {
        for (s, expected) in [
            ("honeycomb", ClusterMode::Honeycomb),
            ("fastest", ClusterMode::Fastest),
            ("fast", ClusterMode::Fast),
            ("balanced", ClusterMode::Balanced),
            ("better", ClusterMode::Better),
            ("best", ClusterMode::Best),
        ] {
            let parsed = ClusterMode::from_str_opt(s);
            assert_eq!(parsed, Some(expected), "failed for '{s}'");
        }
    }

    #[test]
    fn unknown_string_becomes_custom() {
        let v = ClusterMode::from_str_opt("my_plugin").unwrap();
        assert!(matches!(v, ClusterMode::Custom(ref s) if s == "my_plugin"));
    }

    #[test]
    fn custom_equality() {
        assert_eq!(
            ClusterMode::Custom("x".into()),
            ClusterMode::Custom("x".into())
        );
        assert_ne!(
            ClusterMode::Custom("x".into()),
            ClusterMode::Custom("y".into())
        );
    }

    #[test]
    fn display_canonical() {
        assert_eq!(format!("{}", ClusterMode::Fastest), "fastest");
        assert_eq!(format!("{}", ClusterMode::Best), "best");
        assert_eq!(format!("{}", ClusterMode::Honeycomb), "honeycomb");
    }

    // ── serde round-trip ──────────────────────────────────────────────────────

    #[test]
    fn serde_round_trip() {
        for mode in [
            ClusterMode::Honeycomb,
            ClusterMode::Fastest,
            ClusterMode::Fast,
            ClusterMode::Balanced,
            ClusterMode::Better,
            ClusterMode::Best,
            ClusterMode::Custom("plugin_x".into()),
        ] {
            let json = serde_json::to_string(&mode).unwrap();
            let back: ClusterMode = serde_json::from_str(&json).unwrap();
            assert_eq!(back, mode, "serde round-trip failed for {mode}");
        }
    }
}
