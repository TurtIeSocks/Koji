use std::cell::RefCell;

use koji_core::{KojiBbox, Precision, SingleVec};
use macros::time;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::utils;

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

    let bbox = match KojiBbox::from_points(points) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // ── generate_cluster_candidates_grid ──────────────────────────────────────

    #[test]
    fn empty_points_returns_empty() {
        let result = generate_cluster_candidates_grid(&[], 0.001, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn zero_density_returns_empty() {
        let result = generate_cluster_candidates_grid(&[[40.0, -74.0]], 0.001, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn density_n_returns_n_squared_candidates() {
        let pts = vec![[40.0, -74.0], [40.01, -73.99]];
        let density = 4;
        let result = generate_cluster_candidates_grid(&pts, 0.001, density);
        assert_eq!(
            result.len(),
            density * density,
            "expected {n}^2={t} candidates, got {got}",
            n = density,
            t = density * density,
            got = result.len()
        );
    }

    #[test]
    fn candidates_all_have_valid_lat_lon() {
        let pts = vec![[40.0, -74.0], [40.1, -73.9]];
        let result = generate_cluster_candidates_grid(&pts, 0.01, 5);
        for [lat, lon] in &result {
            assert!(lat.is_finite(), "lat is not finite: {lat}");
            assert!(lon.is_finite(), "lon is not finite: {lon}");
        }
    }

    #[test]
    fn single_point_expanded_bbox_returns_density_squared() {
        // A single point's bbox has zero area, but expand() pads it so the step
        // is non-zero → normal density^2 candidates are returned.
        let result = generate_cluster_candidates_grid(&[[40.0, -74.0]], 0.001, 4);
        // expand() makes the bbox non-degenerate → density^2 = 16 candidates.
        assert_eq!(
            result.len(),
            16,
            "single point with expand should yield density^2 candidates, got {}",
            result.len()
        );
    }

    // ── generate_clusters_from_points ─────────────────────────────────────────

    #[test]
    fn generate_clusters_from_points_returns_n_squared() {
        let pts = vec![[40.0, -74.0], [40.1, -73.9]];
        let density = 3;
        let result = generate_clusters_from_points(&pts, 70.0, density);
        assert_eq!(result.len(), density * density);
    }

    // ── meters_to_degrees ─────────────────────────────────────────────────────

    #[test]
    fn meters_to_degrees_positive_and_finite() {
        let deg = meters_to_degrees(70.0, 40.0);
        assert!(deg > 0.0, "expected positive degrees, got {deg}");
        assert!(deg.is_finite());
    }

    #[test]
    fn meters_to_degrees_zero_is_zero() {
        assert_eq!(meters_to_degrees(0.0, 40.0), 0.0);
    }

    #[test]
    fn meters_to_degrees_larger_radius_larger_degrees() {
        let d1 = meters_to_degrees(70.0, 40.0);
        let d2 = meters_to_degrees(700.0, 40.0);
        assert!(d2 > d1, "larger radius should give larger degree value");
    }
}

