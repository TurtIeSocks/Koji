//! Shared resolution helpers + default constants used by the arg-groups.
//!
//! These mirror the legacy fat-`Args` `init()` exactly (the parity contract is
//! the spec §2 defaults table). `resolve_data_points` is the single source of
//! truth for the v2 arg-groups.

use super::inputs::DataPointsArg;
use koji_core::Precision;
use koji_core::{KojiGeometry, KojiGeometryCollection};

pub(crate) const DEFAULT_RADIUS: Precision = 70.0;
pub(crate) const DEFAULT_S2_LEVEL: u8 = 15;
pub(crate) const DEFAULT_S2_SIZE: u8 = 9;
pub(crate) const DEFAULT_MIN_POINTS: usize = 1;

/// `None`/`0` → `usize::MAX`, else the value (parity with old `init()`).
pub(crate) fn resolve_max_clusters(v: Option<usize>) -> usize {
    match v {
        Some(0) | None => usize::MAX,
        Some(n) => n,
    }
}

/// Appends the clustering plugin-arg tail exactly as the old `init()` did.
pub(crate) fn clustering_plugin_args(
    base: Option<String>,
    radius: Precision,
    min_points: usize,
    max_clusters: usize,
) -> String {
    let mut s = base.unwrap_or_default();
    s += &format!(" --radius {radius}");
    s += &format!(" --min_points {min_points}");
    s += &format!(" --max_clusters {max_clusters}");
    s
}

/// Appends the bootstrap plugin-arg tail exactly as the old `init()` did.
pub(crate) fn bootstrap_plugin_args(base: Option<String>, radius: Precision) -> String {
    let mut s = base.unwrap_or_default();
    s += &format!(" --radius {radius}");
    s
}


/// Resolve an optional [`DataPointsArg`] into a flat `[lat, lon]` list. A feature
/// with an unconvertible geometry yields no points (matches the old matrix's
/// empty arm). Parity with old `init()`.
// Consumed by the per-op request types (`ops.rs`).
pub(crate) fn resolve_data_points(data_points: Option<DataPointsArg>) -> koji_core::SingleVec {
    if let Some(data_points) = data_points {
        match data_points {
            // `Array` is already the `[lat, lon]` list. `Struct` is `PointStruct`s
            // — map each to `[lat, lon]` directly. `Feature`/`FeatureCollection`
            // go through the Phase 1 `TryFrom` then the Phase 1B inherent
            // `to_single_vec` (no `To*` matrix). A feature with an unconvertible
            // geometry yields no points, matching the old matrix's empty arm.
            DataPointsArg::Array(data_points) => data_points,
            DataPointsArg::Struct(data_points) => {
                data_points.into_iter().map(|p| [p.lat, p.lon]).collect()
            }
            DataPointsArg::Feature(feature) => match KojiGeometry::try_from(feature) {
                Ok(kg) => KojiGeometryCollection::new(vec![kg]).to_single_vec(),
                Err(_) => vec![],
            },
            DataPointsArg::FeatureCollection(fc) => match KojiGeometryCollection::try_from(fc) {
                Ok(coll) => coll.to_single_vec(),
                Err(_) => vec![],
            },
        }
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::requests::inputs::DataPointsArg;
    use koji_core::PointStruct;

    // ── resolve_max_clusters ─────────────────────────────────────────────────

    #[test]
    fn resolve_max_clusters_none_gives_usize_max() {
        assert_eq!(resolve_max_clusters(None), usize::MAX);
    }

    #[test]
    fn resolve_max_clusters_zero_gives_usize_max() {
        assert_eq!(resolve_max_clusters(Some(0)), usize::MAX);
    }

    #[test]
    fn resolve_max_clusters_positive_gives_value() {
        assert_eq!(resolve_max_clusters(Some(5)), 5);
        assert_eq!(resolve_max_clusters(Some(1)), 1);
    }

    // ── clustering_plugin_args ───────────────────────────────────────────────

    #[test]
    fn clustering_plugin_args_none_base_produces_tail() {
        let s = clustering_plugin_args(None, 70.0, 1, usize::MAX);
        assert!(s.starts_with(" --radius 70"));
        assert!(s.contains("--min_points 1"));
        assert!(s.contains("--max_clusters"));
    }

    #[test]
    fn clustering_plugin_args_some_base_prepends_it() {
        let s = clustering_plugin_args(Some("--custom arg".into()), 50.0, 3, 10);
        assert!(s.starts_with("--custom arg"));
        assert!(s.contains("--radius 50"));
        assert!(s.contains("--min_points 3"));
        assert!(s.contains("--max_clusters 10"));
    }

    // ── bootstrap_plugin_args ────────────────────────────────────────────────

    #[test]
    fn bootstrap_plugin_args_none_base() {
        let s = bootstrap_plugin_args(None, 70.0);
        assert_eq!(s, " --radius 70");
    }

    #[test]
    fn bootstrap_plugin_args_some_base_prepends() {
        let s = bootstrap_plugin_args(Some("--foo".into()), 30.0);
        assert!(s.starts_with("--foo"));
        assert!(s.contains("--radius 30"));
    }

    // ── resolve_data_points ──────────────────────────────────────────────────

    #[test]
    fn resolve_data_points_none_gives_empty() {
        assert!(resolve_data_points(None).is_empty());
    }

    #[test]
    fn resolve_data_points_array_passthrough() {
        let pts: koji_core::SingleVec = vec![[1.0, 2.0], [3.0, 4.0]];
        let result = resolve_data_points(Some(DataPointsArg::Array(pts.clone())));
        assert_eq!(result, pts);
    }

    #[test]
    fn resolve_data_points_struct_maps_lat_lon() {
        let structs: koji_core::SingleStruct = vec![
            PointStruct {
                lat: 10.0,
                lon: 20.0,
            },
            PointStruct {
                lat: 30.0,
                lon: 40.0,
            },
        ];
        let result = resolve_data_points(Some(DataPointsArg::Struct(structs)));
        assert_eq!(result, vec![[10.0, 20.0], [30.0, 40.0]]);
    }

    #[test]
    fn resolve_data_points_invalid_feature_gives_empty() {
        // A Feature with no geometry is unconvertible → empty, matching the old matrix.
        let feature = geojson::Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        };
        let result = resolve_data_points(Some(DataPointsArg::Feature(feature)));
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_data_points_empty_feature_collection_gives_empty() {
        let fc = geojson::FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        };
        let result = resolve_data_points(Some(DataPointsArg::FeatureCollection(fc)));
        // An empty FeatureCollection converts fine but has no points.
        assert!(result.is_empty());
    }
}
