use macros::StrEnum;

/// Domain equivalent of the scanner `type` column. Mirrors the legacy sea-orm
/// `Type` variant-for-variant; `model::db` bridges the two via `enum_bridge!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StrEnum)]
pub enum FenceType {
    #[str("circle_pokemon")]
    CirclePokemon,
    #[str("circle_smart_pokemon")]
    CircleSmartPokemon,
    #[str("circle_raid")]
    CircleRaid,
    #[str("circle_smart_raid")]
    CircleSmartRaid,
    #[str("auto_quest")]
    AutoQuest,
    #[str("circle_quest")]
    CircleQuest,
    #[str("circle_station")]
    CircleStation,
    #[str("pokemon_iv")]
    PokemonIv,
    #[str("leveling")]
    Leveling,
    #[str("auto_pokemon")]
    AutoPokemon,
    #[str("auto_tth")]
    AutoTth,
    /// Only valid in the Kōji database.
    #[str("unset")]
    Unset,
}
