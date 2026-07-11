use koji_core::{KojiBbox, Precision, SingleVec};
use macros::time;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::utils;

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

            // Jitter within the cell [0, step) — DETERMINISTIC, keyed on the
            // cell index (splitmix64). The jitter's job is only to break grid
            // alignment with the data; an OS-seeded RNG here made every greedy
            // run produce different candidates → different clusters → the
            // long-standing "~1% nondeterminism". Same request, same route.
            let h = splitmix64(idx as u64);
            let dj_lat = unit_f64(h) * lat_step;
            let dj_lon = unit_f64(splitmix64(h)) * lon_step;

            let lat = bbox.min_lat + (i as Precision) * lat_step + dj_lat;
            let lon = bbox.min_lon + (j as Precision) * lon_step + dj_lon;
            [lat, lon]
        })
        .collect()
}

/// SplitMix64 — tiny, high-quality, stateless mixer for index-keyed jitter.
#[inline]
fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// Map a u64 to [0, 1) with 53-bit precision.
#[inline]
fn unit_f64(h: u64) -> Precision {
    (h >> 11) as Precision / (1u64 << 53) as Precision
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

    // ── meters_to_degrees: latitude scaling ───────────────────────────────────

    #[test]
    fn meters_to_degrees_at_equator_smallest() {
        // At the equator lat_rad = 0 → cos(0) = 1 → lon_deg = lat_deg.
        // lat_deg = 70 / 6371000 * 180/π ≈ 0.000629°.
        let d_equator = meters_to_degrees(70.0, 0.0);
        let d_mid_lat = meters_to_degrees(70.0, 45.0);
        // At higher latitude cos shrinks, so lon_deg grows.
        // meters_to_degrees returns max(lat_deg, lon_deg).
        assert!(
            d_mid_lat >= d_equator,
            "higher lat should need >= degrees for same meter radius; equator={d_equator}, 45°={d_mid_lat}"
        );
    }

    #[test]
    fn meters_to_degrees_southern_hemisphere() {
        // Negative latitude should behave like positive (cos is symmetric).
        let d_pos = meters_to_degrees(70.0, 45.0);
        let d_neg = meters_to_degrees(70.0, -45.0);
        assert!(
            (d_pos - d_neg).abs() < 1e-10,
            "±45° should give same result; got {d_pos}, {d_neg}"
        );
    }

    // ── generate_cluster_candidates_grid: bbox containment ────────────────────

    #[test]
    fn candidates_within_expanded_bbox() {
        let pts = vec![[40.0, -74.0], [40.1, -73.9]];
        let radius_deg = 0.01;
        let result = generate_cluster_candidates_grid(&pts, radius_deg, 4);
        // Expanded bbox: [39.99, -74.01] to [40.11, -73.89].
        for [lat, lon] in &result {
            assert!(
                *lat >= 39.98 && *lat <= 40.12,
                "lat {lat} out of expected expanded range"
            );
            assert!(
                *lon >= -74.02 && *lon <= -73.88,
                "lon {lon} out of expected expanded range"
            );
        }
    }

    #[test]
    fn large_density_produces_correct_count() {
        let pts = vec![[40.0, -74.0], [40.1, -73.9]];
        let density = 16;
        let result = generate_cluster_candidates_grid(&pts, 0.001, density);
        assert_eq!(
            result.len(),
            density * density,
            "expected {n}² = {t}, got {got}",
            n = density,
            t = density * density,
            got = result.len()
        );
    }
}
