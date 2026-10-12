use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use geo::Coord;
use geohash::encode;
use model::api::Precision;
use rayon::slice::ParallelSliceMut;
use rstar::{AABB, PointDistance, RTreeObject};
use s2::{cellid::CellID, latlng::LatLng};

use super::{SortDedupe, cluster::Cluster};

// Spawn visibility uses this sphere, rather than geo's mean Earth radius.
const EARTH_RADIUS_M: Precision = 6_367_000.;

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub radius: Precision,
    pub center: [Precision; 2],
    pub cell_id: CellID,
    cached_envelope: AABB<[Precision; 2]>,
}

impl Point {
    pub fn new(radius: Precision, cell_level: u64, center: [Precision; 2]) -> Self {
        // Keep a conservative envelope: the exact range check happens in
        // contains_point. Coordinates at the R-tree boundary are [lat, lon]
        // in degrees; spherical calculations use radians.
        let angular_radius = (radius * 2. / EARTH_RADIUS_M).min(std::f64::consts::PI);
        let lat = center[0].to_radians();
        let lat_min = (center[0] - angular_radius.to_degrees()).max(-90.);
        let lat_max = (center[0] + angular_radius.to_degrees()).min(90.);
        let (lon_min, lon_max) = if lat_min <= -90. || lat_max >= 90. {
            (-180., 180.)
        } else {
            let lon_delta = (angular_radius.sin() / lat.cos())
                .clamp(-1., 1.)
                .asin()
                .to_degrees();
            let min = center[1] - lon_delta;
            let max = center[1] + lon_delta;
            // A single AABB cannot wrap across the antimeridian. Include all
            // longitudes so it cannot prune visible points on the other side.
            if min <= -180. || max >= 180. {
                (-180., 180.)
            } else {
                (min, max)
            }
        };

        Self {
            radius,
            center,
            cell_id: CellID::from(LatLng::from_degrees(center[0], center[1])).parent(cell_level),
            cached_envelope: AABB::from_corners([lat_min, lon_min], [lat_max, lon_max]),
        }
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
    // Despite the trait name, existing range callers expect meters here.
    fn distance_2(&self, other: &[Precision; 2]) -> Precision {
        let [spawn_lat, spawn_lon] = self.center.map(Precision::to_radians);
        let [player_lat, player_lon] = other.map(Precision::to_radians);
        let a = (((spawn_lat - player_lat) / 2.).sin().powi(2)
            + player_lat.cos() * spawn_lat.cos() * ((spawn_lon - player_lon) / 2.).sin().powi(2))
        .clamp(0., 1.);
        2. * EARTH_RADIUS_M * a.sqrt().atan2((1. - a).sqrt())
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstar::RTree;

    fn point(center: [Precision; 2]) -> Point {
        Point::new(70., 20, center)
    }

    #[test]
    fn visibility_at_seventy_meter_boundary() {
        let spawn = point([0., 0.]);
        for (meters, visible) in [(69.999, true), (70., true), (70.001, false)] {
            let player = [(meters / EARTH_RADIUS_M).to_degrees(), 0.];
            assert!((spawn.distance_2(&player) - meters).abs() < 1e-10);
            assert_eq!(spawn.contains_point(&player), visible, "{meters}m");
            let tree = RTree::bulk_load(vec![spawn]);
            assert_eq!(
                tree.locate_all_at_point(&player).count(),
                usize::from(visible)
            );
        }
    }

    #[test]
    fn coordinates_are_latitude_longitude_in_degrees() {
        let spawn = point([60., 10.]);
        // Along a meridian, one degree is R * pi / 180 meters.
        let meridian = spawn.distance_2(&[61., 10.]);
        assert!((meridian - EARTH_RADIUS_M * std::f64::consts::PI / 180.).abs() < 1e-8);
        // One degree of longitude at 60 N is about half as far.
        let parallel = spawn.distance_2(&[60., 11.]);
        assert!((parallel - 55_562.027_818_388_62).abs() < 1e-7);
    }

    #[test]
    fn distance_is_symmetric_at_high_latitudes() {
        for (a, b) in [
            ([80., 25.], [80.0001, 25.001]),
            ([-85., -40.], [-85.0001, -40.002]),
            ([45., 179.9998], [45., -179.9998]),
        ] {
            let forward = point(a).distance_2(&b);
            let reverse = point(b).distance_2(&a);
            assert!((forward - reverse).abs() < 1e-10);
            assert!(forward < 70.);
            assert!(point(a).contains_point(&b));
            assert!(point(b).contains_point(&a));
        }
    }

    #[test]
    fn rtree_visibility_across_antimeridian_and_poles() {
        for (spawn, player) in [
            ([0., 179.9998], [0., -179.9998]),
            ([0., -179.9998], [0., 179.9998]),
            ([89.9999, 0.], [89.9999, 180.]),
            ([-89.9999, 0.], [-89.9999, -180.]),
            ([80., 25.], [80.0001, 25.001]),
        ] {
            let spawn = point(spawn);
            assert!(spawn.contains_point(&player));
            let tree = RTree::bulk_load(vec![spawn]);
            assert_eq!(tree.locate_all_at_point(&player).count(), 1);
            let outside = [player[0] - player[0].signum() * 0.01, player[1]];
            assert!(!spawn.contains_point(&outside));
            assert_eq!(tree.locate_all_at_point(&outside).count(), 0);
        }
    }

    #[test]
    fn identical_coordinates_have_zero_distance() {
        let spawn = point([51.5, -0.12]);
        assert_eq!(spawn.distance_2(&spawn.center), 0.);
        assert!(spawn.contains_point(&spawn.center));
    }

    #[test]
    fn antipodal_roundoff_stays_finite() {
        // This antipodal pair can round a above 1 before clamping.
        let spawn = point([-0.0074, 0.]);
        let distance = spawn.distance_2(&[0.0074, 180.]);
        assert!(distance.is_finite());
        assert!((distance - std::f64::consts::PI * EARTH_RADIUS_M).abs() < 0.2);
        assert!(!spawn.contains_point(&[0.0074, 180.]));
    }

    #[test]
    fn configured_radius_is_preserved() {
        let player = [(75. / EARTH_RADIUS_M).to_degrees(), 0.];
        assert!(!point([0., 0.]).contains_point(&player));
        let spawn = Point::new(80., 20, [0., 0.]);
        assert!(spawn.contains_point(&player));
        assert_eq!(
            RTree::bulk_load(vec![spawn])
                .locate_all_at_point(&player)
                .count(),
            1
        );
    }
}
