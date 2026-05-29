//! Dragonite `/v2/areas` wire types — reconciled against the real contract
//! (`routes/areas.go`, `routes/v2_areas.go`, `routes/v2_geofence*.go`).
//!
//! ## Shape (verified)
//! `ApiAreaV2 = ApiArea[V2GeofencePatch]`. Every field is `omitempty`/optional:
//! a request sends only what it intends to change; a response populates all of
//! them. The geofence field is the **same type** in every position — a single
//! tri-state nullable GeoJSON `Feature` ([`V2GeofencePatch`]) — appearing once
//! at the area root (the *base* fence) and once inside each mode block. This is
//! the inverse of an earlier guess that modeled one object keyed by mode.
//!
//! ```jsonc
//! {
//!   "id": 42, "name": "downtown", "enabled": true, "enable_quests": true,
//!   "geofence": <Feature|null>,                       // base fence
//!   "pokemon_mode": { "workers": 4, "route": [{"lat":..,"lon":..}],
//!                     "enable_scout": false, "invasion": true,
//!                     "geofence": <Feature|null> },
//!   "quest_mode":   { "workers": 2, "hours": [0,6,12], "max_login_queue": 5,
//!                     "route": [...], "geofence": <Feature|null> },
//!   "fort_mode":    { "workers": 1, "route": [...], "full_route": [...],
//!                     "prio_raid": true, "showcase": false, "invasion": true,
//!                     "geofence": <Feature|null> },
//!   "rare_pokemon_mode": { "workers": 0, "permyriad": 100, "extra": [25,26] }
//! }
//! ```
//!
//! The geofence value marshals/parses as a bare GeoJSON `Feature` (Dragonite
//! also *accepts* a bare Geometry or a legacy `[{lat,lon}]` ring on input, but
//! Koji always sends the canonical `Feature`).

use geojson::Feature;
use serde::{Deserialize, Serialize};

use crate::patch::Tri;

/// A geofence field as carried by `/v2/areas`: a tri-state nullable GeoJSON
/// `Feature`. `Absent` = field omitted (PATCH no-op), `Null` = clear the fence,
/// `Value(feature)` = set it. On responses the field is always present, so it
/// decodes to `Null` (empty fence) or `Value`.
pub type V2GeofencePatch = Tri<Feature>;

/// A single route point, `{"lat":…,"lon":…}` (Dragonite `ApiLocation`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ApiLocation {
    pub lat: f64,
    pub lon: f64,
}

/// Per-mode geofence slot identifier, mirroring the `area_fence.mode` ENUM
/// (`ENUM('base','pokemon','quest','fort')`, migration
/// `m20260529_000003_dragonite_linkage`, architecture §10 / PR #558).
///
/// This is a Koji-internal selector for *which* slot a calculated route/fence
/// targets — it is **not** itself a wire field (the wire keys the slots by
/// position: the area-root `geofence` is `Base`, and each mode block carries
/// its own `geofence`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AreaMode {
    Base,
    Pokemon,
    Quest,
    Fort,
}

impl AreaMode {
    /// All four modes, `base`-first.
    pub const ALL: [AreaMode; 4] = [
        AreaMode::Base,
        AreaMode::Pokemon,
        AreaMode::Quest,
        AreaMode::Fort,
    ];
}

/// Pokémon-mode block of an area.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiAreaPokemonMode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<ApiLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_scout: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invasion: Option<bool>,
    #[serde(default, skip_serializing_if = "Tri::is_absent")]
    pub geofence: V2GeofencePatch,
}

/// Quest-mode block of an area.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiAreaQuestMode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hours: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_login_queue: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<ApiLocation>,
    #[serde(default, skip_serializing_if = "Tri::is_absent")]
    pub geofence: V2GeofencePatch,
}

/// Fort-mode block of an area.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiAreaFortMode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub route: Vec<ApiLocation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub full_route: Vec<ApiLocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prio_raid: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub showcase: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invasion: Option<bool>,
    #[serde(default, skip_serializing_if = "Tri::is_absent")]
    pub geofence: V2GeofencePatch,
}

/// Rare-Pokémon-mode block of an area (no geofence; carries an `extra` Pokédex
/// id list).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiAreaRarePokemonMode {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workers: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permyriad: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<i64>,
}

/// An area record from Dragonite's `/v2/areas` API — used both as the response
/// body and as a partial PATCH/POST request body (send only the fields to
/// change; the rest stay `None`/`Absent` and are omitted).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiArea {
    /// Server-assigned id. Present on responses; omit on create. The
    /// `dragonite_area_id` linkage target (architecture §9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enable_quests: Option<bool>,
    /// Base geofence (the area-root fence; per-mode fences override it).
    #[serde(default, skip_serializing_if = "Tri::is_absent")]
    pub geofence: V2GeofencePatch,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pokemon_mode: Option<ApiAreaPokemonMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quest_mode: Option<ApiAreaQuestMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fort_mode: Option<ApiAreaFortMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rare_pokemon_mode: Option<ApiAreaRarePokemonMode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_mode_serializes_lowercase() {
        assert_eq!(
            serde_json::to_string(&AreaMode::Pokemon).unwrap(),
            r#""pokemon""#
        );
        assert_eq!(serde_json::to_string(&AreaMode::Base).unwrap(), r#""base""#);
    }

    #[test]
    fn empty_patch_serializes_to_empty_object() {
        // A default ApiArea sends nothing — the canonical no-op PATCH body.
        assert_eq!(serde_json::to_string(&ApiArea::default()).unwrap(), "{}");
    }

    #[test]
    fn route_only_patch_emits_only_that_mode() {
        let patch = ApiArea {
            quest_mode: Some(ApiAreaQuestMode {
                route: vec![ApiLocation {
                    lat: 40.1,
                    lon: -75.2,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_value(&patch).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "quest_mode": { "route": [{"lat": 40.1, "lon": -75.2}] }
            })
        );
    }

    #[test]
    fn full_area_response_round_trips() {
        // Mirrors buildSingleAreaV2's output shape.
        let body = r#"{
            "id": 42, "name": "downtown", "enabled": true, "enable_quests": true,
            "geofence": {"type":"Feature","properties":{},"geometry":{"type":"Point","coordinates":[1.0,2.0]}},
            "pokemon_mode": {"workers":4,"route":[{"lat":40.0,"lon":-75.0}],"enable_scout":false,"invasion":true,"geofence":null},
            "quest_mode": {"workers":2,"hours":[0,6,12],"max_login_queue":5,"route":[],"geofence":null},
            "fort_mode": {"workers":1,"route":[],"full_route":[],"prio_raid":true,"showcase":false,"invasion":true,"geofence":null},
            "rare_pokemon_mode": {"workers":0,"permyriad":100,"extra":[25,26]}
        }"#;
        let area: ApiArea = serde_json::from_str(body).expect("v2 area must decode");
        assert_eq!(area.id, Some(42));
        assert_eq!(area.name.as_deref(), Some("downtown"));
        // base geofence present (a Feature)
        assert!(matches!(area.geofence, Tri::Value(_)));
        let pm = area.pokemon_mode.expect("pokemon_mode present");
        assert_eq!(pm.workers, Some(4));
        assert_eq!(pm.route.len(), 1);
        assert_eq!(
            pm.route[0],
            ApiLocation {
                lat: 40.0,
                lon: -75.0
            }
        );
        // explicit null per-mode geofence → Tri::Null
        assert_eq!(pm.geofence, Tri::Null);
        let qm = area.quest_mode.expect("quest_mode present");
        assert_eq!(qm.hours, vec![0, 6, 12]);
        assert_eq!(qm.max_login_queue, Some(5));
        assert_eq!(
            area.rare_pokemon_mode.expect("rare present").extra,
            vec![25, 26]
        );
    }

    #[test]
    fn geofence_clear_serializes_as_null() {
        // Clearing the base fence (Tri::Null) must emit an explicit null.
        let patch = ApiArea {
            geofence: Tri::Null,
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&patch).unwrap(),
            r#"{"geofence":null}"#
        );
    }
}
