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
