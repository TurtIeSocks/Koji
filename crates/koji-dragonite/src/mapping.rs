//! Koji ↔ Dragonite conversions.
//!
//! The route mapping here is SOUND — it is grounded entirely in koji-core types.
//! `koji_core::SingleVec` is `Vec<[f64; 2]>`, and the points are ordered
//! `[lat, lon]` (confirmed by `koji_core::geometry::single_vec::GetBbox`, which
//! treats `point[0]` as latitude and `point[1]` as longitude). Dragonite's route
//! arrays are likewise lat/lon pairs, so this is an identity copy — documented
//! and tested so the ordering assumption is explicit and can't silently flip.
//!
//! The geofence mapping is a STUB that carries GeoJSON through unchanged; the
//! target shape is provisional (see [`crate::types::ApiGeofence`]).

use geojson::Feature;
use koji_core::SingleVec;

use crate::types::ApiGeofence;

/// Convert a Koji route ([`koji_core::SingleVec`], `Vec<[f64; 2]>` in
/// `[lat, lon]` order) into a Dragonite route array (`Vec<[f64; 2]>`, also
/// `[lat, lon]`).
///
/// This is an identity copy. It exists as a named, tested boundary so the
/// `[lat, lon]` ordering assumption is documented in one place. If Dragonite
/// ever turns out to expect `[lon, lat]`, flip it *here* and the test will guard
/// the change.
///
/// Ordering assumption: **`[lat, lon]`** on both sides.
pub fn route_to_dragonite(route: &SingleVec) -> Vec<[f64; 2]> {
    route.clone()
}

/// Carry a Koji `geojson::Feature` fence into an [`ApiGeofence`].
///
/// ⚠️ STUB. The Koji side is a real `geojson::Feature`; this clones it into the
/// provisional GeoJSON-native `ApiGeofence` wrapper. Whether Dragonite actually
/// wants a full Feature (vs. a bare geometry or a coordinate ring) is
/// unverified — see [`crate::types::ApiGeofence`].
// TODO(dragonite-reconcile): verify against Dragonite /v2/areas OpenAPI — the
// target geofence representation. Today this is a zero-loss Feature pass-through.
pub fn feature_to_api_geofence(feat: &Feature) -> ApiGeofence {
    ApiGeofence {
        feature: feat.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_to_dragonite_preserves_points_and_order() {
        // [lat, lon] pairs.
        let route: SingleVec = vec![[40.1, -75.2], [40.3, -75.4], [40.5, -75.6]];
        let out = route_to_dragonite(&route);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0], [40.1, -75.2]);
        assert_eq!(out[1], [40.3, -75.4]);
        assert_eq!(out[2], [40.5, -75.6]);
        // First element's first component is latitude (the documented ordering).
        assert_eq!(out[0][0], 40.1, "index 0 must be latitude");
        assert_eq!(out[0][1], -75.2, "index 1 must be longitude");
    }

    #[test]
    fn route_to_dragonite_handles_empty() {
        let route: SingleVec = vec![];
        assert!(route_to_dragonite(&route).is_empty());
    }
}
