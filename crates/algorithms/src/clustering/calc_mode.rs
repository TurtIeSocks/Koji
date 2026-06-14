use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalculationMode {
    Radius,
    S2,
    Custom(String),
}

impl<'de> Deserialize<'de> for CalculationMode {
    fn deserialize<D>(deserializer: D) -> Result<CalculationMode, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        match s.to_lowercase().as_str() {
            "radius" => Ok(CalculationMode::Radius),
            "s2" => Ok(CalculationMode::S2),
            _ => Ok(CalculationMode::Custom(s)),
        }
    }
}

impl Serialize for CalculationMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            CalculationMode::Radius => "radius",
            CalculationMode::S2 => "s2",
            CalculationMode::Custom(s) => s,
        })
    }
}
