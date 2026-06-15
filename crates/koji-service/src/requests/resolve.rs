//! Shared resolution helpers + default constants used by the arg-groups.
//!
//! These mirror the legacy fat-`Args` `init()` exactly (the parity contract is
//! the spec §2 defaults table). `validate_s2_cell` and `resolve_data_points` are
//! the single source of truth for the v2 arg-groups.

use super::inputs::DataPointsArg;
use koji_core::{KojiGeometry, KojiGeometryCollection};

pub(crate) const DEFAULT_RADIUS: f64 = 70.0;
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
    radius: f64,
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
pub(crate) fn bootstrap_plugin_args(base: Option<String>, radius: f64) -> String {
    let mut s = base.unwrap_or_default();
    s += &format!(" --radius {radius}");
    s
}

/// Validate an S2 split-cell level. `None` → 0; out-of-range (not 0–20) warns and
/// defaults to 0; otherwise the value. Parity with old `init()`.
pub(crate) fn validate_s2_cell(value_to_check: Option<u64>, label: &str) -> u64 {
    if let Some(cell_level) = value_to_check {
        if cell_level.le(&20) && cell_level.ge(&0) {
            cell_level
        } else {
            log::warn!(
                "{} only supports 0-20, {} was provided, defaulting to 0",
                label,
                cell_level
            );
            0
        }
    } else {
        0
    }
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
