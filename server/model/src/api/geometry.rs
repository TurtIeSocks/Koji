use geo::{MultiPolygon, Polygon, Simplify};

use super::*;

impl EnsurePoints for Geometry {
    fn ensure_first_last(self) -> Self {
        let mut return_value = self;
        match &mut return_value.value {
            Value::MultiPolygon(polygons) => {
                for polygon in polygons.into_iter() {
                    for line_string in polygon.into_iter() {
                        let last = line_string.last().unwrap();
                        if last[0] != line_string[0][0] && last[1] != line_string[0][1] {
                            line_string.push(line_string[0].clone())
                        }
                    }
                }
                return_value
            }
            Value::Polygon(poly) => {
                for line_string in poly {
                    let last = line_string.last().unwrap();
                    if last[0] != line_string[0][0] && last[1] != line_string[0][1] {
                        line_string.push(line_string[0].clone())
                    }
                }
                return_value
            }
            _ => return_value,
        }
    }
}

impl GeometryHelpers for Geometry {
    fn simplify(self) -> Self {
        match self.value {
            Value::Polygon(_) => {
                Geometry::from(&Polygon::<f64>::try_from(self).unwrap().simplify(&0.0001))
            }
            Value::MultiPolygon(_) => Geometry::from(
                &MultiPolygon::<f64>::try_from(self)
                    .unwrap()
                    .simplify(&0.0001),
            ),
            _ => self,
        }
    }
}

impl ToSingleVec for Geometry {
    fn to_single_vec(self) -> single_vec::SingleVec {
        let mut return_value = vec![];
        match self.value {
            // This should be unused now but leaving it since the work has been done
            Value::MultiPolygon(polygons) => {
                for polygon in polygons.into_iter() {
                    for line in polygon.into_iter() {
                        for point in line.into_iter() {
                            if point.len() == 2 {
                                return_value.push([point[1], point[0]]);
                            }
                        }
                    }
                }
            }
            Value::Polygon(geometry) => {
                for line in geometry.into_iter() {
                    for point in line.into_iter() {
                        if point.len() == 2 {
                            return_value.push([point[1], point[0]]);
                        }
                    }
                }
            }
            Value::MultiPoint(points) => {
                for point in points.into_iter() {
                    if point.len() == 2 {
                        return_value.push([point[1], point[0]]);
                    }
                }
            }
            Value::Point(point) => {
                if point.len() == 2 {
                    return_value.push([point[1], point[0]]);
                }
            }
            _ => {
                println!("Unsupported Geometry: {:?}", self.value.type_name())
            }
        }
        return_value
    }
}

impl ToFeature for Geometry {
    fn to_feature(self, _enum_type: Option<&Type>) -> Feature {
        Feature {
            bbox: self.clone().to_single_vec().get_bbox(),
            geometry: Some(self),
            ..Default::default()
        }
    }
}

impl ToFeatureVec for Geometry {
    fn to_feature_vec(self) -> Vec<Feature> {
        match self.value {
            Value::MultiPolygon(val) => val
                .into_iter()
                .map(|polygon| Feature {
                    bbox: polygon
                        .clone()
                        .into_iter()
                        .flat_map(|x| {
                            x.into_iter()
                                .map(|y| [y[0], y[1]])
                                .collect::<single_vec::SingleVec>()
                        })
                        .collect::<single_vec::SingleVec>()
                        .get_bbox(),
                    geometry: Some(Geometry {
                        bbox: None,
                        value: Value::Polygon(polygon),
                        foreign_members: None,
                    }),
                    ..Default::default()
                })
                .collect(),
            Value::GeometryCollection(val) => val
                .into_iter()
                .map(|geometry| geometry.to_feature(None))
                .collect(),
            _ => vec![self.to_feature(None)],
        }
    }
}
