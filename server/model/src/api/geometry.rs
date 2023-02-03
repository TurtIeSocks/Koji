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
        let mut geometry = match self.value {
            Value::Polygon(_) => {
                Geometry::from(&Polygon::<f64>::try_from(self).unwrap().simplify(&0.0001))
            }
            Value::MultiPolygon(_) => Geometry::from(
                &MultiPolygon::<f64>::try_from(self)
                    .unwrap()
                    .simplify(&0.0001),
            ),
            _ => self,
        };
        geometry.bbox = geometry.get_bbox();
        geometry
    }
}

impl GetBbox for Geometry {
    fn get_bbox(&self) -> Option<Bbox> {
        self.clone().to_single_vec().get_bbox()
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
                log::warn!("Unsupported Geometry: {:?}", self.value.type_name())
            }
        }
        return_value
    }
}

impl ToFeature for Geometry {
    fn to_feature(self, enum_type: Option<Type>) -> Feature {
        let bbox = self.get_bbox();
        Feature {
            bbox: bbox.clone(),
            geometry: Some(Self {
                bbox,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    match enum_type {
                        Type::Leveling => {
                            Value::Point(self.to_single_vec().to_point_array().to_vec())
                        }
                        Type::CirclePokemon => Value::MultiPoint(
                            self.to_single_vec()
                                .into_iter()
                                .map(|s_vec| vec![s_vec[1], s_vec[0]])
                                .collect(),
                        ),
                        Type::AutoQuest => Value::MultiPolygon(match self.value {
                            Value::Polygon(geometry) => vec![geometry],
                            Value::MultiPolygon(polygons) => polygons,
                            Value::Point(point) => vec![vec![vec![point]]],
                            Value::MultiPoint(points) => vec![vec![points]],
                            Value::LineString(line) => vec![vec![line]],
                            Value::MultiLineString(lines) => vec![lines],
                            Value::GeometryCollection(_) => {
                                log::error!("Geometry Collections are not currently supported");
                                vec![vec![vec![vec![]]]]
                            }
                        }),
                        _ => self.value,
                    }
                } else {
                    Value::Polygon(match self.value {
                        Value::Polygon(geometry) => geometry,
                        Value::MultiPolygon(polygons) => polygons.into_iter().flatten().collect(),
                        Value::Point(point) => vec![vec![point]],
                        Value::MultiPoint(points) => vec![points],
                        Value::LineString(line) => vec![line],
                        Value::MultiLineString(lines) => lines,
                        Value::GeometryCollection(_) => {
                            log::error!("Geometry Collections are not currently supported");
                            vec![vec![vec![]]]
                        }
                    })
                },
            }),
            ..Default::default()
        }
    }
}

impl ToFeatureVec for Geometry {
    fn to_feature_vec(self) -> Vec<Feature> {
        match self.value {
            Value::MultiPolygon(val) => val
                .into_iter()
                .map(|polygon| {
                    let bbox = polygon
                        .clone()
                        .into_iter()
                        .flat_map(|x| {
                            x.into_iter()
                                .map(|y| [y[0] as Precision, y[1] as Precision])
                                .collect::<single_vec::SingleVec>()
                        })
                        .collect::<single_vec::SingleVec>()
                        .get_bbox();
                    Feature {
                        bbox: bbox.clone(),
                        geometry: Some(Geometry {
                            bbox,
                            value: Value::Polygon(polygon),
                            foreign_members: None,
                        }),
                        ..Default::default()
                    }
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

impl ToCollection for Vec<Geometry> {
    fn to_collection(self, _name: Option<String>, enum_type: Option<Type>) -> FeatureCollection {
        FeatureCollection {
            bbox: self
                .clone()
                .into_iter()
                .map(|geom| geom.to_single_vec())
                .flatten()
                .collect::<single_vec::SingleVec>()
                .get_bbox(),
            foreign_members: None,
            features: self
                .into_iter()
                .map(|geometry| geometry.to_feature(enum_type.clone()))
                .collect(),
        }
    }
}
