//! tsp-mt adapter: route cluster centers with the tsp-geo ILS solver.
//! `solve` = standalone (`SortBy::Tsp`, the Route default); `refine` =
//! improve an already-sorted (S2-seeded) order (`SortBy::TspHybrid`).
//!
//! No wire-exposed knobs: fixed defaults except `max_neighbors` 10 → 20 so
//! sparse/gappy areas (islands, rivers) keep cross-gap candidate edges.
//! Note the wall-clock budget (auto 2–120 s from n) makes results
//! machine-dependent across runs even with the fixed seed.

use koji_core::SingleVec;
use tsp_geo::{GeoPoint, SolverConfig};

use super::sorting::sort_s2;

fn config() -> SolverConfig {
    let mut cfg = SolverConfig::default();
    cfg.max_neighbors = 20;
    cfg
}

fn geo_points(clusters: &SingleVec) -> Vec<GeoPoint> {
    clusters
        .iter()
        .map(|c| GeoPoint::from_lat_lng(c[0], c[1]))
        .collect()
}

fn reorder(clusters: SingleVec, order: Vec<u32>) -> SingleVec {
    order.into_iter().map(|i| clusters[i as usize]).collect()
}

/// Standalone tsp-mt: greedy construction + ILS over the raw clusters.
/// Solver rejection (out-of-range or non-finite coords) logs and falls back to the S2 sort.
pub(super) fn solve(clusters: SingleVec) -> SingleVec {
    if clusters.len() < 2 {
        return clusters;
    }
    match tsp_geo::solve_order(&geo_points(&clusters), &config()) {
        Ok(order) => reorder(clusters, order),
        Err(e) => {
            log::error!("tsp solve failed: {e}; falling back to s2 sort");
            sort_s2(clusters)
        }
    }
}

/// Hybrid: refine an already-S2-sorted order (identity seed tour).
/// Solver rejection logs and keeps the seed order.
pub(super) fn refine(clusters: SingleVec) -> SingleVec {
    if clusters.len() < 2 {
        return clusters;
    }
    let initial: Vec<u32> = (0..clusters.len() as u32).collect();
    // `initial` is a freshly-built identity tour of the correct length, so
    // `refine_order`'s permutation-rejection Err arm is dead by construction here;
    // the only reachable Err is coord validation, which the fallback below handles.
    match tsp_geo::refine_order(&geo_points(&clusters), &initial, &config()) {
        Ok(order) => reorder(clusters, order),
        Err(e) => {
            log::error!("tsp refine failed: {e}; keeping s2 seed order");
            clusters
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::Precision;

    fn sorted_xy(order: &SingleVec) -> Vec<(i64, i64)> {
        let mut s: Vec<(i64, i64)> = order
            .iter()
            .map(|p| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64))
            .collect();
        s.sort();
        s
    }

    /// Cyclic haversine-ish tour length in degrees-space chord (adequate for
    /// relative comparisons in tests).
    fn tour_len(order: &SingleVec) -> Precision {
        let n = order.len();
        if n < 2 {
            return 0.0;
        }
        (0..n)
            .map(|i| {
                let (a, b) = (order[i], order[(i + 1) % n]);
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
            })
            .sum()
    }

    fn bowtie() -> SingleVec {
        vec![[0.0, 0.0], [0.01, 0.01], [0.0, 0.01], [0.01, 0.0]]
    }

    #[test]
    fn solve_is_permutation_and_uncrosses() {
        let crossed = bowtie();
        let out = solve(crossed.clone());
        assert_eq!(sorted_xy(&out), sorted_xy(&crossed), "same multiset");
        assert!(tour_len(&out) < tour_len(&crossed) - 1e-9, "must shorten");
    }

    #[test]
    fn refine_is_permutation_and_never_worsens() {
        let crossed = bowtie();
        let out = refine(crossed.clone());
        assert_eq!(sorted_xy(&out), sorted_xy(&crossed), "same multiset");
        assert!(tour_len(&out) <= tour_len(&crossed) + 1e-12);
    }

    #[test]
    fn tiny_inputs_pass_through() {
        for clusters in [SingleVec::new(), vec![[1.0, 2.0]]] {
            assert_eq!(solve(clusters.clone()), clusters);
            assert_eq!(refine(clusters.clone()), clusters);
        }
    }
}
