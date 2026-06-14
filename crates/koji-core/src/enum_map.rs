//! String → domain-enum mappers. Returns the koji-core domain `Category` enum;
//! the db layer converts to its sea-orm enum via `enum_bridge!` at the boundary.

use crate::Category;

pub fn get_category_enum(category: String) -> Category {
    match category.to_lowercase().as_str() {
        "database" => Category::Database,
        "boolean" => Category::Boolean,
        "number" => Category::Number,
        "object" => Category::Object,
        "array" => Category::Array,
        "color" => Category::Color,
        _ => Category::String,
    }
}
