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
