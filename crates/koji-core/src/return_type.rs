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
    Feature,
    FeatureCollection,
    PoracleSingle,
    Poracle,
    Sql,
}
