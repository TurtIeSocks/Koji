use super::*;

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
