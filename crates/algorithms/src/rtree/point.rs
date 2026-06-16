use std::{
    fmt::Display,
    hash::{Hash, Hasher},
};

use geo::{Coord, Destination, Distance, Haversine};
use geohash::encode;
use koji_core::Precision;
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── Point::new / basic properties ─────────────────────────────────────────

    #[test]
    fn center_matches_input() {
        let center = [40.0_f64, -74.0_f64];
        let p = Point::new(70.0, 20, center);
        assert_eq!(p.center[0], center[0]);
        assert_eq!(p.center[1], center[1]);
    }

    #[test]
    fn cell_id_reproducible() {
        let p1 = Point::new(70.0, 20, [40.0, -74.0]);
        let p2 = Point::new(70.0, 20, [40.0, -74.0]);
        assert_eq!(p1.cell_id, p2.cell_id);
    }

    #[test]
    fn different_coords_different_cell_id() {
        let p1 = Point::new(70.0, 20, [40.0, -74.0]);
        let p2 = Point::new(70.0, 20, [-33.0, 151.0]);
        assert_ne!(p1.cell_id, p2.cell_id);
    }

    // ── PointDistance: distance_2 / contains_point ────────────────────────────

    #[test]
    fn distance_to_self_is_zero() {
        let p = Point::new(70.0, 20, [40.0, -74.0]);
        let d = p.distance_2(&p.center);
        assert!(d < 1.0, "distance to self should be ~0, got {d}");
    }

    #[test]
    fn contains_point_true_when_within_radius() {
        let radius = 1_000.0; // 1 km
        let p = Point::new(radius, 20, [40.0, -74.0]);
        // ~11 m away — well inside 1 km radius.
        let nearby = [40.0001, -74.0];
        assert!(p.contains_point(&nearby));
    }

    #[test]
    fn contains_point_false_when_outside_radius() {
        let radius = 10.0; // 10 m
        let p = Point::new(radius, 20, [40.0, -74.0]);
        // ~111 km away.
        let far = [41.0, -74.0];
        assert!(!p.contains_point(&far));
    }

    // ── PartialEq / Hash: same cell_id = equal regardless of coords ───────────

    #[test]
    fn eq_based_on_cell_id_not_radius() {
        let p1 = Point::new(70.0, 20, [40.0, -74.0]);
        let p2 = Point::new(500.0, 20, [40.0, -74.0]); // different radius, same coords
        assert_eq!(p1, p2, "same coords at same cell level must be equal");
    }

    // ── Display ───────────────────────────────────────────────────────────────

    #[test]
    fn display_contains_lat_lon() {
        let p = Point::new(70.0, 20, [40.0, -74.0]);
        let s = format!("{p}");
        assert!(s.contains("40.000000"), "missing lat in display: {s}");
        assert!(s.contains("-74.000000"), "missing lon in display: {s}");
    }

    // ── _get_geohash ──────────────────────────────────────────────────────────

    #[test]
    fn geohash_length_12() {
        let p = Point::new(70.0, 20, [40.0, -74.0]);
        let hash = p._get_geohash();
        assert_eq!(hash.len(), 12, "expected 12-char geohash, got: {hash}");
    }

    // ── SortDedupe for Vec<&Point> ────────────────────────────────────────────

    #[test]
    fn sort_dedupe_removes_duplicates() {
        let p = Point::new(70.0, 20, [40.0, -74.0]);
        let q = Point::new(70.0, 20, [41.0, -74.0]);
        let mut refs: Vec<&Point> = vec![&p, &q, &p];
        refs.sort_dedupe();
        assert_eq!(refs.len(), 2, "dedup should remove duplicate cell_id");
    }

    #[test]
    fn sort_dedupe_empty_ok() {
        let mut refs: Vec<&Point> = vec![];
        refs.sort_dedupe();
        assert!(refs.is_empty());
    }
}
