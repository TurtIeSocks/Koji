//! Koji → Dragonite conversions + per-mode PATCH builders.
//!
//! `koji_core::SingleVec` is `Vec<[Precision; 2]>` ordered `[lat, lon]` (confirmed by
//! `koji_core::KojiBbox::from_points`, which reads `point[0]` as latitude).
//! Dragonite routes are `[{lat, lon}]` objects, so the conversion is
//! a field rename — documented + tested here so the ordering can't silently
//! flip. Geofences are GeoJSON `Feature`s on both sides (zero-loss).

use geojson::Feature;
use koji_core::SingleVec;

use crate::patch::Tri;
use crate::types::{
    ApiArea, ApiAreaFortMode, ApiAreaPokemonMode, ApiAreaQuestMode, ApiLocation, AreaMode,
    V2GeofencePatch,
};

/// Convert a Koji route ([`SingleVec`], `[lat, lon]` points) into Dragonite's
/// `[{lat, lon}]` array. Ordering assumption: **`point[0] = lat`,
/// `point[1] = lon`**.
pub fn route_to_api_locations(route: &SingleVec) -> Vec<ApiLocation> {
    route
        .iter()
        .map(|p| ApiLocation {
            lat: p[0],
            lon: p[1],
        })
        .collect()
}

/// Wrap a Koji `geojson::Feature` fence as a geofence patch value (`Tri::Value`
/// → "set this fence"). Dragonite accepts the bare `Feature` directly.
pub fn feature_to_geofence(feat: &Feature) -> V2GeofencePatch {
    Tri::Value(feat.clone())
}

/// Build a partial [`ApiArea`] PATCH that sets `mode`'s **route** to `route`.
///
/// Routes are per-mode; [`AreaMode::Base`] has no route slot and yields an
/// empty (no-op) patch.
pub fn area_route_patch(mode: AreaMode, route: &SingleVec) -> ApiArea {
    let locs = route_to_api_locations(route);
    let mut area = ApiArea::default();
    match mode {
        AreaMode::Pokemon => {
            area.pokemon_mode = Some(ApiAreaPokemonMode {
                route: locs,
                ..Default::default()
            })
        }
        AreaMode::Quest => {
            area.quest_mode = Some(ApiAreaQuestMode {
                route: locs,
                ..Default::default()
            })
        }
        AreaMode::Fort => {
            area.fort_mode = Some(ApiAreaFortMode {
                route: locs,
                ..Default::default()
            })
        }
        AreaMode::Base => {}
    }
    area
}

/// Build a partial [`ApiArea`] PATCH that sets `mode`'s **geofence** to `feat`.
/// [`AreaMode::Base`] targets the area-root fence; the others target their mode
/// block's fence.
pub fn area_geofence_patch(mode: AreaMode, feat: &Feature) -> ApiArea {
    let g = feature_to_geofence(feat);
    let mut area = ApiArea::default();
    match mode {
        AreaMode::Base => area.geofence = g,
        AreaMode::Pokemon => {
            area.pokemon_mode = Some(ApiAreaPokemonMode {
                geofence: g,
                ..Default::default()
            })
        }
        AreaMode::Quest => {
            area.quest_mode = Some(ApiAreaQuestMode {
                geofence: g,
                ..Default::default()
            })
        }
        AreaMode::Fort => {
            area.fort_mode = Some(ApiAreaFortMode {
                geofence: g,
                ..Default::default()
            })
        }
    }
    area
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn route_conversion_preserves_points_and_order() {
        let route: SingleVec = vec![[40.1, -75.2], [40.3, -75.4]];
        let out = route_to_api_locations(&route);
        assert_eq!(out.len(), 2);
        assert_eq!(
            out[0],
            ApiLocation {
                lat: 40.1,
                lon: -75.2
            }
        );
        assert_eq!(out[0].lat, 40.1, "index 0 must be latitude");
        assert_eq!(out[0].lon, -75.2, "index 1 must be longitude");
    }

    #[test]
    fn route_conversion_handles_empty() {
        assert!(route_to_api_locations(&vec![]).is_empty());
    }

    #[test]
    fn quest_route_patch_targets_quest_mode_only() {
        let patch = area_route_patch(AreaMode::Quest, &vec![[1.0, 2.0]]);
        assert_eq!(
            serde_json::to_value(&patch).unwrap(),
            json!({ "quest_mode": { "route": [{"lat": 1.0, "lon": 2.0}] } })
        );
    }

    #[test]
    fn base_route_patch_is_noop() {
        // Base has no route slot.
        let patch = area_route_patch(AreaMode::Base, &vec![[1.0, 2.0]]);
        assert_eq!(serde_json::to_string(&patch).unwrap(), "{}");
    }

    #[test]
    fn base_geofence_patch_targets_root() {
        let feat = Feature::default();
        let patch = area_geofence_patch(AreaMode::Base, &feat);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("geofence").is_some(), "base fence at area root");
        assert!(v.get("pokemon_mode").is_none());
    }

    #[test]
    fn pokemon_geofence_patch_nests_under_mode() {
        let feat = Feature::default();
        let patch = area_geofence_patch(AreaMode::Pokemon, &feat);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("geofence").is_none(), "no root fence");
        assert!(
            v["pokemon_mode"].get("geofence").is_some(),
            "fence nested under pokemon_mode"
        );
    }

    #[test]
    fn pokemon_route_patch_targets_pokemon_mode_only() {
        let patch = area_route_patch(AreaMode::Pokemon, &vec![[1.0, 2.0], [3.0, 4.0]]);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("quest_mode").is_none());
        assert!(v.get("fort_mode").is_none());
        assert!(v.get("geofence").is_none());
        let pm = v["pokemon_mode"].as_object().expect("pokemon_mode present");
        let route = pm["route"].as_array().expect("route present");
        assert_eq!(route.len(), 2);
        assert_eq!(route[0]["lat"], 1.0);
        assert_eq!(route[0]["lon"], 2.0);
        assert_eq!(route[1]["lat"], 3.0);
        assert_eq!(route[1]["lon"], 4.0);
    }

    #[test]
    fn fort_route_patch_targets_fort_mode_only() {
        let patch = area_route_patch(AreaMode::Fort, &vec![[10.0, 20.0]]);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("pokemon_mode").is_none());
        assert!(v.get("quest_mode").is_none());
        assert!(v.get("geofence").is_none());
        let fm = v["fort_mode"].as_object().expect("fort_mode present");
        let route = fm["route"].as_array().expect("route present");
        assert_eq!(route.len(), 1);
        assert_eq!(route[0]["lat"], 10.0);
        assert_eq!(route[0]["lon"], 20.0);
    }

    #[test]
    fn quest_geofence_patch_nests_under_quest_mode() {
        let feat = Feature::default();
        let patch = area_geofence_patch(AreaMode::Quest, &feat);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("geofence").is_none(), "no root fence");
        assert!(v.get("pokemon_mode").is_none());
        assert!(v.get("fort_mode").is_none());
        assert!(
            v["quest_mode"].get("geofence").is_some(),
            "fence nested under quest_mode"
        );
    }

    #[test]
    fn fort_geofence_patch_nests_under_fort_mode() {
        let feat = Feature::default();
        let patch = area_geofence_patch(AreaMode::Fort, &feat);
        let v = serde_json::to_value(&patch).unwrap();
        assert!(v.get("geofence").is_none(), "no root fence");
        assert!(v.get("pokemon_mode").is_none());
        assert!(v.get("quest_mode").is_none());
        assert!(
            v["fort_mode"].get("geofence").is_some(),
            "fence nested under fort_mode"
        );
    }

    #[test]
    fn feature_to_geofence_returns_tri_value() {
        let feat = Feature {
            id: Some(geojson::feature::Id::String("test-id".into())),
            ..Feature::default()
        };
        let result = feature_to_geofence(&feat);
        match result {
            crate::patch::Tri::Value(f) => {
                assert_eq!(f.id, Some(geojson::feature::Id::String("test-id".into())));
            }
            other => panic!("expected Tri::Value, got {:?}", other),
        }
    }

    #[test]
    fn route_to_api_locations_single_point() {
        let route = vec![[51.5, -0.1]];
        let locs = route_to_api_locations(&route);
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].lat, 51.5);
        assert_eq!(locs[0].lon, -0.1);
    }

    #[test]
    fn route_patch_empty_route_produces_empty_route_array() {
        // Empty route is valid — no panic, no spurious points.
        let patch = area_route_patch(AreaMode::Pokemon, &vec![]);
        let v = serde_json::to_value(&patch).unwrap();
        // Empty route serializes to omitted (skip_serializing_if = Vec::is_empty).
        assert_eq!(
            v.get("pokemon_mode").and_then(|m| m.get("route")),
            None,
            "empty route must be omitted from wire payload"
        );
    }

    #[test]
    fn all_area_modes_covered_by_route_patch() {
        // Regression guard: if AreaMode gains a new variant, route_patch must handle it.
        // This test exercises all variants through the public API.
        for mode in AreaMode::ALL {
            let _patch = area_route_patch(mode, &vec![[0.0, 0.0]]);
        }
    }

    #[test]
    fn all_area_modes_covered_by_geofence_patch() {
        let feat = Feature::default();
        for mode in AreaMode::ALL {
            let _patch = area_geofence_patch(mode, &feat);
        }
    }
}
