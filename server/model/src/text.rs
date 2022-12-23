use super::{
    scanner::{InstanceParsing, RdmInstanceArea},
    *,
};

pub trait TextHelpers {
    fn text_test(&self) -> bool;
    fn parse_scanner_instance(self, name: Option<String>, enum_type: Option<&Type>) -> Feature;
}

impl TextHelpers for String {
    fn text_test(&self) -> bool {
        let split: Vec<&str> = self.split_whitespace().collect();
        match split[0].parse::<f64>() {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    fn parse_scanner_instance(self, name: Option<String>, enum_type: Option<&Type>) -> Feature {
        let mut parsed = if self.starts_with("{") {
            match serde_json::from_str::<InstanceParsing>(&self) {
                Ok(result) => match result {
                    InstanceParsing::Feature(feat) => feat,
                    InstanceParsing::Rdm(json) => {
                        let mut feature = match json.area {
                            RdmInstanceArea::Leveling(point) => point.to_feature(enum_type),
                            RdmInstanceArea::Single(area) => area.to_feature(enum_type),
                            RdmInstanceArea::Multi(area) => area.to_feature(enum_type),
                        };
                        if let Some(radius) = json.radius {
                            feature.set_property("radius", radius);
                        }
                        feature
                    }
                },
                Err(err) => {
                    println!(
                        "Error Parsing Instance: {}\n{}",
                        name.clone().unwrap_or("".to_string()),
                        err
                    );
                    Feature::default()
                }
            }
        } else {
            self.to_feature(enum_type)
        };
        parsed.add_instance_properties(name, enum_type);
        parsed
    }
}

impl ToPointArray for String {
    fn to_point_array(self) -> point_array::PointArray {
        self.to_single_vec()[0]
    }
}

impl ToSingleVec for String {
    fn to_single_vec(self) -> single_vec::SingleVec {
        let mut points: single_vec::SingleVec = vec![];
        let test = self.text_test();
        let coords: Vec<&str> = self.split(if test { "," } else { "\n" }).collect();
        for coord in coords {
            let lat_lon: Vec<&str> = if test {
                coord.split_whitespace().collect()
            } else {
                coord.split(",").collect()
            };
            if lat_lon.is_empty() || lat_lon.concat().is_empty() {
                continue;
            }
            let lat = lat_lon[0].trim().parse::<f64>();
            let lat = match lat {
                Ok(lat) => lat,
                Err(_) => continue,
            };
            let lon = lat_lon[1].trim().parse::<f64>();
            let lon = match lon {
                Ok(lon) => lon,
                Err(_) => continue,
            };
            points.push([lat, lon]);
        }
        points.ensure_first_last()
    }
}

impl ToMultiVec for String {
    fn to_multi_vec(self) -> multi_vec::MultiVec {
        vec![self.to_single_vec()]
    }
}

impl ToPointStruct for String {
    fn to_struct(self) -> point_struct::PointStruct {
        self.to_single_vec().to_struct()
    }
}

impl ToSingleStruct for String {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        self.to_single_vec()
            .into_iter()
            .map(|each| each.to_struct())
            .collect()
    }
}

impl ToMultiStruct for String {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        vec![self.to_single_struct()]
    }
}

impl ToFeature for String {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        let multi_vec = self.to_multi_vec();
        Feature {
            bbox: multi_vec.clone().to_single_vec().get_bbox(),
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    multi_vec.get_geojson_value(enum_type)
                } else {
                    multi_vec.polygon()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToCollection for String {
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

impl ToPoracle for String {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            path: Some(self.to_single_vec()),
            ..Default::default()
        }
    }
}
