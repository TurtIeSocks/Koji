use crate::PointStruct;

pub trait HasLatLon {
    fn lat(&self) -> f64;
    fn lon(&self) -> f64;
}

impl HasLatLon for PointStruct {
    fn lat(&self) -> f64 {
        self.lat
    }
    fn lon(&self) -> f64 {
        self.lon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_struct_lat_lon_trait_returns_fields() {
        let p = PointStruct {
            lat: 48.85,
            lon: 2.35,
        };
        assert_eq!(p.lat(), 48.85);
        assert_eq!(p.lon(), 2.35);
    }

    #[test]
    fn point_struct_default_is_zero() {
        let p = PointStruct::default();
        assert_eq!(p.lat(), 0.0);
        assert_eq!(p.lon(), 0.0);
    }

    #[test]
    fn point_struct_negative_coords() {
        let p = PointStruct {
            lat: -33.87,
            lon: 151.21,
        };
        assert_eq!(p.lat(), -33.87);
        assert_eq!(p.lon(), 151.21);
    }
}
