use super::*;

use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Poracle {
    pub id: Option<u64>,
    pub name: Option<String>,
    pub color: Option<String>,
    pub group: Option<String>,
    pub description: Option<String>,
    pub user_selectable: Option<bool>,
    pub display_in_matches: Option<bool>,
    pub path: Option<single_vec::SingleVec>,
    pub multipath: Option<multi_vec::MultiVec>,
}

impl Default for Poracle {
    fn default() -> Poracle {
        Poracle {
            id: Some(0),
            name: Some("".to_string()),
            color: None,
            group: None,
            description: None,
            user_selectable: None,
            display_in_matches: None,
            path: Some(vec![]),
            multipath: None,
        }
    }
}

// impl Poracle {
//     fn multipath_to_path(&mut self) {
//         self.path = Some(self.clone().to_single_vec());
//     }
//     fn path_to_multipath(&mut self) {
//         self.multipath = Some(self.clone().to_multi_vec())
//     }
// }

impl ToPointArray for Poracle {
    fn to_point_array(self) -> point_array::PointArray {
        if let Some(multipath) = self.multipath {
            multipath[0][0]
        } else if let Some(path) = self.path {
            path[0]
        } else {
            [0., 0.]
        }
    }
}

impl ToSingleVec for Poracle {
    fn to_single_vec(self) -> single_vec::SingleVec {
        if let Some(multipath) = self.multipath {
            multipath.into_iter().flatten().collect()
        } else if let Some(path) = self.path {
            path
        } else {
            vec![]
        }
    }
}

impl ToMultiVec for Poracle {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        if let Some(multipath) = self.multipath.as_ref() {
            multipath.to_vec()
        } else if let Some(path) = self.path.as_ref() {
            vec![path.to_vec()]
        } else {
            vec![]
        }
    }
}

impl ToPointStruct for Poracle {
    fn to_struct(self) -> point_struct::PointStruct {
        if let Some(multipath) = self.multipath {
            point_struct::PointStruct {
                lat: multipath[0][0][0],
                lon: multipath[0][0][1],
            }
        } else if let Some(path) = self.path {
            point_struct::PointStruct {
                lat: path[0][0],
                lon: path[0][1],
            }
        } else {
            point_struct::PointStruct::default()
        }
    }
}

impl ToSingleStruct for Poracle {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        if let Some(multipath) = self.multipath {
            multipath
                .into_iter()
                .flat_map(|point| point.to_single_struct())
                .collect()
        } else if let Some(path) = self.path {
            path.into_iter().map(|point| point.to_struct()).collect()
        } else {
            vec![]
        }
    }
}

impl ToMultiStruct for Poracle {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        if let Some(multipath) = self.multipath {
            multipath
                .into_iter()
                .map(|point| point.to_single_struct())
                .collect()
        } else if let Some(path) = self.path {
            vec![path.into_iter().map(|point| point.to_struct()).collect()]
        } else {
            vec![]
        }
    }
}

impl ToFeature for Poracle {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        let mut feature = Feature {
            bbox: self.clone().to_single_vec().get_bbox(),
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    self.clone().to_multi_vec().get_geojson_value(enum_type)
                } else if self.multipath.is_some() {
                    self.clone().to_multi_vec().multi_polygon()
                } else if self.path.is_some() {
                    self.clone().to_multi_vec().polygon()
                } else {
                    Value::Point(vec![0., 0.])
                },
            }),
            ..Default::default()
        };
        if let Some(property) = self.name {
            feature.set_property("name", property);
        }
        if let Some(property) = self.id {
            feature.set_property("id", property);
        }
        if let Some(property) = self.color {
            feature.set_property("color", property);
        }
        if let Some(property) = self.group {
            feature.set_property("group", property);
        }
        if let Some(property) = self.description {
            feature.set_property("description", property);
        }
        if let Some(property) = self.user_selectable {
            feature.set_property("user_selectable", property);
        }
        if let Some(property) = self.display_in_matches {
            feature.set_property("display_in_matches", property);
        }
        feature
    }
}

impl ToCollection for Poracle {
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

impl ToCollection for Vec<Poracle> {
    fn to_collection(self, name: Option<String>, enum_type: Option<&Type>) -> FeatureCollection {
        let name = if let Some(name) = name {
            name
        } else {
            "".to_string()
        };
        let length = self.len();
        FeatureCollection {
            bbox: self
                .clone()
                .into_iter()
                .flat_map(|x| x.to_single_vec())
                .collect::<single_vec::SingleVec>()
                .get_bbox(),
            features: self
                .into_iter()
                .enumerate()
                .map(|(i, poracle_feat)| {
                    poracle_feat.to_feature(enum_type).ensure_properties(
                        Some(if length > 1 {
                            format!("{}_{}", name, i)
                        } else {
                            name.clone()
                        }),
                        enum_type,
                    )
                })
                .collect(),
            foreign_members: None,
        }
    }
}

impl ToText for Poracle {
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        if let Some(multipath) = self.multipath {
            multipath.to_text(sep_1, sep_2)
        } else if let Some(path) = self.path {
            path.to_text(sep_1, sep_2)
        } else {
            "".to_string()
        }
    }
}
