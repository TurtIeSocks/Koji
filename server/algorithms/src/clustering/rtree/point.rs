use std::hash::{Hash, Hasher};

use model::api::{single_vec::SingleVec, Precision};
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use s2::{cellid::CellID, latlng::LatLng};

const R: Precision = 6378137.0;
const X: Precision = std::f64::consts::PI / 180.0;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub radius: Precision,
    pub center: [Precision; 2],
    pub cell_id: CellID,
}

impl Point {
    pub fn new(radius: Precision, center: [Precision; 2]) -> Self {
        Self {
            radius,
            center,
            cell_id: CellID::from(LatLng::from_degrees(center[0], center[1])).parent(20),
        }
    }

    pub fn interpolate(&self, next: &Point, ratio: f64) -> Self {
        let lat = self.center[0] * (1. - ratio) + next.center[0] * ratio;
        let lon = self.center[1] * (1. - ratio) + next.center[1] * ratio;
        let new_point = Self::new(self.radius, [lat, lon]);
        new_point
    }

    // pub fn midpoint(&self, other: &Point) -> Self {
    //     let lat = (self.center[0] + other.center[0]) / 2.0;
    //     let lon = (self.center[1] + other.center[1]) / 2.0;
    //     Self::new(self.radius, [lat, lon])
    // }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.cell_id == other.cell_id
    }
}

impl Eq for Point {}

impl Hash for Point {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cell_id.hash(state);
    }
}

impl RTreeObject for Point {
    type Envelope = AABB<[Precision; 2]>;

    fn envelope(&self) -> Self::Envelope {
        let corner_1 = [self.center[0] - self.radius, self.center[1] - self.radius];
        let corner_2 = [self.center[0] + self.radius, self.center[1] + self.radius];
        AABB::from_corners(corner_1, corner_2)
    }
}

impl PointDistance for Point {
    fn distance_2(&self, other: &[Precision; 2]) -> Precision {
        let lat1 = self.center[0] * X;
        let lon1 = self.center[1] * X;
        let lat2 = other[0] * X;
        let lon2 = other[1] * X;
        let a = lat1.sin() * lat2.sin() + lat1.cos() * lat2.cos() * (lon2 - lon1).cos();
        a.acos() * R
    }

    fn contains_point(&self, point: &<Self::Envelope as rstar::Envelope>::Point) -> bool {
        self.distance_2(point) <= self.radius
    }
}

pub fn main(radius: f64, points: SingleVec) -> RTree<Point> {
    let spawnpoints = points
        .into_iter()
        .map(|p| Point::new(radius, p))
        .collect::<Vec<_>>();
    RTree::bulk_load(spawnpoints)
}
