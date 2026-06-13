use serde::{Deserialize, Serialize};

/// An entity id that may arrive as either a string or a number on the wire.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum UnknownId {
    String(String),
    Number(u32),
}

impl std::fmt::Display for UnknownId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnknownId::Number(id) => write!(f, "{}", id),
            UnknownId::String(id) => f.write_str(id),
        }
    }
}
