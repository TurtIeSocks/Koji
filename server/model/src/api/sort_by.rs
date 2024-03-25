use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum SortBy {
    Unset,
    GeoHash,
    PointCount,
    Random,
    S2Cell,
    LatLon,
    Hilbert,
    Custom(String),
}

impl PartialEq for SortBy {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SortBy::Unset, SortBy::Unset) => true,
            (SortBy::GeoHash, SortBy::GeoHash) => true,
            (SortBy::PointCount, SortBy::PointCount) => true,
            (SortBy::Random, SortBy::Random) => true,
            (SortBy::S2Cell, SortBy::S2Cell) => true,
            _ => false,
        }
    }
}

impl Eq for SortBy {}

impl<'de> Deserialize<'de> for SortBy {
    fn deserialize<D>(deserializer: D) -> Result<SortBy, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;

        match s.to_lowercase().as_str() {
            "geohash" => Ok(SortBy::GeoHash),
            "cluster_count" | "point_count" | "clustercount" | "pointcount" => {
                Ok(SortBy::PointCount)
            }
            "random" => Ok(SortBy::Random),
            "s2" | "s2cell" => Ok(SortBy::S2Cell),
            "latlon" => Ok(SortBy::LatLon),
            "" | "none" | "unset" => Ok(SortBy::Unset),
            "hilbert" => Ok(SortBy::Hilbert),
            // This is for backwards compatibility since the custom below would end up with a value of "TSP"
            "tsp" => Ok(SortBy::Custom("tsp".to_string())),
            _ => Ok(SortBy::Custom(s)),
        }
    }
}
