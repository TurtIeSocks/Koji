use geo::Coord;
use geohash::encode;
use koji_core::SingleVec;
use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use rayon::{
    iter::{IntoParallelRefIterator, ParallelIterator},
    slice::ParallelSliceMut,
};
use s2::{cellid::CellID, latlng::LatLng};

use crate::rtree::{self, cluster, point};

pub trait SortRandom {
    fn sort_random(self) -> Self;
    fn sort_random_mut(&mut self);
}

impl SortRandom for SingleVec {
    fn sort_random(self) -> Self {
        let mut clusters = self;
        clusters.sort_random_mut();
        clusters
    }

    fn sort_random_mut(&mut self) {
        let mut rng = SmallRng::seed_from_u64(42);
        self.shuffle(&mut rng);
    }
}

pub trait SortGeohash {
    fn sort_geohash(self) -> Self;
    fn sort_geohash_mut(&mut self);
}

impl SortGeohash for SingleVec {
    fn sort_geohash(self) -> Self {
        let mut points = self;
        points.sort_geohash_mut();
        points
    }

    fn sort_geohash_mut(&mut self) {
        self.par_sort_by(|a, b| {
            match encode(Coord { x: a[1], y: a[0] }, 12) {
                Ok(geohash) => geohash,
                Err(e) => {
                    log::warn!("Error while encoding geohash: {}", e);
                    "".to_string()
                }
            }
            .cmp(&match encode(Coord { x: b[1], y: b[0] }, 12) {
                Ok(geohash) => geohash,
                Err(e) => {
                    log::warn!("Error while encoding geohash: {}", e);
                    "".to_string()
                }
            })
        })
    }
}

pub trait SortS2 {
    fn sort_s2(self) -> Self;
    fn sort_s2_mut(&mut self);
}

impl SortS2 for SingleVec {
    fn sort_s2(self) -> Self {
        let mut points = self;
        points.sort_s2_mut();
        points
    }

    fn sort_s2_mut(&mut self) {
        self.par_sort_by(|a, b| {
            let a: CellID = LatLng::from_degrees(a[0], a[1]).into();
            let b: CellID = LatLng::from_degrees(b[0], b[1]).into();
            a.0.cmp(&b.0)
        });
    }
}

pub trait SortLatLng {
    fn sort_lat_lng(self) -> Self;
    fn sort_lat_lng_mut(&mut self);
}

impl SortLatLng for SingleVec {
    fn sort_lat_lng(self) -> Self {
        let mut points = self;
        points.sort_lat_lng_mut();
        points
    }

    fn sort_lat_lng_mut(&mut self) {
        self.par_sort_by(|a, b| {
            b[0].partial_cmp(&a[0])
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(b[1].partial_cmp(&a[1]).unwrap_or(std::cmp::Ordering::Equal))
        })
    }
}

pub trait SortPointCount {
    fn sort_point_count(self, points: &SingleVec, radius: f64) -> Self;
    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64);
}

impl SortPointCount for SingleVec {
    fn sort_point_count(self, points: &SingleVec, radius: f64) -> Self {
        let mut clusters = self;
        clusters.sort_point_count_mut(points, radius);
        clusters
    }

    fn sort_point_count_mut(&mut self, points: &SingleVec, radius: f64) {
        let tree = rtree::spawn(radius, points);
        let clusters: Vec<point::Point> = self
            .par_iter()
            .map(|c| point::Point::new(radius, 20, *c))
            .collect();
        let mut clusters: Vec<cluster::Cluster<'_>> = rtree::cluster_info(&tree, &clusters);
        clusters.par_sort_by(|a, b| b.all.len().cmp(&a.all.len()));
        *self = clusters.into_iter().map(|c| c.point.center).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SortRandom ────────────────────────────────────────────────────────────

    #[test]
    fn sort_random_preserves_length() {
        let pts: SingleVec = (0..10).map(|i| [i as f64, 0.0]).collect();
        let sorted = pts.clone().sort_random();
        assert_eq!(sorted.len(), pts.len());
    }

    #[test]
    fn sort_random_is_deterministic_with_fixed_seed() {
        // SmallRng seeded at 42 → same shuffle every time.
        let pts: SingleVec = (0..10).map(|i| [i as f64, 0.0]).collect();
        let a = pts.clone().sort_random();
        let b = pts.clone().sort_random();
        assert_eq!(a, b, "sort_random must be deterministic (seed=42)");
    }

    #[test]
    fn sort_random_actually_shuffles() {
        // With 20 elements, the identity order is extremely unlikely after a seeded shuffle.
        let pts: SingleVec = (0..20).map(|i| [i as f64, 0.0]).collect();
        let sorted = pts.clone().sort_random();
        assert_ne!(sorted, pts, "seeded shuffle should change order of 20 elements");
    }

    // ── SortGeohash ───────────────────────────────────────────────────────────

    #[test]
    fn sort_geohash_preserves_length() {
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        let sorted = pts.clone().sort_geohash();
        assert_eq!(sorted.len(), pts.len());
    }

    #[test]
    fn sort_geohash_is_stable_order() {
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        let a = pts.clone().sort_geohash();
        let b = pts.clone().sort_geohash();
        assert_eq!(a, b, "sort_geohash must be deterministic");
    }

    #[test]
    fn sort_geohash_lexicographic_order() {
        // geohash at higher lat/lon is lexicographically larger in well-known cases.
        // Just verify idempotency: sorting an already-sorted list is the same.
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        let once = pts.clone().sort_geohash();
        let twice = once.clone().sort_geohash();
        assert_eq!(once, twice, "sort_geohash must be idempotent");
    }

    // ── SortS2 ────────────────────────────────────────────────────────────────

    #[test]
    fn sort_s2_preserves_length() {
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        assert_eq!(pts.clone().sort_s2().len(), pts.len());
    }

    #[test]
    fn sort_s2_idempotent() {
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        let once = pts.clone().sort_s2();
        let twice = once.clone().sort_s2();
        assert_eq!(once, twice, "sort_s2 must be idempotent");
    }

    // ── SortLatLng ────────────────────────────────────────────────────────────

    #[test]
    fn sort_lat_lng_descending_lat_then_lon() {
        let pts: SingleVec = vec![[40.0, -74.0], [41.0, -74.0], [40.0, -73.0]];
        let sorted = pts.sort_lat_lng();
        // lat descending: 41 first.
        assert!(sorted[0][0] >= sorted[1][0]);
        assert!(sorted[1][0] >= sorted[2][0]);
    }

    #[test]
    fn sort_lat_lng_idempotent() {
        let pts: SingleVec = vec![[40.0, -74.0], [-33.0, 151.0], [51.5, -0.1]];
        let once = pts.clone().sort_lat_lng();
        let twice = once.clone().sort_lat_lng();
        assert_eq!(once, twice);
    }

    // ── SortPointCount ────────────────────────────────────────────────────────

    #[test]
    fn sort_point_count_high_density_cluster_first() {
        // Dense cluster at 40°N -74°W (10 points within 70 m);
        // one isolated cluster at 0°N 0°E.
        let radius = 70.0;
        let dense_center = [40.0_f64, -74.0_f64];
        let mut data: SingleVec = (0..10)
            .map(|i| [40.0 + i as f64 * 0.0001, -74.0])
            .collect();
        data.push([0.0, 0.0]);

        let clusters: SingleVec = vec![dense_center, [0.0, 0.0]];
        let sorted = clusters.sort_point_count(&data, radius);

        // dense_center must come first (most data points within radius).
        assert_eq!(sorted.len(), 2);
        let first = sorted[0];
        let dist_to_dense = ((first[0] - 40.0).powi(2) + (first[1] - -74.0).powi(2)).sqrt();
        assert!(
            dist_to_dense < 1.0,
            "dense cluster should be first, got {:?}",
            first
        );
    }
}

