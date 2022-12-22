use super::*;

pub type MultiStruct<T = Precision> = Vec<Vec<point_struct::PointStruct<T>>>;

impl ToPointArray for MultiStruct {
    fn to_point_array(self) -> point_array::PointArray {
        [self[0][0].lat, self[0][0].lon]
    }
}

impl ToSingleVec for MultiStruct {
    fn to_single_vec(self) -> single_vec::SingleVec {
        self.into_iter()
            .map(|point| point.to_point_array())
            .collect()
    }
}

impl ToMultiVec for MultiStruct {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        self.into_iter()
            .map(|point| point.to_single_vec())
            .collect()
    }
}

impl ToPointStruct for MultiStruct {
    fn to_struct(self) -> point_struct::PointStruct {
        println!("`to_struct()` was called on a SingleVec and this was likely unintentional, did you mean to map over the values first?");
        point_struct::PointStruct {
            lat: self[0][0].lat,
            lon: self[0][0].lon,
        }
    }
}

impl ToSingleStruct for MultiStruct {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        self.into_iter().flatten().collect()
    }
}

impl ToMultiStruct for MultiStruct {
    fn to_multi_struct(self) -> MultiStruct {
        self
    }
}

impl ToFeature for MultiStruct {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        Feature {
            geometry: Some(Geometry {
                bbox: self.clone().to_single_vec().get_bbox(),
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    self.to_multi_vec().get_geojson_value(enum_type)
                } else {
                    self.to_multi_vec().multi_polygon()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToCollection for MultiStruct {
    fn to_collection(self, enum_type: Option<&Type>) -> FeatureCollection {
        let feature = self.to_feature(enum_type);
        FeatureCollection {
            bbox: feature.bbox.clone(),
            features: vec![feature],
            foreign_members: None,
        }
    }
}

impl ToText for MultiStruct {
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        self.into_iter()
            .map(|each| each.to_text(sep_1, sep_2))
            .collect()
    }
}

impl ToPoracle for MultiStruct {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            multipath: Some(self.to_multi_vec()),
            ..Default::default()
        }
    }
}
