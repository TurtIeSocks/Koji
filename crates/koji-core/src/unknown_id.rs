use serde::{Deserialize, Serialize};

/// An entity id that may arrive as either a string or a number on the wire.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnknownId {
    String(String),
    Number(u32),
}

impl ToString for UnknownId {
    fn to_string(&self) -> String {
        match self {
            UnknownId::Number(id) => id.to_string(),
            UnknownId::String(id) => id.to_string(),
        }
    }
}
