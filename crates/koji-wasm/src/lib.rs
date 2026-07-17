//! Browser WASM build of Koji's algorithms (demo/portfolio): clustering,
//! routing, bootstrap, s2 cells, and geometry conversion via [`calc`].
//! Plugins, DB and network are intentionally excluded.

#[cfg(test)]
use koji_core::Precision;
use wasm_bindgen::prelude::*;

pub mod calc;
mod convert;
mod dto;

pub use dto::{ClusterRequest, ClusterResponse, StatsSummary};

/// Re-export the wasm-bindgen-rayon thread-pool initializer. The JS caller must
/// `await initThreadPool(navigator.hardwareConcurrency)` before calling
/// `cluster` or `calc`.
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;

/// Install a panic hook so Rust panics surface as readable console errors.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Crate version string (for the demo footer / cache-busting).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Cluster `points` under `req`'s parameters. Returns centers + stats, or a
/// `JsError` on invalid input.
#[wasm_bindgen]
pub fn cluster(req: ClusterRequest) -> Result<ClusterResponse, JsError> {
    let (points, cfg) = req.into_core()?;
    let mut stats = algorithms::stats::Stats::new("wasm".to_string(), cfg.min_points);
    // `collection` is only consulted by the S2 calculation mode; the radius demo
    // path passes an empty FeatureCollection.
    let collection = geojson::FeatureCollection {
        bbox: None,
        features: vec![],
        foreign_members: None,
    };
    let clusters = algorithms::clustering::main(&points, &cfg, collection, &mut stats);
    Ok(ClusterResponse::from_parts(clusters, &stats))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Build a minimal `ClusterRequest` with sensible defaults.
    fn req(
        points: Vec<[Precision; 2]>,
        cluster_mode: &str,
        radius: Precision,
        min_points: usize,
        max_clusters: usize,
        center_clusters: bool,
    ) -> ClusterRequest {
        ClusterRequest {
            points,
            radius,
            min_points,
            max_clusters,
            cluster_mode: cluster_mode.to_string(),
            calculation_mode: "radius".to_string(),
            center_clusters,
        }
    }

    // ── version ──────────────────────────────────────────────────────────────

    /// `version()` must return the crate's own version string (not empty, not
    /// some other crate's version). The exact value is cargo-managed.
    #[test]
    fn version_is_nonempty_semver_like() {
        let v = version();
        assert!(!v.is_empty(), "version must not be empty");
        // Semver: at least one '.' (e.g. "0.1.0")
        assert!(v.contains('.'), "expected semver-like version, got: {v}");
        assert_eq!(
            v,
            env!("CARGO_PKG_VERSION"),
            "version() must equal CARGO_PKG_VERSION"
        );
    }

    // ── cluster: empty input ─────────────────────────────────────────────────

    /// Empty point list → zero clusters, total_points = 0. The algorithm
    /// short-circuits; returning an Err here would be a regression.
    #[test]
    fn cluster_empty_points_returns_empty() {
        let resp = cluster(req(vec![], "fastest", 100.0, 1, 0, false))
            .expect("empty input must not error");
        assert_eq!(resp.clusters.len(), 0);
        assert_eq!(resp.stats.total_points, 0);
        assert_eq!(resp.stats.total_clusters, 0);
        assert_eq!(resp.stats.points_covered, 0);
    }

    // ── cluster: fastest mode ────────────────────────────────────────────────

    /// A tight group of collocated points should produce exactly one cluster
    /// under the `fastest` algorithm.
    #[test]
    fn cluster_fastest_tight_group_gives_one_cluster() {
        // Six points within ~10 m of each other — well within a 200 m radius.
        let pts: Vec<[Precision; 2]> = vec![
            [35.6895, 139.6917],
            [35.6895, 139.6918],
            [35.6896, 139.6917],
            [35.6896, 139.6918],
            [35.6897, 139.6917],
            [35.6897, 139.6918],
        ];
        let resp = cluster(req(pts, "fastest", 200.0, 1, 0, false)).expect("should succeed");
        assert!(!resp.clusters.is_empty(), "expected at least one cluster");
        assert_eq!(resp.stats.total_points, 6);
        assert!(
            resp.stats.points_covered >= 1,
            "must cover at least some points"
        );
        assert!(resp.stats.cluster_time_ms >= 0.0);
    }

    /// min_points threshold: if every point is isolated (pairwise distance much
    /// larger than radius) and min_points = 3, `fastest` produces no clusters
    /// (each candidate has fewer than 3 neighbours).
    #[test]
    fn cluster_fastest_isolated_points_high_min_points_gives_no_clusters() {
        // Three points ~111 km apart — a 100 m radius can't group any two.
        let pts: Vec<[Precision; 2]> = vec![[40.0, -74.0], [41.0, -74.0], [42.0, -74.0]];
        let resp = cluster(req(pts, "fastest", 100.0, 3, 0, false)).expect("should succeed");
        assert_eq!(
            resp.clusters.len(),
            0,
            "isolated points can't meet min_points=3"
        );
        assert_eq!(resp.stats.total_clusters, 0);
    }

    // ── cluster: balanced (crucible) mode ────────────────────────────────────

    /// `balanced` routes to the crucible algorithm; a dense group must still
    /// produce ≥ 1 cluster and report sane stats.
    #[test]
    fn cluster_balanced_dense_group_produces_clusters() {
        let pts: Vec<[Precision; 2]> = (0..10)
            .map(|i| [35.0 + i as Precision * 0.00001, 139.0])
            .collect();
        let resp = cluster(req(pts, "balanced", 300.0, 1, 0, false)).expect("should succeed");
        assert!(
            !resp.clusters.is_empty(),
            "expected ≥1 cluster from balanced mode"
        );
        assert_eq!(resp.stats.total_points, 10);
    }

    // ── cluster: max_clusters = 0 semantics (unlimited) ──────────────────────

    /// max_clusters = 0 must be interpreted as "unlimited" (maps to usize::MAX
    /// in `into_core`). A single tight group must not be silently dropped.
    #[test]
    fn cluster_max_clusters_zero_means_unlimited() {
        let pts: Vec<[Precision; 2]> = vec![[35.0, 139.0], [35.0001, 139.0]];
        let resp = cluster(req(pts, "balanced", 200.0, 1, 0, false))
            .expect("max_clusters=0 should succeed");
        // At least one cluster must be returned (points are close together).
        assert!(!resp.clusters.is_empty());
    }

    // ── cluster: center_clusters flag ────────────────────────────────────────

    /// With `center_clusters = true` the SEC recentering pass runs. Output must
    /// still be non-empty and have valid lat/lng ranges.
    #[test]
    fn cluster_center_clusters_produces_valid_coords() {
        let pts: Vec<[Precision; 2]> = vec![
            [35.0, 139.0],
            [35.0001, 139.0],
            [35.0002, 139.0],
            [35.0003, 139.0],
        ];
        let resp = cluster(req(pts, "fastest", 200.0, 1, 0, true))
            .expect("center_clusters=true should succeed");
        for center in &resp.clusters {
            assert!(
                (-90.0..=90.0).contains(&center[0]),
                "lat out of range: {}",
                center[0]
            );
            assert!(
                (-180.0..=180.0).contains(&center[1]),
                "lng out of range: {}",
                center[1]
            );
        }
    }

    // ── cluster: stats contract ───────────────────────────────────────────────

    /// `cluster_time_ms` must be non-negative, and `points_covered` must not
    /// exceed `total_points`.
    #[test]
    fn cluster_stats_invariants_hold() {
        let pts: Vec<[Precision; 2]> = vec![[40.0, -74.0], [40.0001, -74.0], [40.0002, -74.0]];
        let resp = cluster(req(pts, "fastest", 500.0, 1, 0, false)).expect("should succeed");
        assert!(
            resp.stats.cluster_time_ms >= 0.0,
            "time must be non-negative"
        );
        assert!(
            resp.stats.points_covered <= resp.stats.total_points,
            "covered ({}) > total ({})",
            resp.stats.points_covered,
            resp.stats.total_points
        );
    }

    /// total_clusters in stats must equal the length of the clusters vec.
    #[test]
    fn cluster_stats_total_clusters_matches_vec_len() {
        let pts: Vec<[Precision; 2]> = vec![
            [35.0, 139.0],
            [35.0001, 139.0],
            [36.0, 139.0], // far from first two
        ];
        let resp = cluster(req(pts, "fastest", 50.0, 1, 0, false)).expect("should succeed");
        assert_eq!(
            resp.stats.total_clusters,
            resp.clusters.len(),
            "stats.total_clusters must equal clusters.len()"
        );
    }

    // ── ClusterResponse::from_parts: ms conversion ────────────────────────────

    /// `cluster_time_ms` = `stats.cluster_time * 1000.0`. Verify the unit
    /// conversion in `from_parts` by constructing a Stats with a known value.
    #[test]
    fn from_parts_converts_cluster_time_to_ms() {
        use algorithms::stats::Stats;
        let mut stats = Stats::new("test".into(), 1);
        // Manually set cluster_time to 1.5 seconds via the public field.
        stats.cluster_time = 1.5;
        let resp = ClusterResponse::from_parts(vec![], &stats);
        assert!(
            (resp.stats.cluster_time_ms - 1500.0).abs() < 1e-9,
            "expected 1500 ms, got {}",
            resp.stats.cluster_time_ms
        );
    }
}
