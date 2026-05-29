use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PointStruct {
    pub lat: Precision,
    pub lon: Precision,
}
impl Default for PointStruct {
    fn default() -> PointStruct {
        PointStruct { lat: 0., lon: 0. }
    }
}

impl From<PointArray> for PointStruct {
    fn from(p: PointArray) -> Self {
        PointStruct {
            lat: p[0],
            lon: p[1],
        }
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

impl ToFeature for PointStruct {
    fn to_feature(self, ctx: &FeatureCtx) -> Feature {
        let bbox = self.clone().to_single_vec().get_bbox();
        Feature {
            bbox: bbox.clone(),
            geometry: Some(Geometry {
                bbox,
                foreign_members: None,
                value: if let Some(enum_type) = ctx.fence_type {
                    self.to_multi_vec().get_geojson_value(enum_type)
                } else {
                    self.to_multi_vec().point()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToText for PointStruct {
    fn to_text(self, sep_1: &str, sep_2: &str, _poly_sep: bool) -> String {
        format!("{}{}{}{}", self.lat, sep_1, self.lon, sep_2)
    }
}

wrapper_conversions!(PointStruct);
