//! Inbound wire inputs for the v2 request surface: the accepted `area` geometry
//! shapes ([`GeoInput`]) and the data-point list shapes ([`DataPointsArg`]).
//!
//! The canonical home for these input enums (the v2 per-op requests use them).
//! The conversions are geojson-only — no direct `geo` dependency.

use geojson::{Feature, FeatureCollection, Geometry};
use koji_core::{KojiGeometry, KojiGeometryCollection};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DataPointsArg {
    Array(koji_core::SingleVec),
    Struct(koji_core::SingleStruct),
    Feature(Feature),
    FeatureCollection(FeatureCollection),
}

/// Accepted inbound geometry shapes for the request `area`. geojson-only:
/// the bare-array wire forms (`[Feature]`, `[Geometry]`, raw point arrays,
/// poracle, bbox, text) were dropped in Phase 2 — clients send a
/// `FeatureCollection`, a `Feature`, or a `GeometryCollection`.
///
/// Untagged: serde picks the first variant that deserializes. `FeatureCollection`
/// leads (most specific — requires `type: "FeatureCollection"`), then `Feature`,
/// then a bare `Geometry`/`GeometryCollection`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeoInput {
    FeatureCollection(FeatureCollection),
    Feature(Feature),
    Geometry(Geometry),
}

impl GeoInput {
    /// Normalize the inbound geojson into a [`KojiGeometryCollection`] via the
    /// Phase 1 `TryFrom`. A `GeometryCollection` fans out into one item per
    /// child geometry (property-less, default meta) — matching the outbound
    /// `From<&KojiGeometryCollection> for geojson::Geometry`.
    // `KojiGeojsonError` is the shared edge-conversion error; the codebase
    // carries it by value (see `Model::to_koji_geometry`), so allow the lint
    // here too rather than diverging the signature with a Box.
    #[allow(clippy::result_large_err)]
    pub fn to_koji(&self) -> Result<KojiGeometryCollection, koji_core::KojiGeojsonError> {
        match self {
            GeoInput::FeatureCollection(fc) => KojiGeometryCollection::try_from(fc.clone()),
            GeoInput::Feature(f) => Ok(KojiGeometryCollection::new(vec![KojiGeometry::try_from(
                f.clone(),
            )?])),
            GeoInput::Geometry(g) => geometry_to_koji(g),
        }
    }

    /// Normalize the inbound geojson into the algorithm-edge `FeatureCollection`,
    /// lenient like the legacy `init()`: a geometry that fails to normalize logs a
    /// warning and yields an empty collection.
    pub fn to_feature_collection(&self) -> FeatureCollection {
        match self.to_koji() {
            Ok(coll) => FeatureCollection::from(&coll),
            Err(err) => {
                log::warn!("[AREA] failed to normalize inbound geometry: {err}");
                FeatureCollection::default()
            }
        }
    }
}

/// Normalize an optional request `area` into a `FeatureCollection` — lenient (no
/// area → empty collection; a bad geometry logs and yields empty). The shared
/// parity-faithful `area` resolution for the geo handlers + the v2 calc enqueue.
pub fn area_collection(area: &Option<GeoInput>) -> FeatureCollection {
    area.as_ref()
        .map(GeoInput::to_feature_collection)
        .unwrap_or_default()
}

/// Convert a bare geojson `Geometry` into a collection. A `GeometryCollection`
/// becomes one [`KojiGeometry`] per child; any other geometry becomes a single
/// item. Both paths carry default (property-less) metadata. Each child is
/// wrapped in a property-less `Feature` and routed through the Phase 1
/// `TryFrom` so this layer needs no direct `geo` dependency.
#[allow(clippy::result_large_err)]
fn geometry_to_koji(g: &Geometry) -> Result<KojiGeometryCollection, koji_core::KojiGeojsonError> {
    let children: Vec<Geometry> = match &g.value {
        geojson::Value::GeometryCollection(geometries) => geometries.clone(),
        _ => vec![g.clone()],
    };
    let items = children
        .into_iter()
        .map(|geometry| {
            KojiGeometry::try_from(Feature {
                bbox: None,
                geometry: Some(geometry),
                id: None,
                properties: None,
                foreign_members: None,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(KojiGeometryCollection::new(items))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn polygon_geometry() -> Geometry {
        Geometry::new(geojson::Value::Polygon(vec![vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![1.0, 1.0],
            vec![0.0, 1.0],
            vec![0.0, 0.0],
        ]]))
    }

    fn polygon_feature() -> Feature {
        Feature {
            bbox: None,
            geometry: Some(polygon_geometry()),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    fn polygon_fc() -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: vec![polygon_feature()],
            foreign_members: None,
        }
    }

    // ── GeoInput::to_koji ────────────────────────────────────────────────────

    #[test]
    fn geo_input_feature_collection_converts() {
        let gi = GeoInput::FeatureCollection(polygon_fc());
        assert!(gi.to_koji().is_ok());
    }

    #[test]
    fn geo_input_feature_converts() {
        let gi = GeoInput::Feature(polygon_feature());
        assert!(gi.to_koji().is_ok());
    }

    #[test]
    fn geo_input_geometry_converts() {
        let gi = GeoInput::Geometry(polygon_geometry());
        assert!(gi.to_koji().is_ok());
    }

    #[test]
    fn geo_input_feature_no_geometry_fails() {
        let gi = GeoInput::Feature(Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        });
        assert!(gi.to_koji().is_err());
    }

    // ── GeoInput::to_feature_collection ─────────────────────────────────────

    #[test]
    fn to_feature_collection_returns_empty_on_bad_geometry() {
        let gi = GeoInput::Feature(Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        });
        let fc = gi.to_feature_collection();
        // The lenient path: a failure logs a warning and yields an empty collection.
        assert!(fc.features.is_empty());
    }

    #[test]
    fn to_feature_collection_round_trips_polygon() {
        let gi = GeoInput::Feature(polygon_feature());
        let fc = gi.to_feature_collection();
        assert!(!fc.features.is_empty());
    }

    // ── area_collection ──────────────────────────────────────────────────────

    #[test]
    fn area_collection_none_gives_empty() {
        let fc = area_collection(&None);
        assert!(fc.features.is_empty());
    }

    #[test]
    fn area_collection_some_polygon_gives_feature() {
        let gi = GeoInput::Feature(polygon_feature());
        let fc = area_collection(&Some(gi));
        assert!(!fc.features.is_empty());
    }

    // ── GeoInput serde (untagged order: FC > Feature > Geometry) ────────────

    #[test]
    fn geo_input_deserializes_feature_collection_first() {
        let json = r#"{"type":"FeatureCollection","features":[]}"#;
        let gi: GeoInput = serde_json::from_str(json).unwrap();
        assert!(matches!(gi, GeoInput::FeatureCollection(_)));
    }

    #[test]
    fn geo_input_deserializes_feature() {
        let json = r#"{"type":"Feature","geometry":null,"properties":null}"#;
        let gi: GeoInput = serde_json::from_str(json).unwrap();
        assert!(matches!(gi, GeoInput::Feature(_)));
    }

    #[test]
    fn geo_input_deserializes_geometry() {
        let json = r#"{"type":"Point","coordinates":[1.0,2.0]}"#;
        let gi: GeoInput = serde_json::from_str(json).unwrap();
        assert!(matches!(gi, GeoInput::Geometry(_)));
    }

    // ── geometry_to_koji (GeometryCollection fans out) ───────────────────────

    #[test]
    fn geometry_to_koji_geometry_collection_fans_out() {
        let gc_json = r#"{
            "type": "GeometryCollection",
            "geometries": [
                {"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,1],[0,0]]]},
                {"type":"Polygon","coordinates":[[[2,2],[3,2],[3,3],[2,3],[2,2]]]}
            ]
        }"#;
        let geom: Geometry = serde_json::from_str(gc_json).unwrap();
        let gi = GeoInput::Geometry(geom);
        let coll = gi.to_koji().unwrap();
        // Both children become separate KojiGeometry entries.
        assert_eq!(coll.items.len(), 2);
    }
}
