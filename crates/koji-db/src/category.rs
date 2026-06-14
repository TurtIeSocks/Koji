//! Domain `Category` — the pure value-type for a property's data category,
//! relocated here from koji-core (koji-db is its sole consumer). It bridges to
//! its sea-orm storage counterpart `crate::db::sea_orm_active_enums::Category`
//! via the crate-local `enum_bridge!` macro (see `db/enum_bridge.rs`).

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
