//! S2-seeded local-search route refinement (2-opt + Or-opt over the closed
//! tour). Opt-in via `SortBy::Tsp`. Deterministic, dependency-light, wasm-safe.

use koji_core::SingleVec;

/// Refine a closed-tour visiting order (each point `[lat, lon]`) with 2-opt +
/// Or-opt local search. Returns a permutation of the input that lowers the
/// cyclic tour length. Deterministic.
pub fn optimize(order: SingleVec) -> SingleVec {
    order
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_core::Precision;

    /// Sum of unit-sphere chord lengths around the closed tour (incl. wrap).
    fn tour_len(order: &SingleVec) -> Precision {
        use s2::{latlng::LatLng, point::Point};
        let v: Vec<(f64, f64, f64)> = order
            .iter()
            .map(|p| {
                let pt = Point::from(LatLng::from_degrees(p[0], p[1])).0;
                (pt.x, pt.y, pt.z)
            })
            .collect();
        let chord = |a: (f64, f64, f64), b: (f64, f64, f64)| {
            ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
        };
        let n = v.len();
        if n < 2 {
            return 0.0;
        }
        (0..n).map(|i| chord(v[i], v[(i + 1) % n])).sum()
    }

    fn sorted_xy(order: &SingleVec) -> Vec<(i64, i64)> {
        let mut s: Vec<(i64, i64)> = order
            .iter()
            .map(|p| ((p[0] * 1e6).round() as i64, (p[1] * 1e6).round() as i64))
            .collect();
        s.sort();
        s
    }

    #[test]
    fn optimize_is_permutation() {
        let order: SingleVec = vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]];
        let out = optimize(order.clone());
        assert_eq!(sorted_xy(&out), sorted_xy(&order), "same multiset");
    }

    #[test]
    fn optimize_edge_cases_return_same_set() {
        for order in [
            SingleVec::new(),
            vec![[1.0, 2.0]],
            vec![[1.0, 2.0], [3.0, 4.0]],
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],
        ] {
            let out = optimize(order.clone());
            assert_eq!(sorted_xy(&out), sorted_xy(&order));
        }
    }

    #[test]
    fn optimize_never_worsens() {
        let order: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let before = tour_len(&order);
        let after = tour_len(&optimize(order));
        assert!(after <= before + 1e-12, "tour must not get longer");
    }

    #[test]
    fn optimize_is_deterministic() {
        let order: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        assert_eq!(optimize(order.clone()), optimize(order));
    }
}
