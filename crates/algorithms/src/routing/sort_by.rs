use macros::StrEnum;

#[derive(Debug, Clone, PartialEq, Eq, StrEnum)]
pub enum SortBy {
    #[str("unset", alias("", "none"))]
    Unset,
    #[str("geohash")]
    GeoHash,
    #[str("pointcount", alias("cluster_count", "point_count", "clustercount"))]
    PointCount,
    #[str("random")]
    Random,
    #[str("s2cell", alias("s2"))]
    S2Cell,
    #[str("latlon")]
    LatLon,
    #[str("tsp", alias("optimized", "2opt"))]
    Tsp,
    #[str("tsphybrid", alias("hybrid"))]
    TspHybrid,
    #[str(default)]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── StrEnum-derived from_str_opt / Display ────────────────────────────────

    #[test]
    fn canonical_names_round_trip() {
        for (s, expected) in [
            ("unset", SortBy::Unset),
            ("geohash", SortBy::GeoHash),
            ("pointcount", SortBy::PointCount),
            ("random", SortBy::Random),
            ("s2cell", SortBy::S2Cell),
            ("latlon", SortBy::LatLon),
        ] {
            let parsed = SortBy::from_str_opt(s);
            assert_eq!(parsed, Some(expected), "failed for '{s}'");
        }
    }

    #[test]
    fn aliases_parse_correctly() {
        // "" and "none" are aliases for Unset.
        assert_eq!(SortBy::from_str_opt(""), Some(SortBy::Unset));
        assert_eq!(SortBy::from_str_opt("none"), Some(SortBy::Unset));
        // s2 is an alias for S2Cell.
        assert_eq!(SortBy::from_str_opt("s2"), Some(SortBy::S2Cell));
        // point_count and cluster_count are aliases for PointCount.
        assert_eq!(
            SortBy::from_str_opt("point_count"),
            Some(SortBy::PointCount)
        );
        assert_eq!(
            SortBy::from_str_opt("cluster_count"),
            Some(SortBy::PointCount)
        );
        assert_eq!(
            SortBy::from_str_opt("clustercount"),
            Some(SortBy::PointCount)
        );
    }

    #[test]
    fn unknown_string_becomes_custom() {
        let s = SortBy::from_str_opt("my_plugin");
        assert!(matches!(s, Some(SortBy::Custom(ref v)) if v == "my_plugin"));
    }

    #[test]
    fn equality_of_same_custom() {
        let a = SortBy::Custom("foo".into());
        let b = SortBy::Custom("foo".into());
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_of_different_custom() {
        let a = SortBy::Custom("foo".into());
        let b = SortBy::Custom("bar".into());
        assert_ne!(a, b);
    }

    #[test]
    fn display_canonical() {
        assert_eq!(format!("{}", SortBy::Unset), "unset");
        assert_eq!(format!("{}", SortBy::GeoHash), "geohash");
        assert_eq!(format!("{}", SortBy::S2Cell), "s2cell");
        assert_eq!(format!("{}", SortBy::LatLon), "latlon");
    }

    #[test]
    fn tsp_hybrid_parses_and_displays() {
        assert_eq!(SortBy::from_str_opt("tsphybrid"), Some(SortBy::TspHybrid));
        assert_eq!(SortBy::from_str_opt("hybrid"), Some(SortBy::TspHybrid));
        assert_eq!(format!("{}", SortBy::TspHybrid), "tsphybrid");
    }
}
