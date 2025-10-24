use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use geo::{Coord, Destination, Distance, Haversine};
use geohash::encode;
use model::api::Precision;
use rayon::slice::ParallelSliceMut;
use rstar::{AABB, PointDistance, RTreeObject};
use s2::{cellid::CellID, latlng::LatLng};

use super::{SortDedupe, cluster::Cluster};

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub radius: Precision,
    pub center: [Precision; 2],
    pub cell_id: CellID,
    cached_envelope: AABB<[Precision; 2]>,
}

impl Point {
    pub fn new(radius: Precision, cell_level: u64, center: [Precision; 2]) -> Self {
        let gp = geo::Point::new(center[1], center[0]);
        let corner_1 = Haversine.destination(gp, 225., radius * 2.);
        let corner_2 = Haversine.destination(gp, 45., radius * 2.);

        Self {
            radius,
            center,
            cell_id: CellID::from(LatLng::from_degrees(center[0], center[1])).parent(cell_level),
            cached_envelope: AABB::from_corners(
                [corner_1.y(), corner_1.x()],
                [corner_2.y(), corner_2.x()],
            ),
        }
    }

    fn gp(&self) -> geo::Point {
        geo::Point::new(self.center[1], self.center[0])
    }

    pub fn _get_geohash(&self) -> String {
        encode(
            Coord {
                x: self.center[1],
                y: self.center[0],
            },
            12,
        )
        .unwrap()
    }
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
        self.cached_envelope
    }
}

impl PointDistance for Point {
    fn distance_2(&self, other: &[Precision; 2]) -> Precision {
        Haversine.distance(self.gp(), geo::Point::new(other[1], other[0]))
    }

    fn contains_point(&self, point: &<Self::Envelope as rstar::Envelope>::Point) -> bool {
        self.distance_2(point) <= self.radius
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:.6}, {:.6} | {} | {}",
            self.center[0],
            self.center[1],
            self.cell_id.0,
            self._get_geohash(),
        )
    }
}

impl<'a> From<Cluster<'a>> for Point {
    fn from(cluster: Cluster) -> Self {
        cluster.point
    }
}

impl SortDedupe for Vec<&Point> {
    fn sort_dedupe(&mut self) {
        self.par_sort_by(|a, b| a.cell_id.cmp(&b.cell_id));
        self.dedup_by(|a, b| a.cell_id == b.cell_id);
    }
}
