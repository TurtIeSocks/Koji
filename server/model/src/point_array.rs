use super::*;

pub type PointArray<T = Precision> = [T; 2];

impl ToPointArray for PointArray {
    fn to_point_array(self) -> PointArray {
        self
    }
}

impl ToSingleVec for PointArray {
    fn to_single_vec(self) -> single_vec::SingleVec {
        vec![self]
    }
}

impl ToMultiVec for PointArray {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        vec![self.to_single_vec()]
    }
}

impl ToPointStruct for PointArray {
    fn to_struct(self) -> point_struct::PointStruct {
        point_struct::PointStruct {
            lat: self[0],
            lon: self[1],
        }
    }
}

impl ToSingleStruct for PointArray {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        vec![self.to_struct()]
    }
}

impl ToMultiStruct for PointArray {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        vec![self.to_single_struct()]
    }
}

impl ToFeature for PointArray {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        Feature {
            bbox: self.clone().to_single_vec().get_bbox(),
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    self.to_multi_vec().get_geojson_value(enum_type)
                } else {
                    self.to_multi_vec().point()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToCollection for PointArray {
    fn to_collection(self, enum_type: Option<&Type>) -> FeatureCollection {
        let feature = self.to_feature(enum_type);
        FeatureCollection {
            bbox: feature.bbox.clone(),
            features: vec![feature],
            foreign_members: None,
        }
    }
}

impl ToText for PointArray {
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        format!("{}{}{}{}", self[0], sep_1, self[1], sep_2)
    }
}

impl ToPoracle for PointArray {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            path: Some(self.to_single_vec()),
            ..Default::default()
        }
    }
}
