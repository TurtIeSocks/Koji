use serde::{Deserialize, Serialize};

use crate::geometry::KojiBbox;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsArg {
    #[serde(flatten)]
    pub bbox: KojiBbox,
    pub last_seen: Option<u32>,
    pub ids: Option<Vec<String>>,
    pub tth: Option<SpawnpointTth>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SpawnpointTth {
    All,
    Known,
    Unknown,
}

#[cfg(test)]
mod bounds_arg_tests {
    use super::*;

    #[test]
    fn bounds_arg_wire_is_flat_and_identical() {
        let json = r#"{"min_lat":1.0,"min_lon":2.0,"max_lat":3.0,"max_lon":4.0,"last_seen":5}"#;
        let parsed: BoundsArg = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bbox.min_lat, 1.0);
        assert_eq!(parsed.bbox.min_lon, 2.0);
        assert_eq!(parsed.bbox.max_lat, 3.0);
        assert_eq!(parsed.bbox.max_lon, 4.0);
        assert_eq!(parsed.last_seen, Some(5));
        let back = serde_json::to_value(&parsed).unwrap();
        assert_eq!(back["min_lat"], 1.0);
        assert_eq!(back["max_lon"], 4.0);
        assert_eq!(back["last_seen"], 5);
        assert!(
            back.get("bbox").is_none(),
            "bbox must be flattened, not nested"
        );
    }
}
