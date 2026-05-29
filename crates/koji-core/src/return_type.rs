use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ReturnTypeArg {
    AltText,
    Text,
    SingleArray,
    MultiArray,
    SingleStruct,
    MultiStruct,
    Geometry,
    GeometryVec,
    Feature,
    FeatureVec,
    FeatureCollection,
    PoracleSingle,
    Poracle,
    Sql,
}
