use std::cell::RefCell;

use macros::time;
use model::api::{Precision, single_vec::SingleVec};
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::utils;

/// Represents a bounding box in lat/lon coordinates
#[derive(Debug, Clone, Copy)]
struct BBox {
    min_lat: Precision,
    max_lat: Precision,
    min_lon: Precision,
    max_lon: Precision,
}

impl BBox {
    /// Create a bounding box from a vector of points
    fn from_points(points: &[[Precision; 2]]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }

        let mut min_lat = Precision::INFINITY;
        let mut max_lat = Precision::NEG_INFINITY;
        let mut min_lon = Precision::INFINITY;
        let mut max_lon = Precision::NEG_INFINITY;

        for &[lat, lon] in points {
            min_lat = min_lat.min(lat);
            max_lat = max_lat.max(lat);
            min_lon = min_lon.min(lon);
            max_lon = max_lon.max(lon);
        }

        Some(BBox {
            min_lat,
            max_lat,
            min_lon,
            max_lon,
        })
    }

    /// Expand the bounding box by a given radius (in degrees)
    /// This ensures cluster centers near edges can still cover boundary points
    fn expand(&self, radius_deg: Precision) -> Self {
        BBox {
            min_lat: self.min_lat - radius_deg,
            max_lat: self.max_lat + radius_deg,
            min_lon: self.min_lon - radius_deg,
            max_lon: self.max_lon + radius_deg,
        }
    }
}

thread_local! {
    static TLS_RNG: RefCell<SmallRng> = RefCell::new(SmallRng::from_os_rng());
}

pub fn generate_clusters_from_points(
    points: &[[Precision; 2]],
    radius_meters: Precision,
    density: usize,
) -> SingleVec {
    let center = utils::centroid(points);
    generate_cluster_candidates_grid(points, meters_to_degrees(radius_meters, center[0]), density)
}

/// Generate candidates using a grid-based approach for better coverage
#[time()]
pub fn generate_cluster_candidates_grid(
    points: &[[Precision; 2]],
    cluster_radius_deg: Precision,
    grid_density: usize,
) -> SingleVec {
    // Quick exit for empty input
    if points.is_empty() || grid_density == 0 {
        return Vec::new();
    }

    let bbox = match BBox::from_points(points) {
        Some(b) => b.expand(cluster_radius_deg),
        None => return Vec::new(),
    };

    let n = grid_density as Precision;
    let lat_step = (bbox.max_lat - bbox.min_lat) / n;
    let lon_step = (bbox.max_lon - bbox.min_lon) / n;

    // Degenerate bbox: fall back to a single point to avoid NaNs/zero step
    if !lat_step.is_finite() || !lon_step.is_finite() || lat_step == 0.0 || lon_step == 0.0 {
        let lat = 0.5 * (bbox.min_lat + bbox.max_lat);
        let lon = 0.5 * (bbox.min_lon + bbox.max_lon);
        return vec![[lat, lon]];
    }

    // Known final length → collect directly, no intermediate Vecs per row.
    let total = grid_density * grid_density;

    (0..total)
        .into_par_iter()
        .map(|idx| {
            // Map a flat index -> (i, j)
            let i = idx / grid_density;
            let j = idx % grid_density;

            // Jitter within the cell [0, step)
            let (dj_lat, dj_lon) = TLS_RNG.with(|cell| {
                let mut rng = cell.borrow_mut();
                (
                    rng.random::<Precision>() * lat_step,
                    rng.random::<Precision>() * lon_step,
                )
            });

            let lat = bbox.min_lat + (i as Precision) * lat_step + dj_lat;
            let lon = bbox.min_lon + (j as Precision) * lon_step + dj_lon;
            [lat, lon]
        })
        .collect()
}

/// Convert radius in meters to approximate degrees
/// This is a rough approximation; for precise calculations use a geodesic library
pub fn meters_to_degrees(radius_meters: Precision, latitude: Precision) -> Precision {
    const EARTH_RADIUS: Precision = 6371000.0; // meters
    let lat_rad = latitude.to_radians();
    let lat_deg = radius_meters / EARTH_RADIUS * (180.0 / std::f64::consts::PI);
    let lon_deg = lat_deg / lat_rad.cos();
    lat_deg.max(lon_deg) // Use the larger for safety
}
