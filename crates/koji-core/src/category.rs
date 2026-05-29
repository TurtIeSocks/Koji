use macros::StrEnum;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StrEnum)]
pub enum Category {
    #[str("boolean")]
    Boolean,
    #[str("string")]
    String,
    #[str("number")]
    Number,
    #[str("object")]
    Object,
    #[str("array")]
    Array,
    #[str("database")]
    Database,
    #[str("color")]
    Color,
}
