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
    #[str(default)]
    Custom(String),
}
