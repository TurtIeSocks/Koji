use sea_orm::FromQueryResult;
use koji_core::Precision;
use serde::{Deserialize, Serialize};

/// Query-row for `SELECT lat, lon` from golbat tables. Converts into the pure
/// `koji_core::PointStruct`.
#[derive(Debug, FromQueryResult)]
pub struct LatLonRow {
    pub lat: Precision,
    pub lon: Precision,
}

impl From<LatLonRow> for koji_core::PointStruct {
    fn from(r: LatLonRow) -> Self {
        koji_core::PointStruct {
            lat: r.lat,
            lon: r.lon,
        }
    }
}

#[derive(Debug, Clone, FromQueryResult)]
pub struct Spawnpoint {
    pub lat: Precision,
    pub lon: Precision,
    pub despawn_sec: Option<u16>,
}

#[derive(Debug, FromQueryResult, Serialize, Deserialize, Clone)]
pub struct Total {
    pub total: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GenericData {
    pub i: String,
    pub p: [Precision; 2],
}

impl GenericData {
    pub fn new(i: String, lat: Precision, lon: Precision) -> Self {
        GenericData { i, p: [lat, lon] }
    }
}

pub trait GenericDataToVec {
    fn to_single_vec(self) -> koji_core::SingleVec;
}

impl GenericDataToVec for Vec<GenericData> {
    fn to_single_vec(self) -> koji_core::SingleVec {
        // `GenericData.p` is already `[lat, lon]` — the exact `PointArray` shape,
        // so map it through directly (no `To*` matrix trait).
        self.into_iter().map(|g| g.p).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::SingleVec;

    // ── LatLonRow → PointStruct ───────────────────────────────────────────────

    #[test]
    fn latlon_row_converts_to_point_struct() {
        let row = LatLonRow {
            lat: 12.345,
            lon: 67.890,
        };
        let ps: koji_core::PointStruct = row.into();
        assert_eq!(ps.lat, 12.345);
        assert_eq!(ps.lon, 67.890);
    }

    #[test]
    fn latlon_row_negative_coords_preserved() {
        let row = LatLonRow {
            lat: -33.87,
            lon: 151.21,
        };
        let ps: koji_core::PointStruct = row.into();
        assert_eq!(ps.lat, -33.87);
        assert_eq!(ps.lon, 151.21);
    }

    // ── GenericData::new ──────────────────────────────────────────────────────

    #[test]
    fn generic_data_new_stores_id_and_coords() {
        let g = GenericData::new("v42".to_string(), 10.0, 20.0);
        assert_eq!(g.i, "v42");
        assert_eq!(g.p, [10.0, 20.0]);
    }

    #[test]
    fn generic_data_p_order_is_lat_lon() {
        // p[0] must be lat, p[1] must be lon — critical for downstream consumers.
        let lat = 48.858;
        let lon = 2.295;
        let g = GenericData::new("eiffel".to_string(), lat, lon);
        assert_eq!(g.p[0], lat);
        assert_eq!(g.p[1], lon);
    }

    // ── GenericDataToVec::to_single_vec ───────────────────────────────────────

    #[test]
    fn to_single_vec_empty_returns_empty() {
        let v: Vec<GenericData> = vec![];
        let sv: SingleVec = v.to_single_vec();
        assert!(sv.is_empty());
    }

    #[test]
    fn to_single_vec_preserves_all_points_in_order() {
        let data = vec![
            GenericData::new("g0".to_string(), 1.0, 2.0),
            GenericData::new("g1".to_string(), 3.0, 4.0),
            GenericData::new("g2".to_string(), 5.0, 6.0),
        ];
        let sv: SingleVec = data.to_single_vec();
        assert_eq!(sv.len(), 3);
        assert_eq!(sv[0], [1.0, 2.0]);
        assert_eq!(sv[1], [3.0, 4.0]);
        assert_eq!(sv[2], [5.0, 6.0]);
    }

    #[test]
    fn to_single_vec_is_lat_lon_order() {
        // Ensures the coord order the rest of the system expects: [lat, lon].
        let g = GenericData::new("x".to_string(), 51.5, -0.1);
        let sv: SingleVec = vec![g].to_single_vec();
        assert_eq!(sv[0][0], 51.5); // lat
        assert_eq!(sv[0][1], -0.1); // lon
    }

    // ── Total ─────────────────────────────────────────────────────────────────

    #[test]
    fn total_serializes_and_deserializes() {
        let t = Total { total: 42 };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("42"));
        let t2: Total = serde_json::from_str(&json).unwrap();
        assert_eq!(t2.total, 42);
    }

    #[test]
    fn total_zero_roundtrips() {
        let t = Total { total: 0 };
        let t2: Total = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(t2.total, 0);
    }

    #[test]
    fn generic_data_clone_is_independent() {
        let g = GenericData::new("original".to_string(), 1.0, 2.0);
        let mut g2 = g.clone();
        g2.i = "clone".to_string();
        assert_eq!(g.i, "original");
        assert_eq!(g2.i, "clone");
    }
}
