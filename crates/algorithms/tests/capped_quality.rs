//! Capped-quality ladder invariant: with a BINDING `max_clusters`, the quality
//! mode (Better → crucible) must not cover fewer points than the cheaper
//! Balanced (greedy). Regression for a real report (2026-07-11): on a
//! Manhattan-sized workload with cap 300, Balanced covered 11,819 points while
//! Better covered 11,785 — the quality ladder inverted, because crucible's
//! whole pipeline optimizes full-coverage-minimal-count and the cap was a
//! post-hoc cut over a pool with no slack in it.

use algorithms::clustering::{self, CalculationMode, ClusterMode, ClusteringConfig, S2Config};
use algorithms::stats::Stats;
use koji_core::{Precision, SingleVec};

/// Deterministic LCG so the fixture never changes between runs.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 11
    }
    fn unit(&mut self) -> Precision {
        (self.next() % 1_000_000) as Precision / 1_000_000.0
    }
}

/// Heavy-tailed clumped city, ~5,200 points: 400 clumps over a ~5 km × 2 km
/// strip; a few huge clumps (150 pts), some medium (40), most small (6) —
/// mimics stacked-spawnpoint density so a binding cap has real choices.
fn heavy_tailed_city() -> SingleVec {
    let mut rng = Lcg(0xC0FFEE);
    let mut pts: SingleVec = Vec::new();
    // ~5 km of lon at this latitude, ~2 km of lat.
    let (lat0, lon0) = (40.70, -74.01);
    let (lat_span, lon_span) = (0.018, 0.06);
    for i in 0..400 {
        let clat = lat0 + rng.unit() * lat_span;
        let clon = lon0 + rng.unit() * lon_span;
        let size = if i % 40 == 0 {
            150
        } else if i % 8 == 0 {
            40
        } else {
            6
        };
        for _ in 0..size {
            // Points within ~±40 m of the clump center (one 70 m disk covers most).
            let dlat = (rng.unit() - 0.5) * 0.00072;
            let dlon = (rng.unit() - 0.5) * 0.00095;
            pts.push([clat + dlat, clon + dlon]);
        }
    }
    pts
}

fn covered_with(mode: ClusterMode, pts: &SingleVec, cap: usize) -> (usize, usize) {
    let cfg = ClusteringConfig {
        mode: mode.clone(),
        radius: 70.0,
        min_points: 3,
        max_clusters: cap,
        calculation_mode: CalculationMode::Radius,
        s2: S2Config::default(),
        center_clusters: false,
        plugin_args: String::new(),
    };
    let mut stats = Stats::new("capped-quality".into(), 3);
    let clusters = clustering::main(
        pts,
        &cfg,
        geojson::FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        },
        &mut stats,
    );
    assert!(
        clusters.len() <= cap,
        "{mode:?}: cap {cap} violated ({} clusters)",
        clusters.len()
    );
    (stats.points_covered, clusters.len())
}

#[test]
fn better_beats_balanced_under_binding_cap() {
    let pts = heavy_tailed_city();
    // Uncapped this workload wants ~245 clusters; cap 150 forces real
    // selection pressure. Guard the fixture: the cap must actually bind.
    let cap = 150;
    let (_, uncapped_n) = covered_with(ClusterMode::Balanced, &pts, usize::MAX);
    assert!(
        uncapped_n > cap,
        "fixture rot: uncapped Balanced needs {uncapped_n} ≤ cap {cap} — the cap no longer binds"
    );
    let (balanced, bn) = covered_with(ClusterMode::Balanced, &pts, cap);
    let (better, tn) = covered_with(ClusterMode::Better, &pts, cap);
    println!(
        "balanced: {balanced} covered / {bn} clusters; better: {better} covered / {tn} clusters"
    );
    assert!(
        better >= balanced,
        "quality ladder inverted under cap {cap}: Better covered {better} < Balanced {balanced}"
    );
}
