use super::*;

pub type PointArray = [Precision; 2];

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

impl ToPointStruct for PointArray {
    fn to_struct(self) -> point_struct::PointStruct {
        self.into()
    }
}

impl ToSingleStruct for PointArray {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        vec![self.to_struct()]
    }
}

impl ToFeature for PointArray {
    fn to_feature(self, ctx: &FeatureCtx) -> Feature {
        let bbox = self.to_single_vec().get_bbox();
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

impl ToText for PointArray {
    fn to_text(self, sep_1: &str, sep_2: &str, _poly_sep: bool) -> String {
        format!("{}{}{}{}", self[0], sep_1, self[1], sep_2)
    }
}

wrapper_conversions!(PointArray);
