use macros::StrEnum;

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
pub enum CalculationMode {
    #[str("radius")]
    Radius,
    #[str("s2")]
    S2,
    #[str(default)]
    Custom(String),
}
