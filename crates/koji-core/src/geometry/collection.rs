use geojson::{Feature, FeatureCollection};

use crate::TrimPrecision;

use super::*;

impl EnsurePoints for FeatureCollection {
    fn ensure_first_last(self) -> Self {
        self.into_iter()
            .map(|feat| feat.ensure_first_last())
            .collect()
    }
}

impl GeometryHelpers for FeatureCollection {
    fn simplify(self) -> Self {
        self.into_iter()
            .map(|feat| {
                if let Some(geometry) = feat.geometry {
                    Feature {
                        geometry: Some(geometry.simplify()),
                        ..feat
                    }
                } else {
                    feat
                }
            })
            .collect()
    }
}

impl TrimPrecision for FeatureCollection {
    fn trim_precision(self, precision: u32) -> Self {
        self.into_iter()
            .map(|feat| {
                if let Some(geometry) = feat.geometry {
                    Feature {
                        geometry: Some(geometry.trim_precision(precision)),
                        ..feat
                    }
                } else {
                    feat
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Feature, FeatureCollection, Geometry, GeometryValue};

    fn make_fc(features: Vec<Feature>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features,
            foreign_members: None,
        }
    }

    fn polygon_feature(ring: Vec<geojson::Position>) -> Feature {
        Feature {
            bbox: None,
            geometry: Some(Geometry::new(GeometryValue::Polygon {
                coordinates: vec![ring],
            })),
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    fn no_geom_feature() -> Feature {
        Feature {
            bbox: None,
            geometry: None,
            id: None,
            properties: None,
            foreign_members: None,
        }
    }

    // ── EnsurePoints for FeatureCollection ──────────────────────────────────

    #[test]
    fn ensure_first_last_closes_open_ring_in_feature() {
        let ring = vec![
            geojson::Position::from([0.0, 0.0]),
            geojson::Position::from([1.0, 0.0]),
            geojson::Position::from([1.0, 1.0]),
        ];
        let fc = make_fc(vec![polygon_feature(ring)]);
        let closed = fc.ensure_first_last();
        let ring_out = match &closed.features[0].geometry.as_ref().unwrap().value {
            GeometryValue::Polygon { coordinates: rings } => rings[0].clone(),
            _ => panic!("expected Polygon"),
        };
        assert_eq!(
            ring_out[0],
            ring_out[ring_out.len() - 1],
            "ring must be closed"
        );
    }

    #[test]
    fn ensure_first_last_feature_without_geometry_preserved() {
        let fc = make_fc(vec![no_geom_feature()]);
        let out = fc.ensure_first_last();
        assert!(out.features[0].geometry.is_none());
    }

    #[test]
    fn ensure_first_last_empty_collection() {
        let fc = make_fc(vec![]);
        let out = fc.ensure_first_last();
        assert!(out.features.is_empty());
    }

    // ── GeometryHelpers::simplify for FeatureCollection ─────────────────────

    #[test]
    fn simplify_reduces_collinear_points() {
        let ring = vec![
            geojson::Position::from([0.0, 0.0]),
            geojson::Position::from([1.0, 0.00001]),
            geojson::Position::from([2.0, 0.0]),
            geojson::Position::from([2.0, 2.0]),
            geojson::Position::from([0.0, 2.0]),
            geojson::Position::from([0.0, 0.0]),
        ];
        let fc = make_fc(vec![polygon_feature(ring)]);
        let simplified = fc.simplify();
        let ring_out = match &simplified.features[0].geometry.as_ref().unwrap().value {
            GeometryValue::Polygon { coordinates: rings } => rings[0].clone(),
            _ => panic!("expected Polygon"),
        };
        assert!(
            ring_out.len() < 6,
            "expected simplification to reduce points"
        );
    }

    #[test]
    fn simplify_preserves_feature_without_geometry() {
        let fc = make_fc(vec![no_geom_feature(), no_geom_feature()]);
        let out = fc.simplify();
        assert_eq!(out.features.len(), 2);
        assert!(out.features[0].geometry.is_none());
    }

    // ── TrimPrecision for FeatureCollection ──────────────────────────────────

    #[test]
    fn trim_precision_rounds_polygon_coords() {
        let ring = vec![geojson::Position::from([1.23456789f64, 9.87654321])];
        let fc = make_fc(vec![polygon_feature(ring)]);
        let trimmed = fc.trim_precision(4);
        let ring_out = match &trimmed.features[0].geometry.as_ref().unwrap().value {
            GeometryValue::Polygon { coordinates: rings } => rings[0].clone(),
            _ => panic!("expected Polygon"),
        };
        assert_eq!(ring_out[0][0], 1.2346);
        assert_eq!(ring_out[0][1], 9.8765);
    }

    #[test]
    fn trim_precision_preserves_no_geom_feature() {
        let fc = make_fc(vec![no_geom_feature()]);
        let out = fc.trim_precision(6);
        assert!(out.features[0].geometry.is_none());
    }
}
