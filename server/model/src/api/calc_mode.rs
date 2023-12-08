use super::*;

#[derive(Debug, Clone)]
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
