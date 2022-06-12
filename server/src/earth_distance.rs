use ndarray::ArrayView1;
use num_traits::{Float, NumCast, Zero};
use petal_neighbors::distance::Metric;
use std::{fmt::Debug, ops::AddAssign};

#[derive(Default, Clone, Debug, PartialEq)]
pub struct EarthDistance {}

impl<A> Metric<A> for EarthDistance
where
    A: Float + Zero + AddAssign + Debug,
{
    fn distance(&self, start: &ArrayView1<A>, end: &ArrayView1<A>) -> A {
        // your definition of the distance here
        let d_lat: A = (end[0] - start[0]).to_radians();
        let d_lon: A = (end[1] - start[1]).to_radians();
        let lat1: A = (start[0]).to_radians();
        let lat2: A = (end[0]).to_radians();

        let multi: A = NumCast::from::<f64>(2.0).unwrap();
        let one: A = NumCast::from::<f64>(1.0).unwrap();
        let rad: A = NumCast::from::<f64>(6371.0).unwrap();

        let a: A = ((d_lat / multi).sin()) * ((d_lat / multi).sin())
            + ((d_lon / multi).sin()) * ((d_lon / multi).sin()) * (lat1.cos()) * (lat2.cos());
        let c: A = multi * ((a.sqrt()).atan2((one - a).sqrt()));
        // println!("{:?}", c * rad);
        rad * c
    }

    fn rdistance(&self, _: &ArrayView1<A>, _: &ArrayView1<A>) -> A {
        A::zero()
    }

    fn rdistance_to_distance(&self, _: A) -> A {
        A::zero()
    }

    fn distance_to_rdistance(&self, _: A) -> A {
        A::zero()
    }
}
