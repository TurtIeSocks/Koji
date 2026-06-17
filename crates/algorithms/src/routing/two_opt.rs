//! S2-seeded local-search route refinement (2-opt + Or-opt over the closed
//! tour). Opt-in via `SortBy::Tsp`. Deterministic, dependency-light, wasm-safe.

use koji_core::{Precision, SingleVec};
use rstar::{primitives::GeomWithData, RTree};
use s2::{latlng::LatLng, point::Point};

const K: usize = 8; // candidate neighbors per node
const MAX_PASSES: usize = 60;
const EPS: Precision = 1e-9; // unit-sphere chord units

type IdxPt = GeomWithData<[Precision; 2], usize>;

/// Refine a closed-tour visiting order (each point `[lat, lon]`) with 2-opt +
/// Or-opt local search. Returns a permutation of the input that lowers the
/// cyclic tour length. Deterministic.
pub fn optimize(order: SingleVec) -> SingleVec {
    let n = order.len();
    if n < 4 {
        return order; // a 0–3 node cycle has nothing to improve
    }

    // 3D unit vectors → cheap chord distance (no per-edge trig).
    let verts: Vec<(f64, f64, f64)> = order
        .iter()
        .map(|p| {
            let v = Point::from(LatLng::from_degrees(p[0], p[1])).0;
            (v.x, v.y, v.z)
        })
        .collect();
    let dist = |i: usize, j: usize| -> Precision {
        let a = verts[i];
        let b = verts[j];
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2) + (a.2 - b.2).powi(2)).sqrt()
    };

    // k-nearest-neighbor candidate lists (rstar on raw lat/lon; Euclidean is
    // fine for *selecting* near neighbors). Sorted by (chord, index) for
    // determinism.
    let tree: RTree<IdxPt> = RTree::bulk_load(
        order
            .iter()
            .enumerate()
            .map(|(i, p)| GeomWithData::new([p[0], p[1]], i))
            .collect(),
    );
    let candidates: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let q = [order[i][0], order[i][1]];
            let mut c: Vec<usize> = tree
                .nearest_neighbor_iter(&q)
                .filter(|g| g.data != i)
                .take(K)
                .map(|g| g.data)
                .collect();
            c.sort_by(|&x, &y| {
                dist(i, x)
                    .partial_cmp(&dist(i, y))
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(x.cmp(&y))
            });
            c
        })
        .collect();

    // Tour as a permutation of node ids (0..n); pos is its inverse.
    let mut tour: Vec<usize> = (0..n).collect();
    let mut pos: Vec<usize> = (0..n).collect();

    let mut pass = 0;
    loop {
        let mut improved = false;

        // ── 2-opt: for edge (a,b=succ a) and candidate c (d=succ c), try
        //    replacing (a,b)+(c,d) with (a,c)+(b,d) by reversing b..c.
        for a in 0..n {
            let pa = pos[a];
            let b = tour[(pa + 1) % n];
            for &c in &candidates[a] {
                if c == b {
                    continue;
                }
                let pc = pos[c];
                let d = tour[(pc + 1) % n];
                if d == a {
                    continue; // edges adjacent — degenerate
                }
                let gain = dist(a, b) + dist(c, d) - dist(a, c) - dist(b, d);
                if gain > EPS {
                    let (lo, hi) = if pa < pc { (pa, pc) } else { (pc, pa) };
                    tour[lo + 1..=hi].reverse();
                    for p in lo + 1..=hi {
                        pos[tour[p]] = p;
                    }
                    improved = true;
                    break; // first-improvement; advance to next a
                }
            }
        }

        // ── Or-opt: relocate a run of L∈{1,2,3} nodes next to a candidate.
        for l in 1..=3usize {
            if n < l + 2 {
                continue;
            }
            for s in 0..n {
                let p0 = pos[s];
                let seg: Vec<usize> = (0..l).map(|t| tour[(p0 + t) % n]).collect();
                let first = seg[0];
                let last = seg[l - 1];
                let prev = tour[(p0 + n - 1) % n];
                let nxt = tour[(p0 + l) % n];
                if seg.contains(&prev) || seg.contains(&nxt) {
                    continue; // wraps onto itself (tiny n)
                }
                // Saving from removing the segment and closing the gap.
                let removed = dist(prev, first) + dist(last, nxt) - dist(prev, nxt);
                if removed <= EPS {
                    continue; // insertion cost is ≥0, so no net gain possible
                }
                let mut applied = false;
                for &c in candidates[first].iter().chain(candidates[last].iter()) {
                    if seg.contains(&c) || c == prev {
                        continue;
                    }
                    let e = tour[(pos[c] + 1) % n];
                    if seg.contains(&e) {
                        continue;
                    }
                    let base = dist(c, e);
                    let add_f = dist(c, first) + dist(last, e) - base;
                    let add_r = dist(c, last) + dist(first, e) - base;
                    let (add, rev) = if add_r < add_f {
                        (add_r, true)
                    } else {
                        (add_f, false)
                    };
                    if removed - add > EPS {
                        let seg_set: std::collections::HashSet<usize> =
                            seg.iter().copied().collect();
                        let mut rest: Vec<usize> =
                            tour.iter().copied().filter(|x| !seg_set.contains(x)).collect();
                        let cpos = rest.iter().position(|&x| x == c).unwrap();
                        let mut ins = seg.clone();
                        if rev {
                            ins.reverse();
                        }
                        rest.splice(cpos + 1..cpos + 1, ins);
                        tour = rest;
                        for (p, &node) in tour.iter().enumerate() {
                            pos[node] = p;
                        }
                        improved = true;
                        applied = true;
                        break;
                    }
                }
                let _ = applied;
            }
        }

        pass += 1;
        if !improved || pass >= MAX_PASSES {
            break;
        }
    }

    tour.into_iter().map(|i| order[i]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn two_opt_uncrosses_bowtie() {
        // Unit-square corners fed in a self-crossing order (two diagonals).
        // Optimal closed tour is the perimeter; 2-opt must uncross it.
        let crossed: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let out = optimize(crossed.clone());
        let perim = tour_len(&vec![[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]]);
        assert!(tour_len(&out) < tour_len(&crossed) - 1e-9, "must shorten");
        assert!(
            (tour_len(&out) - perim).abs() < 1e-9,
            "must reach the perimeter optimum"
        );
    }

    #[test]
    fn two_opt_idempotent_at_local_optimum() {
        let crossed: SingleVec = vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]];
        let once = optimize(crossed);
        let twice = optimize(once.clone());
        assert!((tour_len(&twice) - tour_len(&once)).abs() < 1e-12);
    }

    #[test]
    fn reaches_hexagon_optimum() {
        // 6 points on a circle; the optimal closed tour is the convex
        // perimeter. Seed scrambled. Exercises 2-opt + Or-opt jointly.
        let mut circle: SingleVec = (0..6)
            .map(|k| {
                let t = std::f64::consts::TAU * (k as f64) / 6.0;
                [t.sin(), t.cos()] // [lat, lon] on a small circle near (0,0)
            })
            .collect();
        let perim = tour_len(&circle);
        // Scramble into a crossing order.
        circle.swap(1, 4);
        circle.swap(2, 5);
        let out = optimize(circle.clone());
        assert!(tour_len(&out) < tour_len(&circle) - 1e-9, "must shorten");
        assert!(
            (tour_len(&out) - perim).abs() < 1e-9,
            "must reach the convex perimeter optimum"
        );
    }
}
