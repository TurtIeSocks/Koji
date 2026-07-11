//! Per-chunk solver: candidate generation + exact lazy greedy.
//!
//! Works entirely in planar "1.0 = radius" units. Every decision uses the
//! effective radius `RHO = 1 - 1e-3` so a committed center is strictly inside
//! the true radius of every point it claims (pair-intersection candidates sit
//! at distance exactly RHO from their generating pair, never at the knife edge
//! of the real Haversine radius).

use super::frame::Grid;
use super::geometry::circle_intersections;
use koji_core::Precision;

/// Effective coverage radius in planar units. The 1e-3 backoff (7 cm at 70 m)
/// absorbs the azimuthal-equidistant projection error (≤ ~5e-5 at 50 km
/// chunks) and guarantees the exact-Haversine scorer agrees with every
/// coverage decision made here.
pub const RHO: Precision = 1.0 - 1e-3;

pub struct SolveParams<'a> {
    /// Cluster cost in points (`min_points`, clamped ≥ 1).
    pub m: usize,
    /// Per-point cap on pair-intersection partners (K nearest within 2·RHO).
    pub k_cap: usize,
    /// Deterministic restart variant: permutes candidate order / tie-breaks
    /// so small inputs can take the best of several greedy runs.
    /// 0 = canonical; 1 = reversed candidates; 2 = vertices before points;
    /// 3 = inverted density tie-break; 4 = extra RHO/2 lattice candidates
    /// (helps dense clumps at m ≥ 2; only ever chosen when it scores better
    /// post-refinement).
    pub variant: u8,
    /// Points already covered by fixed external disks (LNS windows): they
    /// contribute candidates but never gain. Empty slice = none.
    pub pre_covered: &'a [bool],
}

/// Greedily cover `pts` (planar, radius units). Returns committed centers in
/// planar coordinates. Deterministic: candidate order is a pure function of
/// the input order.
pub fn solve_chunk(pts: &[[Precision; 2]], params: &SolveParams) -> Vec<[Precision; 2]> {
    if pts.is_empty() {
        return vec![];
    }
    let m = params.m.max(1);
    let grid = Grid::build(pts, 1.0);

    // --- Candidate generation -------------------------------------------
    // C1: every input point. C2: both intersections of RHO-circles around
    // each pair within 2·RHO (capped to k_cap nearest partners per point).
    let mut candidates: Vec<[Precision; 2]> = Vec::with_capacity(pts.len() * (1 + params.k_cap));
    candidates.extend_from_slice(pts);

    let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(pts.len() * params.k_cap);
    let mut neigh: Vec<(Precision, u32)> = Vec::new();
    for (i, &p) in pts.iter().enumerate() {
        neigh.clear();
        grid.for_each_within(pts, p, 2.0 * RHO, |j, d2| {
            if j as usize != i {
                neigh.push((d2, j));
            }
        });
        // K nearest partners; ties broken by index for determinism.
        // total_cmp: NaN distances (degenerate input coords) must not panic — same
        // hardening refine.rs comparators already carry.
        neigh.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for &(_, j) in neigh.iter().take(params.k_cap) {
            let (a, b) = if (i as u32) < j {
                (i as u32, j)
            } else {
                (j, i as u32)
            };
            pairs.push((a, b));
        }
    }
    pairs.sort_unstable();
    pairs.dedup();
    for &(i, j) in &pairs {
        if let Some(inters) = circle_intersections(pts[i as usize], pts[j as usize], RHO) {
            candidates.push(inters[0]);
            candidates.push(inters[1]);
        }
    }

    match params.variant {
        1 => candidates.reverse(),
        2 => {
            let n1 = pts.len().min(candidates.len());
            candidates.rotate_left(n1);
        }
        4 => {
            // Dense interior lattice on top of the point/vertex set. As an
            // always-on addition this measured neutral-to-harmful, but as a
            // restart variant it is only kept when the post-refine score
            // says so — and it wins on dense clumps at m ≥ 2 where legacy's
            // fine grid placed better-centered equal-gain disks.
            let (mut min_x, mut min_y) = (Precision::INFINITY, Precision::INFINITY);
            let (mut max_x, mut max_y) = (Precision::NEG_INFINITY, Precision::NEG_INFINITY);
            for p in pts {
                min_x = min_x.min(p[0]);
                max_x = max_x.max(p[0]);
                min_y = min_y.min(p[1]);
                max_y = max_y.max(p[1]);
            }
            let step = RHO / 2.0;
            let nx = ((max_x - min_x) / step).ceil() as i64 + 1;
            let ny = ((max_y - min_y) / step).ceil() as i64 + 1;
            if nx.saturating_mul(ny) <= 16 * pts.len().max(1) as i64 {
                for ix in 0..nx {
                    for iy in 0..ny {
                        candidates.push([
                            min_x + ix as Precision * step,
                            min_y + iy as Precision * step,
                        ]);
                    }
                }
            }
        }
        _ => {}
    }

    // --- Initial gains (static coverage) --------------------------------
    let mut covered = if params.pre_covered.is_empty() {
        vec![false; pts.len()]
    } else {
        debug_assert_eq!(params.pre_covered.len(), pts.len());
        params.pre_covered.to_vec()
    };
    let count_uncovered = |grid: &Grid, covered: &[bool], c: [Precision; 2]| -> usize {
        let mut g = 0usize;
        grid.for_each_within(pts, c, RHO, |idx, _| {
            if !covered[idx as usize] {
                g += 1;
            }
        });
        g
    };

    let mut static_cov: Vec<usize> = Vec::with_capacity(candidates.len());
    let mut max_gain = 0usize;
    for &c in &candidates {
        let g = count_uncovered(&grid, &covered, c);
        static_cov.push(g);
        max_gain = max_gain.max(g);
    }
    if max_gain < m {
        return vec![];
    }

    // --- Lazy greedy on an integer bucket queue --------------------------
    // Priority is lexicographic (current gain, static coverage): equal-gain
    // ties break toward denser placements, because boundary-extreme vertex
    // candidates strand nearby points that a centrally-placed candidate of
    // equal gain would set up for future clusters. Encoded as one integer
    // bucket index gain·TIE + min(static, TIE−1); gains only decrease as
    // points get covered (submodularity) and static is constant, so popping
    // from the top bucket and recomputing is exact lexicographic greedy.
    const TIE: usize = 256;
    let invert_tie = params.variant == 3;
    let prio = move |g: usize, s: usize| {
        let s = s.min(TIE - 1);
        g * TIE + if invert_tie { TIE - 1 - s } else { s }
    };
    let floor = m * TIE;

    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); (max_gain + 1) * TIE];
    for (ci, &s) in static_cov.iter().enumerate() {
        if s >= m {
            buckets[prio(s, s)].push(ci as u32);
        }
    }

    let mut centers: Vec<[Precision; 2]> = Vec::new();
    let mut cur = buckets.len() - 1;
    loop {
        while cur >= floor && buckets[cur].is_empty() {
            if cur == floor {
                return centers;
            }
            cur -= 1;
        }
        if cur < floor || buckets[cur].is_empty() {
            return centers;
        }
        let ci = buckets[cur].pop().unwrap() as usize;
        let g = count_uncovered(&grid, &covered, candidates[ci]);
        let p = prio(g, static_cov[ci]);
        if p >= cur {
            // Still the best possible: commit.
            // (Recentering the committed disk to its claim-set SEC was tried
            // here and measured strictly worse on every dataset — boundary
            // placements leave a better remainder than centered ones, and
            // the refine passes already re-center where it pays.)
            grid.for_each_within(pts, candidates[ci], RHO, |idx, _| {
                covered[idx as usize] = true;
            });
            centers.push(candidates[ci]);
        } else if g >= m {
            buckets[p].push(ci as u32);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(a: [Precision; 2], b: [Precision; 2]) -> Precision {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
    }

    #[test]
    fn two_far_points_one_disk_via_intersection() {
        // Two points ~1.9 apart: only a pair-intersection candidate covers both.
        let pts = vec![[0.0, 0.0], [1.9, 0.0]];
        let centers = solve_chunk(
            &pts,
            &SolveParams {
                m: 1,
                k_cap: 8,
                variant: 0,
                pre_covered: &[],
            },
        );
        assert_eq!(centers.len(), 1, "one disk should cover both points");
        for p in &pts {
            assert!(d(centers[0], *p) <= RHO + 1e-9);
        }
    }

    #[test]
    fn min_points_filters_sparse_singletons() {
        // Three isolated points, m=2: no candidate reaches gain 2 → no centers.
        let pts = vec![[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]];
        let centers = solve_chunk(
            &pts,
            &SolveParams {
                m: 2,
                k_cap: 8,
                variant: 0,
                pre_covered: &[],
            },
        );
        assert!(centers.is_empty());
    }

    #[test]
    fn m1_covers_everything() {
        let mut pts = vec![];
        for i in 0..30 {
            for j in 0..30 {
                pts.push([i as Precision * 0.83, j as Precision * 0.83]);
            }
        }
        let centers = solve_chunk(
            &pts,
            &SolveParams {
                m: 1,
                k_cap: 8,
                variant: 0,
                pre_covered: &[],
            },
        );
        assert!(!centers.is_empty());
        for p in &pts {
            assert!(
                centers.iter().any(|c| d(*c, *p) <= RHO + 1e-9),
                "all points covered at m=1"
            );
        }
        // Sanity: should be far fewer centers than points.
        assert!(centers.len() < pts.len() / 2);
    }

    #[test]
    fn deterministic() {
        let mut pts = vec![];
        let mut x: u64 = 99;
        for _ in 0..500 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = ((x >> 11) as Precision / (1u64 << 53) as Precision) * 20.0;
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let b = ((x >> 11) as Precision / (1u64 << 53) as Precision) * 20.0;
            pts.push([a, b]);
        }
        let a = solve_chunk(
            &pts,
            &SolveParams {
                m: 1,
                k_cap: 8,
                variant: 0,
                pre_covered: &[],
            },
        );
        let b = solve_chunk(
            &pts,
            &SolveParams {
                m: 1,
                k_cap: 8,
                variant: 0,
                pre_covered: &[],
            },
        );
        assert_eq!(a, b);
    }
}
