use super::*;

#[derive(Debug, Serialize, Deserialize, Clone, FromQueryResult)]
pub struct PointStruct<T: Float = Precision> {
    pub lat: T,
    pub lon: T,
}
impl Default for PointStruct {
    fn default() -> PointStruct {
        PointStruct { lat: 0., lon: 0. }
    }
}

impl ToPointArray for PointStruct {
    fn to_point_array(self) -> point_array::PointArray {
        [self.lat, self.lon]
    }
}

impl ToSingleVec for PointStruct {
    fn to_single_vec(self) -> single_vec::SingleVec {
        vec![self.to_point_array()]
    }
}

impl ToMultiVec for PointStruct {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        vec![self.to_single_vec()]
    }
}

impl ToPointStruct for PointStruct {
    fn to_struct(self) -> PointStruct {
        self
    }
}

impl ToSingleStruct for PointStruct {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        vec![self.to_struct()]
    }
}

impl ToMultiStruct for PointStruct {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        vec![self.to_single_struct()]
    }
}

impl ToFeature for PointStruct {
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

impl ToCollection for PointStruct {
    fn to_collection(self, name: Option<String>, enum_type: Option<&Type>) -> FeatureCollection {
        let feature = self
            .to_feature(enum_type)
            .ensure_properties(name, enum_type);
        FeatureCollection {
            bbox: feature.bbox.clone(),
            features: vec![feature],
            foreign_members: None,
        }
    }
}

impl ToText for PointStruct {
    fn to_text(self, sep_1: &str, sep_2: &str, _poly_sep: bool) -> String {
        format!("{}{}{}{}", self.lat as f32, sep_1, self.lon as f32, sep_2)
    }
}

impl ToPoracle for PointStruct {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            path: Some(self.to_single_vec()),
            ..Default::default()
        }
    }
}
