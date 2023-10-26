use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use geo::Coord;
use geohash::encode;
use map_3d::EARTH_RADIUS;
use model::api::Precision;
use rstar::{PointDistance, RTreeObject, AABB};
use s2::{cell::Cell, cellid::CellID, latlng::LatLng};

use super::cluster::Cluster;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub radius: Precision,
    pub center: [Precision; 2],
    pub cell_id: CellID,
}

impl Point {
    pub fn new(radius: Precision, cell_level: u64, center: [Precision; 2]) -> Self {
        Self {
            radius,
            center,
            cell_id: CellID::from(LatLng::from_degrees(center[0], center[1])).parent(cell_level),
        }
    }

    pub fn interpolate(
        &self,
        next: &Self,
        ratio: Precision,
        wiggle_lat: Precision,
        wiggle_lon: Precision,
    ) -> Self {
        let lat = self.center[0] * (1. - ratio) + (next.center[0] + wiggle_lat) * ratio;
        let lon = self.center[1] * (1. - ratio) + (next.center[1] + wiggle_lon) * ratio;
        let new_point = Self::new(self.radius, self.cell_id.level(), [lat, lon]);
        new_point
    }

    fn haversine_destination(&self, bearing: Precision) -> [Precision; 2] {
        let center_lat = self.center[0].to_radians();
        let center_lng = self.center[1].to_radians();
        let bearing_rad = bearing.to_radians();

        let rad = self.radius * 2. / EARTH_RADIUS;

        let lat =
            { center_lat.sin() * rad.cos() + center_lat.cos() * rad.sin() * bearing_rad.cos() }
                .asin();
        let lng = { bearing_rad.sin() * rad.sin() * center_lat.cos() }
            .atan2(rad.cos() - center_lat.sin() * lat.sin())
            + center_lng;

        [lat.to_degrees(), lng.to_degrees()]
    }

    fn haversine_distance(&self, other: &[Precision; 2]) -> Precision {
        let theta1 = self.center[0].to_radians();
        let theta2 = other[0].to_radians();
        let delta_theta = (other[0] - self.center[0]).to_radians();
        let delta_lambda = (other[1] - self.center[1]).to_radians();
        let a = (delta_theta / 2.).sin().powi(2)
            + theta1.cos() * theta2.cos() * (delta_lambda / 2.).sin().powi(2);
        let c = 2. * a.sqrt().asin();
        EARTH_RADIUS * c
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
        let corner_1 = self.haversine_destination(225.);
        let corner_2 = self.haversine_destination(45.);
        AABB::from_corners(corner_1, corner_2)
    }
}

impl PointDistance for Point {
    fn distance_2(&self, other: &[Precision; 2]) -> Precision {
        self.haversine_distance(other)
    }

    fn contains_point(&self, point: &<Self::Envelope as rstar::Envelope>::Point) -> bool {
        self.distance_2(point) <= self.radius
    }
}

pub trait ToPoint {
    fn to_point(&self, radius: Precision) -> Point;
}

impl ToPoint for CellID {
    fn to_point(&self, radius: Precision) -> Point {
        let cell: Cell = self.into();
        let center = cell.center();
        Point::new(
            radius,
            self.level(),
            [center.latitude().deg(), center.longitude().deg()],
        )
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
