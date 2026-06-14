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
