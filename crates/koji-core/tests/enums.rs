use koji_core::{Category, FenceType};

#[test]
fn fence_type_as_str_and_roundtrip() {
    assert_eq!(FenceType::CirclePokemon.as_str(), "circle_pokemon");
    assert_eq!(FenceType::Unset.as_str(), "unset");
    assert_eq!(
        FenceType::from_str_opt("auto_quest"),
        Some(FenceType::AutoQuest)
    );
    assert_eq!(FenceType::from_str_opt("nope"), None);
}

#[test]
fn fence_type_serde() {
    assert_eq!(
        serde_json::to_string(&FenceType::CircleRaid).unwrap(),
        "\"circle_raid\""
    );
    let v: FenceType = serde_json::from_str("\"leveling\"").unwrap();
    assert_eq!(v, FenceType::Leveling);
    assert!(serde_json::from_str::<FenceType>("\"bogus\"").is_err());
}

#[test]
fn category_serde() {
    assert_eq!(
        serde_json::to_string(&Category::Color).unwrap(),
        "\"color\""
    );
    assert_eq!(Category::from_str_opt("array"), Some(Category::Array));
}
