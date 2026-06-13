//! Edge conversions between geojson and the KojiGeometry types. geo-types stays
//! canonical; geojson is an edge format only.

use geo::Geometry;

use super::{KojiGeometry, KojiGeometryCollection, KojiMeta};

/// Error when a geojson Feature has no geometry or a non-convertible one.
#[derive(Debug)]
pub enum KojiGeojsonError {
    MissingGeometry,
    Convert(geojson::Error),
}

impl std::fmt::Display for KojiGeojsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KojiGeojsonError::MissingGeometry => write!(f, "feature has no geometry"),
            KojiGeojsonError::Convert(e) => write!(f, "geojson->geo conversion failed: {e}"),
        }
    }
}
impl std::error::Error for KojiGeojsonError {}

impl TryFrom<geojson::Feature> for KojiGeometry {
    type Error = KojiGeojsonError;

    fn try_from(f: geojson::Feature) -> Result<Self, Self::Error> {
        let gj = f.geometry.ok_or(KojiGeojsonError::MissingGeometry)?;
        let geometry =
            Geometry::<f64>::try_from(&gj).map_err(KojiGeojsonError::Convert)?;

        // Properties (a serde_json object) deserialize directly into KojiMeta;
        // typed keys populate fields, the rest land in `extra`.
        let meta: KojiMeta = match f.properties {
            Some(props) => serde_json::from_value(serde_json::Value::Object(props))
                .unwrap_or_default(),
            None => KojiMeta::default(),
        };
        Ok(KojiGeometry { geometry, meta })
    }
}

impl TryFrom<geojson::FeatureCollection> for KojiGeometryCollection {
    type Error = KojiGeojsonError;

    fn try_from(fc: geojson::FeatureCollection) -> Result<Self, Self::Error> {
        let items = fc
            .features
            .into_iter()
            .map(KojiGeometry::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(KojiGeometryCollection::new(items))
    }
}

#[cfg(test)]
mod inbound_tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn feature_becomes_kojigeometry_with_meta() {
        let f: geojson::Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
            "properties": { "id": 5, "name": "x", "mode": "quest", "extra_key": 9 }
        }))
        .unwrap();

        let g = KojiGeometry::try_from(f).unwrap();
        assert!(matches!(g.geometry, geo::Geometry::Point(_)));
        assert_eq!(g.meta.id, Some(5));
        assert_eq!(g.meta.mode, Mode::Quest);
        assert_eq!(g.meta.extra.get("extra_key").unwrap(), 9);
    }

    #[test]
    fn featurecollection_becomes_collection() {
        let fc: geojson::FeatureCollection = serde_json::from_value(serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                { "type": "Feature", "geometry": {"type":"Point","coordinates":[0.0,0.0]}, "properties": null },
                { "type": "Feature", "geometry": {"type":"Point","coordinates":[1.0,1.0]}, "properties": null }
            ]
        }))
        .unwrap();

        let c = KojiGeometryCollection::try_from(fc).unwrap();
        assert_eq!(c.items.len(), 2);
    }
}
