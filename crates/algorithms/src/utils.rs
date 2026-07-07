use std::collections::VecDeque;

#[cfg(feature = "native")]
use colored::Colorize;
use geo::Coord;
use geohash::encode;
use hashbrown::HashSet;
use koji_core::Precision;
use koji_core::{PointArray, SingleVec};

use crate::stats::Stats;

pub fn centroid(coords: &[[Precision; 2]]) -> PointArray {
    let (mut x, mut y, mut z) = (0.0, 0.0, 0.0);

    for loc in coords.iter() {
        let lat = loc[0].to_radians();
        let lon = loc[1].to_radians();

        x += lat.cos() * lon.cos();
        y += lat.cos() * lon.sin();
        z += lat.sin();
    }

    let number_of_locations = coords.len() as Precision;
    x /= number_of_locations;
    y /= number_of_locations;
    z /= number_of_locations;

    let hyp = (x * x + y * y).sqrt();
    let lon = y.atan2(x);
    let lat = z.atan2(hyp);

    [lat.to_degrees(), lon.to_degrees()]
}

#[cfg(feature = "native")]
pub fn info_log(file_name: &str, message: String) -> String {
    format!(
        "\r{}{}Z {}  {}{} {}",
        "[".black(),
        chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
        "INFO".green(),
        file_name,
        "]".black(),
        message
    )
}

pub fn rotate_to_best(clusters: SingleVec, stats: &Stats) -> SingleVec {
    let mut final_clusters = VecDeque::<PointArray>::new();

    let best_cluster_set = stats
        .best_clusters
        .iter()
        .map(|x| encode(Coord { x: x[1], y: x[0] }, 12).unwrap())
        .collect::<HashSet<String>>();
    let mut rotate_count = 0;
    for (i, [lat, lon]) in clusters.into_iter().enumerate() {
        if best_cluster_set.contains(&encode(Coord { x: lon, y: lat }, 12).unwrap()) {
            rotate_count = i;
            log::debug!("Found Best! {}, {} - {}", lat, lon, i);
        }
        final_clusters.push_back([lat, lon]);
    }
    final_clusters.rotate_left(rotate_count);

    final_clusters.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::Stats;

    // ── centroid ───────────────────────────────────────────────────────────────

    #[test]
    fn centroid_single_point_is_itself() {
        let pts = [[40.0_f64, -74.0_f64]];
        let c = centroid(&pts);
        assert!((c[0] - 40.0).abs() < 1e-6);
        assert!((c[1] - -74.0).abs() < 1e-6);
    }

    #[test]
    fn centroid_two_symmetric_points() {
        // [40, 0] and [40, 2] — centroid lon ≈ 1 (symmetric), lat ≈ 40.
        let pts = [[40.0_f64, 0.0_f64], [40.0, 2.0]];
        let c = centroid(&pts);
        assert!((c[1] - 1.0).abs() < 0.01, "lon centroid: {}", c[1]);
    }

    #[test]
    fn centroid_many_points_stays_bounded() {
        // 100 points all at the same location → centroid = that location.
        let pts: Vec<[Precision; 2]> = (0..100).map(|_| [48.8566, 2.3522]).collect();
        let c = centroid(&pts);
        assert!((c[0] - 48.8566).abs() < 0.001);
        assert!((c[1] - 2.3522).abs() < 0.001);
    }

    // ── rotate_to_best ─────────────────────────────────────────────────────────

    #[test]
    fn rotate_to_best_puts_best_cluster_first() {
        // Two clusters; "best" has 2 points, "other" has 1.
        // We set best_clusters so rotate_to_best should put it at index 0.
        let best = [40.0_f64, -74.0_f64];
        let other = [41.0_f64, -74.0_f64];

        let mut stats = Stats::new("test".into(), 1);
        stats.best_clusters = vec![best];

        let clusters = vec![other, best];
        let rotated = rotate_to_best(clusters, &stats);

        assert_eq!(rotated.len(), 2);
        // best must be first (or tied for first if already there)
        assert!(
            rotated.iter().any(|c| (c[0] - best[0]).abs() < 1e-6),
            "best cluster missing from output"
        );
        // The "best" cluster should be at index 0 after rotation.
        assert!(
            (rotated[0][0] - best[0]).abs() < 1e-6,
            "rotate_to_best must put the best cluster first; got {:?}",
            rotated[0]
        );
    }

    #[test]
    fn rotate_to_best_empty_returns_empty() {
        let mut stats = Stats::new("test".into(), 1);
        stats.best_clusters = vec![];
        let rotated = rotate_to_best(vec![], &stats);
        assert!(rotated.is_empty());
    }

    #[test]
    fn rotate_to_best_already_first_is_noop() {
        let best = [40.0_f64, -74.0_f64];
        let other = [41.0_f64, -74.0_f64];

        let mut stats = Stats::new("test".into(), 1);
        stats.best_clusters = vec![best];

        // best is already first → rotate_count = 0.
        let clusters = vec![best, other];
        let rotated = rotate_to_best(clusters, &stats);
        assert_eq!(rotated.len(), 2);
        assert!((rotated[0][0] - best[0]).abs() < 1e-6);
    }

    // ── centroid: geographic edge cases ───────────────────────────────────────

    #[test]
    fn centroid_equatorial_point() {
        // Point on the equator at prime meridian: centroid = itself.
        let pts = [[0.0_f64, 0.0_f64]];
        let c = centroid(&pts);
        assert!(c[0].abs() < 1e-6, "lat: {}", c[0]);
        assert!(c[1].abs() < 1e-6, "lon: {}", c[1]);
    }

    #[test]
    fn centroid_three_collinear_meridian_points() {
        // Three points on the same meridian (lon=0), symmetric about equator.
        // Lat 10°, 0°, -10° → centroid lat ≈ 0°.
        let pts = [[10.0_f64, 0.0_f64], [0.0, 0.0], [-10.0, 0.0]];
        let c = centroid(&pts);
        assert!(c[0].abs() < 0.1, "centroid lat should be ~0, got {}", c[0]);
    }

    #[test]
    fn centroid_large_cluster_stays_within_bbox() {
        // 9 points in a 3×3 grid around NYC → centroid is inside the grid bbox.
        let pts: Vec<[Precision; 2]> = (0..3)
            .flat_map(|r| {
                (0..3).map(move |c| [40.0 + r as Precision * 0.01, -74.0 + c as Precision * 0.01])
            })
            .collect();
        let c = centroid(&pts);
        assert!(c[0] >= 40.0 && c[0] <= 40.02, "lat {}", c[0]);
        assert!(c[1] >= -74.0 && c[1] <= -73.98, "lon {}", c[1]);
    }

    #[test]
    fn centroid_southern_hemisphere() {
        // Single point at Sydney: centroid = itself.
        let pts = [[-33.8688_f64, 151.2093_f64]];
        let c = centroid(&pts);
        assert!((c[0] - (-33.8688)).abs() < 0.001);
        assert!((c[1] - 151.2093).abs() < 0.001);
    }

    #[test]
    fn centroid_two_antipodal_lons_same_lat() {
        // Two points same lat, lon 90° apart: centroid lon = 45°.
        let pts = [[40.0_f64, 0.0_f64], [40.0, 90.0]];
        let c = centroid(&pts);
        assert!(
            (c[1] - 45.0).abs() < 0.5,
            "centroid lon ≈ 45°, got {}",
            c[1]
        );
    }

    // ── rotate_to_best: multiple best clusters ─────────────────────────────────

    #[test]
    fn rotate_to_best_uses_last_match_as_rotate_count() {
        // Two clusters match best_clusters: the last one in the original list
        // determines rotate_count (overwrites earlier matches in the loop).
        // We just verify the total length is preserved.
        let a = [40.0_f64, -74.0_f64];
        let b = [41.0_f64, -74.0_f64];
        let c = [42.0_f64, -74.0_f64];

        let mut stats = Stats::new("test".into(), 1);
        // Both a and b are "best" → either could become first.
        stats.best_clusters = vec![a, b];

        let clusters = vec![c, a, b];
        let rotated = rotate_to_best(clusters, &stats);
        assert_eq!(rotated.len(), 3, "no points should be lost");
        // Either a or b is first (whichever is last in the list scan).
        let first = rotated[0];
        assert!(
            (first[0] - a[0]).abs() < 1e-6 || (first[0] - b[0]).abs() < 1e-6,
            "first should be one of the best clusters, got {:?}",
            first
        );
    }

    #[test]
    fn rotate_to_best_no_match_keeps_original_order() {
        // best_clusters doesn't match any cluster → rotate_count stays 0 → no rotation.
        let a = [40.0_f64, -74.0_f64];
        let b = [41.0_f64, -74.0_f64];
        let unknown = [50.0_f64, 0.0_f64]; // not in clusters

        let mut stats = Stats::new("test".into(), 1);
        stats.best_clusters = vec![unknown];

        let clusters = vec![a, b];
        let rotated = rotate_to_best(clusters, &stats);
        assert_eq!(rotated.len(), 2);
        // No match → rotate_left(0) → original order preserved.
        assert!(
            (rotated[0][0] - a[0]).abs() < 1e-6,
            "first should still be a"
        );
    }
}
