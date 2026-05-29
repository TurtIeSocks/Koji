use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum ClusterMode {
    Honeycomb,
    Fastest,
    Fast,
    Balanced,
    Better,
    Best,
    Custom(String),
}

impl<'de> Deserialize<'de> for ClusterMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: String = serde::Deserialize::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "honeycomb" => Ok(ClusterMode::Honeycomb),
            "fastest" => Ok(ClusterMode::Fastest),
            "fast" => Ok(ClusterMode::Fast),
            "balanced" => Ok(ClusterMode::Balanced),
            "better" => Ok(ClusterMode::Better),
            "best" => Ok(ClusterMode::Best),
            "bruteforce" => {
                log::warn!("bruteforce is now deprecated, using `better` strategy instead");
                Ok(ClusterMode::Better)
            }
            "rtree" => {
                log::warn!("rtree is now deprecated, using `balanced` strategy instead");
                Ok(ClusterMode::Balanced)
            }
            _ => Ok(ClusterMode::Custom(s)),
        }
    }
}

impl PartialEq for ClusterMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ClusterMode::Honeycomb, ClusterMode::Honeycomb)
            | (ClusterMode::Fastest, ClusterMode::Fastest)
            | (ClusterMode::Fast, ClusterMode::Fast)
            | (ClusterMode::Balanced, ClusterMode::Balanced)
            | (ClusterMode::Better, ClusterMode::Better)
            | (ClusterMode::Best, ClusterMode::Best) => true,
            _ => false,
        }
    }
}

impl Eq for ClusterMode {}

impl ToString for ClusterMode {
    fn to_string(&self) -> String {
        match self {
            ClusterMode::Honeycomb => "Honeycomb",
            ClusterMode::Fastest => "Fastest",
            ClusterMode::Fast => "Fast",
            ClusterMode::Balanced => "Balanced",
            ClusterMode::Better => "Better",
            ClusterMode::Best => "Best",
            ClusterMode::Custom(s) => s,
        }
        .to_string()
    }
}
