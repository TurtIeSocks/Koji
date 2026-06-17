//! Edge conversions between geojson and the KojiGeometry types. geo-types stays
//! canonical; geojson is an edge format only.

use geo::Geometry;
use crate::Precision;

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
        let geometry = Geometry::<Precision>::try_from(&gj).map_err(KojiGeojsonError::Convert)?;

        // Properties (a serde_json object) deserialize directly into KojiMeta;
        // typed keys populate fields, the rest land in `extra`.
        let meta: KojiMeta = match f.properties {
            Some(props) => {
                serde_json::from_value(serde_json::Value::Object(props)).unwrap_or_default()
            }
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

impl From<&KojiGeometry> for geojson::Feature {
    fn from(g: &KojiGeometry) -> Self {
        let value = geojson::Value::from(&g.geometry);
        let properties = match serde_json::to_value(&g.meta) {
            Ok(serde_json::Value::Object(map)) if !map.is_empty() => Some(map),
            _ => None,
        };
        geojson::Feature {
            bbox: None,
            geometry: Some(geojson::Geometry::new(value)),
            id: g.meta.id.map(|i| geojson::feature::Id::Number(i.into())),
            properties,
            foreign_members: None,
        }
    }
}

impl From<&KojiGeometryCollection> for geojson::FeatureCollection {
    fn from(c: &KojiGeometryCollection) -> Self {
        geojson::FeatureCollection {
            bbox: None,
            features: c.items.iter().map(geojson::Feature::from).collect(),
            foreign_members: None,
        }
    }
}

impl From<&KojiGeometryCollection> for geojson::Geometry {
    /// Property-less `GeometryCollection` output (replaces the old bare
    /// `[Geometry]` array). Metadata is intentionally dropped — this is the
    /// property-less multi-geometry format.
    fn from(c: &KojiGeometryCollection) -> Self {
        let geometries = c
            .items
            .iter()
            .map(|g| geojson::Geometry::new(geojson::Value::from(&g.geometry)))
            .collect();
        geojson::Geometry::new(geojson::Value::GeometryCollection(geometries))
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

#[cfg(test)]
mod outbound_tests {
    use super::*;
    use crate::{KojiMeta, Mode};
    use geo::Point;

    fn sample() -> KojiGeometry {
        KojiGeometry::new(Point::new(1.0, 2.0)).with_meta(KojiMeta {
            id: Some(5),
            mode: Mode::Quest,
            ..Default::default()
        })
    }

    #[test]
    fn element_to_feature_carries_props() {
        let f = geojson::Feature::from(&sample());
        assert!(f.geometry.is_some());
        let props = f.properties.unwrap();
        assert_eq!(props.get("id").unwrap(), 5);
        assert_eq!(props.get("mode").unwrap(), "quest");
    }

    #[test]
    fn collection_to_featurecollection() {
        let c = KojiGeometryCollection::new(vec![sample(), sample()]);
        let fc = geojson::FeatureCollection::from(&c);
        assert_eq!(fc.features.len(), 2);
    }

    #[test]
    fn collection_to_geometrycollection_is_property_less() {
        let c = KojiGeometryCollection::new(vec![sample(), sample()]);
        let g = geojson::Geometry::from(&c);
        assert!(matches!(g.value, geojson::Value::GeometryCollection(ref v) if v.len() == 2));
    }

    #[test]
    fn feature_roundtrip_is_idempotent() {
        let f0 = geojson::Feature::from(&sample());
        let g = KojiGeometry::try_from(f0.clone()).unwrap();
        let f1 = geojson::Feature::from(&g);
        assert_eq!(f0.geometry, f1.geometry);
        assert_eq!(f0.properties, f1.properties);
    }
}
