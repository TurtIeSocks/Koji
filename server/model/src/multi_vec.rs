use super::*;

pub type MultiVec<T = Precision> = Vec<Vec<point_array::PointArray<T>>>;

impl ValueHelpers for MultiVec {
    fn get_geojson_value(self, enum_type: &Type) -> Value {
        match enum_type {
            Type::AutoQuest | Type::PokemonIv => self.multi_polygon(),
            Type::CirclePokemon
            | Type::CircleSmartPokemon
            | Type::CircleRaid
            | Type::CircleSmartRaid
            | Type::ManualQuest => self.multi_point(),
            Type::Leveling => self.point(),
        }
    }
    fn point(self) -> Value {
        Value::Point(vec![self[0][0][1], self[0][0][0]])
    }
    fn multi_point(self) -> Value {
        Value::MultiPoint(
            self.into_iter()
                .flat_map(|poly| {
                    poly.into_iter()
                        .map(|[lat, lon]| vec![lon, lat])
                        .collect::<Vec<Vec<f64>>>()
                })
                .collect(),
        )
    }
    fn polygon(self) -> Value {
        Value::Polygon(
            self.into_iter()
                .map(|lines| lines.into_iter().map(|[lat, lon]| vec![lon, lat]).collect())
                .collect(),
        )
    }
    fn multi_polygon(self) -> Value {
        Value::MultiPolygon(
            self.into_iter()
                .map(|poly| vec![poly.into_iter().map(|[lat, lon]| vec![lon, lat]).collect()])
                .collect(),
        )
    }
}

impl ToPointArray for MultiVec {
    fn to_point_array(self) -> point_array::PointArray {
        self[0][0]
    }
}

impl ToSingleVec for MultiVec {
    fn to_single_vec(self) -> single_vec::SingleVec {
        self.into_iter()
            .flat_map(|polygon| polygon.ensure_first_last())
            .collect()
    }
}

impl ToMultiVec for MultiVec {
    fn to_multi_vec(self) -> MultiVec {
        self.into_iter()
            .map(|polygon| polygon.ensure_first_last())
            .collect()
    }
}

impl ToPointStruct for MultiVec {
    fn to_struct(self) -> point_struct::PointStruct {
        println!("`to_struct()` was called on a MultiVec and this was likely unintentional, did you mean to map over the values first?");
        point_struct::PointStruct {
            lat: self[0][0][0],
            lon: self[1][0][1],
        }
    }
}

impl ToSingleStruct for MultiVec {
    fn to_single_struct(self) -> single_struct::SingleStruct {
        self.into_iter().map(|each| each.to_struct()).collect()
    }
}

impl ToMultiStruct for MultiVec {
    fn to_multi_struct(self) -> multi_struct::MultiStruct {
        self.into_iter()
            .map(|each| each.to_single_struct())
            .collect()
    }
}

impl ToFeature for MultiVec {
    fn to_feature(self, enum_type: Option<&Type>) -> Feature {
        Feature {
            geometry: Some(Geometry {
                bbox: None,
                foreign_members: None,
                value: if let Some(enum_type) = enum_type {
                    self.get_geojson_value(enum_type)
                } else {
                    self.multi_polygon()
                },
            }),
            ..Default::default()
        }
    }
}

impl ToCollection for MultiVec {
    fn to_collection(self, enum_type: Option<&Type>) -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: vec![self.to_feature(enum_type)],
            foreign_members: None,
        }
    }
}

impl ToText for MultiVec {
    fn to_text(self, sep_1: &str, sep_2: &str) -> String {
        self.into_iter()
            .map(|each| each.to_text(sep_1, sep_2))
            .collect()
    }
}

impl ToPoracle for MultiVec {
    fn to_poracle(self) -> poracle::Poracle {
        poracle::Poracle {
            multipath: Some(self.to_multi_vec()),
            ..Default::default()
        }
    }
}
