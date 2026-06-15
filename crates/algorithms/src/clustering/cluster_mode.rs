use macros::StrEnum;

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
pub enum ClusterMode {
    #[str("honeycomb")]
    Honeycomb,
    #[str("fastest")]
    Fastest,
    #[str("fast")]
    Fast,
    #[str("balanced")]
    Balanced,
    #[str("better")]
    Better,
    #[str("best")]
    Best,
    #[str(default)]
    Custom(String),
}
