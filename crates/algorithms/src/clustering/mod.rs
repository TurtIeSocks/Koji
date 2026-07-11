#[cfg(test)]
use koji_core::Precision;
use std::vec;
use web_time::Instant;

#[cfg(feature = "native")]
use koji_plugins::PluginKind;

#[cfg(feature = "native")]
use crate::plugins;
use crate::stats::Stats;

use self::greedy::Greedy;

use super::*;

use geojson::FeatureCollection;
use koji_core::SingleVec;

mod calc_mode;
mod candidates;
mod cluster_mode;
mod config;
mod crucible;
mod fastest;
/// Shared planar-geometry primitives (deterministic Welzl MEC, circle
/// intersections) used by both the crucible and fastest clusterers.
pub(crate) mod geometry;
mod greedy;
mod partition;
mod s2;
pub(crate) mod select;

pub use calc_mode::CalculationMode;
pub use cluster_mode::ClusterMode;
pub use config::{ClusteringConfig, S2Config};

/// Warm-start entry point for incremental re-clustering (`Crucible::run_seeded`).
pub use crucible::Crucible;

pub fn main(
    data_points: &SingleVec,
    cfg: &ClusteringConfig,
    collection: FeatureCollection,
    stats: &mut Stats,
) -> SingleVec {
    if data_points.is_empty() {
        return vec![];
    }
    let time = Instant::now();
    let clusters = match cfg.calculation_mode {
        CalculationMode::S2 => collection
            .into_iter()
            .flat_map(|feature| {
                s2::cluster(
                    feature,
                    data_points,
                    cfg.s2.level,
                    cfg.s2.size,
                    cfg.min_points,
                )
            })
            .collect(),
        _ => match cfg.mode.clone() {
            ClusterMode::Fastest => fastest::main(data_points, cfg.radius, cfg.min_points),
            // ponytail: Better|Best intentionally share the crucible arm (Best is a future-algo slot).
            ClusterMode::Better | ClusterMode::Best => {
                // Cap enforcement is main()'s (point-weighted selection below) —
                // crucible's internal enforce ranks reps at weight 1 per distinct
                // cell (multiplicity-blind) and would pre-empt it.
                let crucible = crucible::Crucible {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: usize::MAX,
                };
                let mut centers = crucible.run(data_points);
                // With a BINDING cap the selection is only as good as its pool:
                // crucible optimizes full-coverage-minimal-count, so its pool has
                // no slack — capping a minimal tiling drops whole neighborhoods.
                // Union in greedy's dense-first solution so the selector can mix
                // both; Better's pool then strictly contains Balanced's.
                if cfg.max_clusters != usize::MAX {
                    let mut greedy = Greedy::default();
                    greedy
                        .set_cluster_mode(ClusterMode::Balanced)
                        .set_min_points(cfg.min_points)
                        .set_radius(cfg.radius);
                    centers.extend(greedy.run(data_points));
                }
                centers
            }
            ClusterMode::Honeycomb | ClusterMode::Fast | ClusterMode::Balanced => {
                let mut greedy = Greedy::default();
                greedy
                    .set_cluster_mode(cfg.mode.clone())
                    .set_min_points(cfg.min_points)
                    .set_radius(cfg.radius);
                greedy.run(data_points)
            }
            #[cfg(feature = "native")]
            ClusterMode::Custom(plugin) => {
                match plugins::resolve(PluginKind::Clustering, &plugin) {
                    Some(plugin_manager) => {
                        match plugin_manager.run(
                            data_points.clone(),
                            &plugins::merged_args(
                                PluginKind::Clustering,
                                &plugin,
                                &cfg.plugin_args,
                            ),
                        ) {
                            Ok(sorted_clusters) => sorted_clusters,
                            Err(e) => {
                                log::error!("Error while running plugin: {}", e);
                                vec![]
                            }
                        }
                    }
                    None => vec![],
                }
            }
            #[cfg(not(feature = "native"))]
            ClusterMode::Custom(_plugin) => {
                log::warn!(
                    "custom clustering plugins are unavailable in wasm; using the crucible algorithm"
                );
                // Same as the Better arm: main()'s weighted selection owns the cap.
                let crucible = crucible::Crucible {
                    radius: cfg.radius,
                    min_points: cfg.min_points,
                    max_clusters: usize::MAX,
                };
                crucible.run(data_points)
            }
        },
    };
    // max_clusters enforcement — uniform across EVERY mode (greedy, crucible,
    // fastest, s2, plugins): keep the max-coverage subset, never an arbitrary
    // prefix or truncation. Algorithms run unbounded so the selector sees the
    // full candidate pool; an internal early-stop would lock in a worse subset.
    // Deliberate trade-off: a BINDING cap now pays the full unbounded
    // clustering cost (measured ~2–2.7× vs the old early-stop on large inputs)
    // in exchange for keeping the best clusters instead of the first ones.
    // Capped requests are the rare path; revisit only if it hurts in practice.
    let clusters = if clusters.len() > cfg.max_clusters {
        match cfg.calculation_mode {
            CalculationMode::S2 => select::cap_s2_clusters(
                clusters,
                data_points,
                cfg.s2.level,
                cfg.s2.size,
                cfg.max_clusters,
            ),
            _ => select::cap_radius_clusters(clusters, data_points, cfg.radius, cfg.max_clusters),
        }
    } else {
        clusters
    };

    let clusters = if cfg.center_clusters {
        sec::with_data(cfg.radius, data_points, &clusters)
    } else {
        clusters
    };

    stats.set_cluster_time(time);
    stats.cluster_stats(cfg.radius, data_points, &clusters);
    stats.set_score();

    clusters
}

#[cfg(feature = "native")]
pub fn clustering_plugins() -> Vec<String> {
    plugins::plugin_names(PluginKind::Clustering)
}

#[cfg(not(feature = "native"))]
pub fn clustering_plugins() -> Vec<String> {
    vec![]
}

pub fn all_clustering_options() -> Vec<String> {
    let mut options = clustering_plugins();
    options.push("honeycomb".to_string());
    options.push("fastest".to_string());
    options.push("balanced".to_string());
    options.push("fast".to_string());
    options.push("better".to_string());
    options.push("best".to_string());
    options
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clustering::{CalculationMode, ClusterMode, ClusteringConfig, S2Config};
    use crate::stats::Stats;
    use geojson::FeatureCollection;

    fn make_cfg(mode: ClusterMode) -> ClusteringConfig {
        ClusteringConfig {
            mode,
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
            calculation_mode: CalculationMode::Radius,
            s2: S2Config::default(),
            center_clusters: false,
            plugin_args: String::new(),
        }
    }

    fn empty_collection() -> FeatureCollection {
        FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        }
    }

    // ── main: empty input always returns empty ─────────────────────────────────

    #[test]
    fn empty_data_returns_empty() {
        let empty: SingleVec = vec![];
        let mut stats = Stats::new("t".into(), 1);
        let result = main(
            &empty,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        assert!(result.is_empty());
    }

    // ── main: Fastest mode with valid data produces non-empty output ───────────

    #[test]
    fn fastest_mode_basic_data() {
        let pts = vec![
            [40.0_f64, -74.0_f64],
            [40.0001, -74.0001],
            [40.0002, -74.0002],
        ];
        let mut stats = Stats::new("t".into(), 1);
        let result = main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        assert!(!result.is_empty(), "Fastest mode should produce clusters");
        // Output stays within valid lat/lon range.
        for [lat, lon] in &result {
            assert!(lat.abs() < 90.0);
            assert!(lon.abs() < 180.0);
        }
    }

    // ── main: quality modes route to crucible ────────────────────────────────

    #[test]
    fn best_mode_produces_clusters() {
        // Dense grid of 100 points in a 0.05° × 0.05° area; radius 500 m ensures
        // many points fall within each disk → crucible finds valid clusters.
        let pts: Vec<[Precision; 2]> = (0..100)
            .map(|i| {
                [
                    40.0 + (i / 10) as Precision * 0.005,
                    -74.0 + (i % 10) as Precision * 0.005,
                ]
            })
            .collect();
        let mut cfg = make_cfg(ClusterMode::Best);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        assert!(
            !result.is_empty(),
            "Best mode should produce clusters for dense 100-point grid"
        );
    }

    #[test]
    fn better_mode_produces_clusters() {
        let pts: Vec<[Precision; 2]> = (0..100)
            .map(|i| {
                [
                    40.0 + (i / 10) as Precision * 0.005,
                    -74.0 + (i % 10) as Precision * 0.005,
                ]
            })
            .collect();
        let mut cfg = make_cfg(ClusterMode::Better);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        assert!(
            !result.is_empty(),
            "Better mode should produce clusters for dense 100-point grid"
        );
    }

    // ── main: stats populated after run ──────────────────────────────────────

    #[test]
    fn stats_cluster_time_set() {
        let pts = vec![[40.0, -74.0], [40.001, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        assert!(stats.cluster_time >= 0.0);
    }

    // ── all_clustering_options ────────────────────────────────────────────────

    #[test]
    fn all_clustering_options_contains_built_ins() {
        let opts = all_clustering_options();
        assert!(opts.contains(&"fastest".to_string()));
        assert!(opts.contains(&"best".to_string()));
        assert!(opts.contains(&"honeycomb".to_string()));
    }

    // ── main: Honeycomb mode ──────────────────────────────────────────────────

    #[test]
    fn honeycomb_mode_produces_clusters() {
        let pts: Vec<[Precision; 2]> = (0..20)
            .map(|i| [40.0 + i as Precision * 0.001, -74.0])
            .collect();
        let mut cfg = make_cfg(ClusterMode::Honeycomb);
        cfg.radius = 500.0;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        // Honeycomb is a layout mode; it should produce some output.
        assert!(!result.is_empty(), "Honeycomb mode should produce clusters");
    }

    // ── main: center_clusters=true path ───────────────────────────────────────

    #[test]
    fn center_clusters_path_runs_without_panic() {
        // center_clusters = true routes through sec::with_data.
        // Just verify it doesn't panic and returns something.
        let pts = vec![[40.0, -74.0], [40.0003, -74.0]];
        let mut cfg = make_cfg(ClusterMode::Fastest);
        cfg.center_clusters = true;
        let mut stats = Stats::new("t".into(), 1);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        // Result may be empty or non-empty depending on SEC; just no panic.
        assert!(result.len() <= pts.len() + 1);
    }

    // ── main: max_clusters caps EVERY mode and keeps the best clusters ────────

    /// 3 dense clumps (8 pts) + 3 small clumps (3 pts), all ~1.1 km apart.
    /// With cap = 3 the kept centers must be the DENSE clumps — selective
    /// best-N, not an arbitrary prefix/truncation.
    fn clumped_points() -> SingleVec {
        let mut pts: SingleVec = Vec::new();
        for (clump, n) in [(0, 8), (1, 8), (2, 8), (3, 3), (4, 3), (5, 3)] {
            let base_lat = 40.0 + clump as Precision * 0.01;
            for j in 0..n {
                pts.push([base_lat + j as Precision * 0.0001, -74.0]); // ~11 m apart
            }
        }
        pts
    }

    /// Count input points within `radius` meters of `center` (Haversine).
    fn covered_by(center: [Precision; 2], pts: &SingleVec, radius: Precision) -> usize {
        use geo::{Distance, Haversine};
        pts.iter()
            .filter(|p| {
                Haversine.distance(
                    geo::Point::new(center[1], center[0]),
                    geo::Point::new(p[1], p[0]),
                ) <= radius
            })
            .count()
    }

    fn assert_capped_and_best(mode: ClusterMode) {
        let pts = clumped_points();
        let mut cfg = make_cfg(mode.clone());
        cfg.min_points = 3;
        cfg.max_clusters = 3;
        let mut stats = Stats::new("t".into(), 3);
        let result = main(&pts, &cfg, empty_collection(), &mut stats);
        assert!(
            result.len() <= 3,
            "{mode:?}: max_clusters=3 must cap the solution, got {}",
            result.len()
        );
        assert!(
            !result.is_empty(),
            "{mode:?}: the cap should bind, not zero the output"
        );
        // Selective: every kept center sits on a DENSE clump (8 pts), never a
        // small one (3 pts) — with 6 candidate clumps and cap 3, max-coverage
        // must prefer the dense three.
        for c in &result {
            let covered = covered_by(*c, &pts, cfg.radius);
            assert!(
                covered >= 8,
                "{mode:?}: kept center {c:?} covers only {covered} points — a small clump was kept over a dense one"
            );
        }
    }

    #[test]
    fn fastest_mode_respects_max_clusters_selectively() {
        assert_capped_and_best(ClusterMode::Fastest);
    }

    #[test]
    fn balanced_mode_respects_max_clusters_selectively() {
        assert_capped_and_best(ClusterMode::Balanced);
    }

    #[test]
    fn better_mode_respects_max_clusters_selectively() {
        assert_capped_and_best(ClusterMode::Better);
    }

    #[test]
    fn s2_mode_respects_max_clusters() {
        // S2 calc mode over a rect containing the clumps; cap must bind here too
        // (the s2 clusterer itself has no cap concept).
        let pts = clumped_points();
        let ring = vec![
            geojson::Position::from([-74.1, 39.9]),
            geojson::Position::from([-73.9, 39.9]),
            geojson::Position::from([-73.9, 40.2]),
            geojson::Position::from([-74.1, 40.2]),
            geojson::Position::from([-74.1, 39.9]),
        ];
        let feature = geojson::Feature {
            bbox: None,
            geometry: Some(geojson::Geometry::new(geojson::GeometryValue::Polygon {
                coordinates: vec![ring],
            })),
            id: None,
            properties: None,
            foreign_members: None,
        };
        let collection = FeatureCollection {
            bbox: None,
            features: vec![feature],
            foreign_members: None,
        };
        let mut cfg = make_cfg(ClusterMode::Balanced);
        cfg.calculation_mode = CalculationMode::S2;
        // Derived S2Config::default() is zeroed (size 0 div-by-zero in the grid
        // walk) — use the real serve-path defaults' shape: one level-15 cell per
        // candidate so each ~1.1 km-spaced clump gets its own cell.
        cfg.s2 = S2Config { level: 15, size: 1 };
        cfg.min_points = 3;
        cfg.max_clusters = 3;
        let mut stats = Stats::new("t".into(), 3);
        let result = main(&pts, &cfg, collection, &mut stats);
        assert!(
            result.len() <= 3,
            "S2 mode: max_clusters=3 must cap the solution, got {}",
            result.len()
        );
        assert!(
            !result.is_empty(),
            "S2 mode: the cap should bind, not zero the output"
        );
    }

    // ── main: mygod_score populated ───────────────────────────────────────────

    #[test]
    fn main_populates_mygod_score() {
        let pts = vec![[40.0, -74.0], [40.0001, -74.0], [40.1, -74.0]];
        let mut stats = Stats::new("t".into(), 1);
        main(
            &pts,
            &make_cfg(ClusterMode::Fastest),
            empty_collection(),
            &mut stats,
        );
        // mygod_score is set during main(); should be non-negative.
        assert!(stats.mygod_score < usize::MAX);
    }
}
