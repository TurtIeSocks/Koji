//! Koji → Dragonite conversions + per-mode PATCH builders.
//!
//! `koji_core::SingleVec` is `Vec<[f64; 2]>` ordered `[lat, lon]` (confirmed by
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
}
