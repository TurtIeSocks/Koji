//! Dragonite `/v2/areas` wire types.
//!
//! ⚠️ **PROVISIONAL.** There is no Dragonite OpenAPI/schema in this repo, so the
//! exact JSON field names and the `ApiArea`/`ApiGeofence` shapes below are NOT
//! verified. Every speculative field carries a
//! `// TODO(dragonite-reconcile): ...` marker. Keep these MINIMAL until the real
//! schema lands — do not grow them by guessing.
//!
//! What IS known / sound here:
//! - [`AreaMode`] mirrors the `area_fence.mode` ENUM that already exists in
//!   Koji's own migration (`m20260529_000003_dragonite_linkage`): exactly
//!   `base | pokemon | quest | fort`, lowercase (architecture §7/§10, PR #558).
//! - GeoJSON-native geometry is a *design decision* from §7 ("Koji `Feature`
//!   fence → `ApiGeofence`, both GeoJSON-native, zero-loss"); whether Dragonite
//!   serializes it as a bare `geojson::Feature`, a `geometry` object, or a
//!   `[lat,lon]` array is unverified.

use geojson::Feature;
use serde::{Deserialize, Serialize};

use crate::patch::Tri;

/// Per-mode geofence slot, mirroring the `area_fence.mode` ENUM (PR #558).
///
/// This enum is sound: it matches the migration's
/// `ENUM('base','pokemon','quest','fort')` exactly. `base` is the fallback that
/// per-mode rows override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AreaMode {
    Base,
    Pokemon,
    Quest,
    Fort,
}

impl AreaMode {
    /// All four modes, in `base`-first order (handy for gathering an area's
    /// slots into a single patch).
    pub const ALL: [AreaMode; 4] = [
        AreaMode::Base,
        AreaMode::Pokemon,
        AreaMode::Quest,
        AreaMode::Fort,
    ];
}

/// A geofence as carried by the Dragonite `/v2/areas` API.
///
/// Modeled GeoJSON-native per architecture §7. We keep a single `feature` field
/// so the GeoJSON passes through zero-loss in either direction.
///
/// ⚠️ PROVISIONAL shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiGeofence {
    // TODO(dragonite-reconcile): verify against Dragonite /v2/areas OpenAPI —
    // the real field name (`geofence`? `geometry`? `fence`?), and whether the
    // payload is a full GeoJSON Feature, a bare Geometry, or a [lat,lon] ring
    // array. Choosing a GeoJSON Feature here per the §7 "GeoJSON-native" design.
    pub feature: Feature,
}

/// An area record from the Dragonite `/v2/areas` API.
///
/// ⚠️ PROVISIONAL shape — kept deliberately tiny. Only `id` is treated as
/// load-bearing (it is the `dragonite_area_id` linkage target, architecture §9).
/// Everything else is a placeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiArea {
    // TODO(dragonite-reconcile): verify the identity field name + type. Koji's
    // migration stores `dragonite_area_id INT UNSIGNED NULL`, so `u32` is the
    // matching Rust type, but the JSON key ("id"? "area_id"?) is unverified.
    pub id: u32,

    // TODO(dragonite-reconcile): verify — areas almost certainly have a name,
    // but the key is a guess. Optional so a minimal response still decodes.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,

    // TODO(dragonite-reconcile): verify — whether areas embed their per-mode
    // geofences inline (and under what key), or whether geofences are a separate
    // endpoint, is entirely unknown. Optional placeholder.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub base_geofence: Option<ApiGeofence>,
}

/// Tri-state PATCH body for an area's geofences.
///
/// Carries the per-mode fence patch fields (PR #558). Each field is a [`Tri`]:
/// absent = leave unchanged, `null` = clear that mode's fence, value = set it.
/// The client sends only the fields the caller set.
///
/// ⚠️ PROVISIONAL — the *existence* of per-mode patch fields is grounded in §7
/// ("base + per-mode `geofence` patch fields") and PR #558, but the JSON key
/// names are guesses.
#[derive(Debug, Clone, Default, Serialize)]
pub struct V2GeofencePatch {
    // TODO(dragonite-reconcile): verify field name ("geofence"? "base"?).
    #[serde(skip_serializing_if = "Tri::is_absent", default)]
    pub base: Tri<ApiGeofence>,

    // TODO(dragonite-reconcile): verify field name ("pokemon_geofence"? "pokemon"?).
    #[serde(skip_serializing_if = "Tri::is_absent", default)]
    pub pokemon: Tri<ApiGeofence>,

    // TODO(dragonite-reconcile): verify field name.
    #[serde(skip_serializing_if = "Tri::is_absent", default)]
    pub quest: Tri<ApiGeofence>,

    // TODO(dragonite-reconcile): verify field name.
    #[serde(skip_serializing_if = "Tri::is_absent", default)]
    pub fort: Tri<ApiGeofence>,
}

impl V2GeofencePatch {
    /// `true` when no field is set — sending this would be a no-op PATCH. The
    /// client can use this to skip empty mutations.
    pub fn is_empty(&self) -> bool {
        self.base.is_absent()
            && self.pokemon.is_absent()
            && self.quest.is_absent()
            && self.fort.is_absent()
    }
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
        let patch = V2GeofencePatch::default();
        assert!(patch.is_empty());
        assert_eq!(serde_json::to_string(&patch).unwrap(), "{}");
    }

    #[test]
    fn patch_with_one_cleared_mode_emits_only_that_field_as_null() {
        let patch = V2GeofencePatch {
            quest: Tri::Null,
            ..Default::default()
        };
        assert!(!patch.is_empty());
        assert_eq!(serde_json::to_string(&patch).unwrap(), r#"{"quest":null}"#);
    }
}
