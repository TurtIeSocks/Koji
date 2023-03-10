use geo::algorithm::RemoveRepeatedPoints;
// use geo_repair::repair::Repair;
use geo_types::MultiPoint;
use sea_orm::ActiveEnum;

use super::*;

impl EnsurePoints for Feature {
    fn ensure_first_last(self) -> Self {
        let geometry = if let Some(geometry) = self.geometry {
            Some(geometry.ensure_first_last())
        } else {
            None
        };
        Self { geometry, ..self }
    }
}

impl ToGeometry for Feature {
    fn to_geometry(self) -> Geometry {
        if let Some(geometry) = self.geometry {
            geometry
        } else {
            Geometry {
                bbox: None,
                foreign_members: None,
                value: Value::Point(vec![0., 0.]),
            }
        }
    }
}

impl FeatureHelpers for Feature {
    fn add_instance_properties(&mut self, name: Option<String>, enum_type: Option<Type>) {
        if !self.contains_property("__name") {
            if let Some(name) = name {
                self.set_property("__name", name)
            }
        }
        if !self.contains_property("__mode") {
            if let Some(enum_type) = enum_type {
                self.set_property("__mode", enum_type.to_value());
            } else if let Some(geometry) = self.geometry.as_ref() {
                match geometry.value {
                    Value::Point(_) | Value::MultiPoint(_) => {
                        self.set_property("__mode", "circle_pokemon");
                    }
                    Value::Polygon(_) | Value::MultiPolygon(_) => {
                        self.set_property("__mode", "auto_quest");
                    }
                    _ => {}
                }
            }
        }
    }
    /// Removes the last point if it matches the first point in a multipoint feature
    fn remove_last_coord(self) -> Self {
        if let Some(geometry) = self.geometry {
            let geometry = match MultiPoint::<f64>::try_from(&geometry) {
                Ok(mp) => {
                    let mp = mp.remove_repeated_points();
                    let new_value = mp.into_iter().map(|p| vec![p.x(), p.y()]).collect();
                    Geometry {
                        value: Value::MultiPoint(new_value),
                        ..geometry
                    }
                }
                Err(_) => geometry,
            };
            Self {
                geometry: Some(geometry),
                ..self
            }
        } else {
            self
        }
    }
    /// Removes internally used properties that start with `__`
    fn remove_internal_props(self) -> Self {
        let mut mutable_self = self.to_owned();
        mutable_self.id = None;
        mutable_self.properties = Some(
            self.properties_iter()
                .filter_map(|(key, val)| {
                    if key.starts_with("__") {
                        None
                    } else {
                        Some((key.to_owned(), val.to_owned()))
                    }
                })
                .collect(),
        );
        mutable_self
    }
}

impl EnsureProperties for Feature {
    fn ensure_properties(self, name: Option<String>, enum_type: Option<Type>) -> Self {
        let mut mutable_self = self;
        mutable_self.add_instance_properties(name, enum_type);
        mutable_self
    }
}

impl GetBbox for Feature {
    fn get_bbox(&self) -> Option<Bbox> {
        if let Some(geometry) = self.geometry.clone() {
            geometry.to_single_vec().get_bbox()
        } else {
            None
        }
    }
}

impl ToSingleVec for Feature {
    fn to_single_vec(self) -> single_vec::SingleVec {
        self.to_multi_vec().into_iter().flatten().collect()
    }
}

impl ToMultiVec for Feature {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        let mut return_value = vec![];
        if let Some(geometry) = self.geometry {
            match geometry.value {
                Value::MultiPolygon(_) => {
                    // let mp = MultiPolygon::<Precision>::try_from(geometry.clone()).unwrap();
                    // let repaired = mp.repair();
                    geometry
                        .to_feature_vec()
                        .into_iter()
                        .for_each(|f| return_value.push(f.to_single_vec()))

                    // if let Some(repaired) = repaired {
                    //     let local: single_vec::SingleVec = repaired
                    //         .exterior_coords_iter()
                    //         .map(|coord| [coord.y as Precision, coord.x as Precision])
                    //         .collect();
                    //     log::info!("Repaired a Polygon");
                    //     return_value.push(local);
                    // } else {
                    //     geometry
                    //         .to_feature_vec()
                    //         .into_iter()
                    //         .for_each(|f| return_value.push(f.to_single_vec()))
                    // }
                }
                Value::GeometryCollection(geometries) => geometries.into_iter().for_each(|g| {
                    let value = g.to_single_vec();
                    if !value.is_empty() {
                        return_value.push(value)
                    }
                }),
                _ => return_value.push(geometry.to_single_vec()),
            }
        }

        return_value
    }
}

impl ToText for Feature {
    fn to_text(self, sep_1: &str, sep_2: &str, poly_sep: bool) -> String {
        self.to_multi_vec().to_text(sep_1, sep_2, poly_sep)
    }
}

impl ToFeatureVec for Feature {
    fn to_feature_vec(self) -> Vec<Feature> {
        if let Some(geometry) = self.geometry {
            geometry.to_feature_vec()
        } else {
            vec![self]
        }
    }
}

impl ToCollection for Feature {
    fn to_collection(self, _name: Option<String>, _enum_type: Option<Type>) -> FeatureCollection {
        let bbox = self.get_bbox();
        FeatureCollection {
            bbox: bbox.clone(),
            features: vec![Feature { bbox, ..self }.ensure_first_last()],
            foreign_members: None,
        }
    }
}

impl GetBbox for Vec<Feature> {
    fn get_bbox(&self) -> Option<Bbox> {
        self.clone()
            .into_iter()
            .flat_map(|f| f.to_single_vec())
            .collect::<single_vec::SingleVec>()
            .get_bbox()
    }
}

impl ToCollection for Vec<Feature> {
    fn to_collection(self, _name: Option<String>, _enum_type: Option<Type>) -> FeatureCollection {
        // let name = if let Some(name) = name {
        //     name
        // } else {
        //     "".to_string()
        // };
        // let length = self.len();
        FeatureCollection {
            bbox: self.get_bbox(),
            features: self
                .into_iter()
                .map(|feat| Feature {
                    bbox: feat.get_bbox(),
                    ..feat.ensure_first_last()
                })
                .collect(),
            foreign_members: None,
        }
    }
}
