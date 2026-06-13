use crate::TrimPrecision;

use super::*;

pub type SingleVec = Vec<point_array::PointArray>;

impl EnsurePoints for SingleVec {
    fn ensure_first_last(self) -> Self {
        if self.is_empty() {
            return self;
        }
        let mut points = self;

        if points[0] != points[points.len() - 1] {
            points.push(points[0]);
        }
        points
    }
}

impl GetBbox for SingleVec {
    /// \[min_lon, min_lat, max_lon, max_lat\]
    fn get_bbox(&self) -> Option<Vec<Precision>> {
        let mut bbox = if self.is_empty() {
            vec![0., 0., 0., 0.]
        } else {
            vec![
                Precision::INFINITY,
                Precision::INFINITY,
                Precision::NEG_INFINITY,
                Precision::NEG_INFINITY,
            ]
        };

        for point in self {
            if point[1] < bbox[0] {
                bbox[0] = point[1]
            }
            if point[1] > bbox[2] {
                bbox[2] = point[1]
            }
            if point[0] < bbox[1] {
                bbox[1] = point[0]
            }
            if point[0] > bbox[3] {
                bbox[3] = point[0]
            }
        }
        Some(bbox.into_iter().map(|e| e.trim_precision(6)).collect())
    }
}

impl ToPointArray for SingleVec {
    fn to_point_array(self) -> point_array::PointArray {
        self[0]
    }
}

impl ToSingleVec for SingleVec {
    fn to_single_vec(self) -> SingleVec {
        self.ensure_first_last()
    }
}

impl ToPointStruct for SingleVec {
    fn to_struct(self) -> point_struct::PointStruct {
        log::warn!(
            "`to_struct()` was called on a SingleVec and this was likely unintentional, did you mean to map over the values first?"
        );
        point_struct::PointStruct {
            lat: self[0][0],
            lon: self[1][1],
        }
    }
}

impl ToSingleStruct for SingleVec {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        self.ensure_first_last()
            .into_iter()
            .map(|each| each.to_struct())
            .collect()
    }
}

impl ToFeature for SingleVec {
    fn to_feature(self, ctx: &FeatureCtx) -> Feature {
        let bbox = self.get_bbox();
        Feature {
            bbox: bbox.clone(),
            geometry: Some(Geometry {
                bbox,
                foreign_members: None,
                value: if let Some(enum_type) = ctx.fence_type {
                    self.to_multi_vec().get_geojson_value(enum_type)
                } else {
                    self.to_multi_vec().polygon()
                },
            }),
            ..Feature::default()
        }
    }
}

impl ToCollection for SingleVec {
    fn to_collection(self, ctx: &FeatureCtx) -> FeatureCollection {
        if self.len() > 1 {
            FeatureCollection {
                bbox: self.get_bbox(),
                features: vec![
                    self.to_feature(ctx), // .ensure_properties(ctx)
                ],
                foreign_members: None,
            }
        } else {
            FeatureCollection::default()
        }
    }
}

impl ToText for SingleVec {
    fn to_text(self, sep_1: &str, sep_2: &str, poly_sep: bool) -> String {
        let last = if self.is_empty() { 0 } else { self.len() - 1 };
        self.into_iter()
            .enumerate()
            .map(|(i, each)| each.to_text(sep_1, if i == last { "" } else { sep_2 }, poly_sep))
            .collect()
    }
}

wrapper_conversions!(@wrappers SingleVec);
