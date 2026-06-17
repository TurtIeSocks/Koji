use crate::Precision;

pub trait HasLatLon {
    fn lat(&self) -> Precision;
    fn lon(&self) -> Precision;
}
