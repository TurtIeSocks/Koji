use koji_core::Category;

#[test]
fn category_serde() {
    assert_eq!(
        serde_json::to_string(&Category::Color).unwrap(),
        "\"color\""
    );
    assert_eq!(Category::from_str_opt("array"), Some(Category::Array));
}
