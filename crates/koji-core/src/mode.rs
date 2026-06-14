//! `Mode` — the scan-purpose tag shared by geofences and routes. Replaces the
//! legacy RDM-derived type/mode enums (the scanner `Type` and the geofence/route
//! mode columns). It is a pure semantic tag and NEVER decides geometry shape (the
//! geometry is self-describing). koji-db mirrors this as a `DeriveActiveEnum` and
//! bridges via `enum_bridge!`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Unset,
    Pokemon,
    Fort,
    Quest,
}

impl Mode {
    /// Map a mode string to the collapsed four-variant `Mode`, accepting BOTH the
    /// 4 canonical wire strings (`pokemon`/`fort`/`quest`/`unset`) AND any of the
    /// 12 historical RDM values. Unknown → `Unset`. This is the single source of
    /// truth for lenient mode parsing (used by `KojiMeta`'s tolerant deserialize).
    pub fn from_legacy(s: &str) -> Self {
        match s {
            // Canonical wire/DB strings.
            "pokemon" => Mode::Pokemon,
            "fort" => Mode::Fort,
            "quest" => Mode::Quest,
            // Legacy 12-value RDM strings.
            "circle_pokemon"
            | "circle_smart_pokemon"
            | "pokemon_iv"
            | "auto_pokemon"
            | "auto_tth" => Mode::Pokemon,
            "circle_raid" | "circle_smart_raid" | "circle_station" => Mode::Fort,
            "auto_quest" | "circle_quest" => Mode::Quest,
            "unset" | "leveling" => Mode::Unset,
            _ => Mode::Unset,
        }
    }

    /// The lowercase wire/DB string for this mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Unset => "unset",
            Mode::Pokemon => "pokemon",
            Mode::Fort => "fort",
            Mode::Quest => "quest",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_uses_lowercase_strings() {
        assert_eq!(
            serde_json::to_string(&Mode::Pokemon).unwrap(),
            "\"pokemon\""
        );
        assert_eq!(serde_json::to_string(&Mode::Unset).unwrap(), "\"unset\"");
        let m: Mode = serde_json::from_str("\"quest\"").unwrap();
        assert_eq!(m, Mode::Quest);
    }

    #[test]
    fn default_is_unset() {
        assert_eq!(Mode::default(), Mode::Unset);
    }

    #[test]
    fn from_legacy_maps_all_twelve() {
        for s in [
            "circle_pokemon",
            "circle_smart_pokemon",
            "pokemon_iv",
            "auto_pokemon",
            "auto_tth",
        ] {
            assert_eq!(Mode::from_legacy(s), Mode::Pokemon, "{s}");
        }
        for s in ["circle_raid", "circle_smart_raid", "circle_station"] {
            assert_eq!(Mode::from_legacy(s), Mode::Fort, "{s}");
        }
        for s in ["auto_quest", "circle_quest"] {
            assert_eq!(Mode::from_legacy(s), Mode::Quest, "{s}");
        }
        for s in ["unset", "leveling", "anything_unknown"] {
            assert_eq!(Mode::from_legacy(s), Mode::Unset, "{s}");
        }
    }

    #[test]
    fn from_legacy_also_accepts_canonical_strings() {
        // `from_legacy` is the single source of truth for lenient parsing, so it
        // must round-trip the 4 canonical wire strings too (not just legacy).
        assert_eq!(Mode::from_legacy("pokemon"), Mode::Pokemon);
        assert_eq!(Mode::from_legacy("fort"), Mode::Fort);
        assert_eq!(Mode::from_legacy("quest"), Mode::Quest);
        assert_eq!(Mode::from_legacy("unset"), Mode::Unset);
    }
}
