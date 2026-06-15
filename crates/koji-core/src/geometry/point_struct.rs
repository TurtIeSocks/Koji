use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PointStruct {
    pub lat: Precision,
    pub lon: Precision,
}
impl Default for PointStruct {
    fn default() -> PointStruct {
        PointStruct { lat: 0., lon: 0. }
    }
}

impl From<PointArray> for PointStruct {
    fn from(p: PointArray) -> Self {
        PointStruct {
            lat: p[0],
            lon: p[1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_zero_zero() {
        let p = PointStruct::default();
        assert_eq!(p.lat, 0.0);
        assert_eq!(p.lon, 0.0);
    }

    #[test]
    fn from_point_array_maps_lat_lon_correctly() {
        // PointArray is [lat, lon].
        let arr: PointArray = [51.5, -0.12];
        let ps = PointStruct::from(arr);
        assert_eq!(ps.lat, 51.5);
        assert_eq!(ps.lon, -0.12);
    }

    #[test]
    fn from_point_array_negative_lat() {
        let arr: PointArray = [-33.87, 151.21];
        let ps = PointStruct::from(arr);
        assert_eq!(ps.lat, -33.87);
        assert_eq!(ps.lon, 151.21);
    }

    #[test]
    fn serde_roundtrip() {
        let ps = PointStruct { lat: 10.5, lon: 20.75 };
        let v = serde_json::to_value(&ps).unwrap();
        assert_eq!(v, serde_json::json!({ "lat": 10.5, "lon": 20.75 }));
        let back: PointStruct = serde_json::from_value(v).unwrap();
        assert_eq!(back.lat, 10.5);
        assert_eq!(back.lon, 20.75);
    }

    #[test]
    fn clone_is_independent() {
        let p = PointStruct { lat: 1.0, lon: 2.0 };
        let mut q = p.clone();
        q.lat = 99.0;
        assert_eq!(p.lat, 1.0, "clone should not alias original");
    }
}
