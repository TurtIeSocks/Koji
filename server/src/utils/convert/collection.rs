use super::*;

pub trait Default {
    fn default() -> Self;
}

impl Default for FeatureCollection {
    fn default() -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            foreign_members: None,
            features: vec![],
        }
    }
}

pub fn from_feature(feature: Feature) -> FeatureCollection {
    FeatureCollection {
        bbox: None,
        foreign_members: None,
        features: vec![feature],
    }
}

pub fn from_features(features: Vec<Feature>) -> FeatureCollection {
    FeatureCollection {
        bbox: None,
        foreign_members: None,
        features,
    }
}
