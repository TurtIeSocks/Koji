use super::*;

impl FeatureHelpers for Feature {
    fn add_instance_properties(&mut self, name: Option<String>, enum_type: Option<&Type>) {
        if let Some(name) = name {
            self.set_property("name", name)
        }
        if let Some(enum_type) = enum_type {
            self.set_property("type", enum_type.to_string());
            // match enum_type {
            //     Type::CirclePokemon | Type::CircleSmartPokemon => {
            //         self.set_property("radius", 70);
            //     }
            //     Type::CircleRaid | Type::CircleSmartRaid => {
            //         self.set_property("radius", 700);
            //     }
            //     Type::ManualQuest => {
            //         self.set_property("radius", 80);
            //     }
            //     _ => {}
            // }
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
                Value::MultiPolygon(_) => geometry
                    .to_feature_vec()
                    .into_iter()
                    .for_each(|f| return_value.push(f.to_single_vec())),
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
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        self.to_multi_vec().to_text(sep_1, sep_2)
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
    fn to_collection(self, _enum_type: Option<&Type>) -> FeatureCollection {
        let bbox = if self.bbox.is_some() {
            self.bbox
        } else {
            self.clone().to_single_vec().get_bbox()
        };
        FeatureCollection {
            bbox: bbox.clone(),
            features: vec![Feature { bbox, ..self }],
            foreign_members: None,
        }
    }
}

impl ToCollection for Vec<Feature> {
    fn to_collection(self, _enum_type: Option<&Type>) -> FeatureCollection {
        FeatureCollection {
            bbox: self
                .clone()
                .into_iter()
                .flat_map(|feat| feat.to_single_vec())
                .collect::<single_vec::SingleVec>()
                .get_bbox(),
            features: self,
            foreign_members: None,
        }
    }
}
