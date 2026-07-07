//! Exact solver for small LNS windows.
//!
//! Minimizes `m·disks + abandoned` over the window's lost points (those not
//! covered by any disk outside the window). Discretization: an optimal disk
//! position for covering a subset S of the lost points can always be moved to
//! a member of S or a pair-intersection vertex of S, so the candidate set
//! below is exact for the objective. Branch-and-bound with three admissible
//! bounds (max wins per node):
//!   (a) global: k disks cover ≤ k·maxcov points, remainder abandoned;
//!   (b) dual-fitting: every covered point pays ≥ m/cov_i of its disk's cost
//!       (cov_i = the largest candidate disk containing i), every abandoned
//!       point pays 1, so Σ min(1, m/cov_i) over remaining points is a floor;
//!   (c) conflict: points pairwise farther than 2·RHO can never share a disk,
//!       so each member of an independent set costs ≥ 1 (own disk or
//!       abandonment).
//! Bails out (None) past the node budget so the caller can fall back to the
//! greedy re-solve.

use super::geometry::circle_intersections;
use super::solve::RHO;
use koji_core::Precision;

type Mask = u128;

/// Hard caps: windows beyond these fall back to greedy.
pub const MAX_EXACT_POINTS: usize = 128;
const MAX_NODES: usize = 200_000;
/// Skip the O(M²) dominance prune past this many masks; pruning is an
/// optimization, never a correctness requirement.
const DOMINANCE_LIMIT: usize = 4096;
/// Fixed-point scale for the dual-fitting shares (rounded down per point, so
/// the summed bound stays admissible).
const SHARE_SCALE: u64 = 1 << 20;

struct Bb<'a> {
    m: usize,
    masks: &'a [Mask],
    /// For each point, the candidate masks covering it (branch enumeration
    /// only ever needs the masks containing the pivot point).
    by_point: &'a [Vec<u32>],
    /// Scaled dual-fitting share per point: min(SCALE, m·SCALE/cov_i).
    share: &'a [u64],
    /// Greedy independent set in the >2·RHO conflict graph.
    is_mask: Mask,
    maxcov: u32,
    nodes: usize,
    best_cost: usize,
    best_choice: Vec<u32>,
    stack: Vec<u32>,
}

impl Bb<'_> {
    /// Admissible lower bound for covering/abandoning `rem`.
    fn lb(&self, rem: Mask) -> usize {
        let p = rem.count_ones() as usize;
        if p == 0 {
            return 0;
        }
        // (a) global coverage bound.
        let maxcov = self.maxcov as usize;
        let mut best = p; // abandon everything (k = 0)
        let mut k = 1;
        while (k - 1) * maxcov < p {
            let covered = (k * maxcov).min(p);
            best = best.min(k * self.m + (p - covered));
            k += 1;
        }
        // (b) per-point dual-fitting.
        let mut sum = 0u64;
        let mut r = rem;
        while r != 0 {
            sum += self.share[r.trailing_zeros() as usize];
            r &= r - 1;
        }
        let dual = sum.div_ceil(SHARE_SCALE) as usize;
        // (c) pairwise-conflict independent set.
        let is_cnt = (rem & self.is_mask).count_ones() as usize;
        best.max(dual).max(is_cnt)
    }

    fn search(&mut self, rem: Mask, cost: usize) {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return;
        }
        if rem == 0 {
            if cost < self.best_cost {
                self.best_cost = cost;
                self.best_choice = self.stack.clone();
            }
            return;
        }
        if cost + self.lb(rem) >= self.best_cost {
            return;
        }
        let u = rem.trailing_zeros() as usize;
        // Branch 1: place a disk covering u (largest residual coverage first).
        let mut covering: Vec<(u32, u32)> = self.by_point[u]
            .iter()
            .map(|&ci| ((self.masks[ci as usize] & rem).count_ones(), ci))
            .collect();
        covering.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        for (_, ci) in covering {
            self.stack.push(ci);
            self.search(rem & !self.masks[ci as usize], cost + self.m);
            self.stack.pop();
            if self.nodes > MAX_NODES {
                return;
            }
        }
        // Branch 2: abandon u.
        self.search(rem & !(1 << u), cost + 1);
    }
}

/// Exactly solve a window: `lost` are the planar positions (radius units) of
/// points that need re-covering; cost is `m` per disk + 1 per abandoned
/// point. Returns the optimal disk centers when the search completes within
/// budget AND beats `incumbent_cost`; `None` otherwise.
pub fn solve_window_exact(
    lost: &[[Precision; 2]],
    m: usize,
    incumbent_cost: usize,
) -> Option<(usize, Vec<[Precision; 2]>)> {
    let n = lost.len();
    if n == 0 || n > MAX_EXACT_POINTS {
        return None;
    }

    // Candidates: the lost points + pair vertices of pairs within 2·RHO.
    let mut cands: Vec<[Precision; 2]> = lost.to_vec();
    for a in 0..n {
        for b in (a + 1)..n {
            if let Some(vs) = circle_intersections(lost[a], lost[b], RHO) {
                cands.push(vs[0]);
                cands.push(vs[1]);
            }
        }
    }

    // Coverage masks; dedupe identical sets and drop dominated ones.
    let r2 = RHO * RHO;
    let mut seen: hashbrown::HashMap<Mask, u32> = hashbrown::HashMap::new();
    let mut masks: Vec<Mask> = Vec::new();
    let mut positions: Vec<[Precision; 2]> = Vec::new();
    for c in cands {
        let mut mask: Mask = 0;
        for (i, p) in lost.iter().enumerate() {
            let d2 = (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2);
            if d2 <= r2 {
                mask |= 1 << i;
            }
        }
        if mask == 0 {
            continue;
        }
        if seen.insert(mask, masks.len() as u32).is_none() {
            masks.push(mask);
            positions.push(c);
        }
    }
    // Dominance prune: a mask strictly contained in another never helps.
    // O(M²), so skipped on oversized candidate sets.
    let keep: Vec<bool> = if masks.len() <= DOMINANCE_LIMIT {
        masks
            .iter()
            .map(|&a| !masks.iter().any(|&b| b != a && (a & b) == a))
            .collect()
    } else {
        vec![true; masks.len()]
    };
    let mut kept_masks: Vec<Mask> = Vec::new();
    let mut kept_pos: Vec<[Precision; 2]> = Vec::new();
    for (i, &k) in keep.iter().enumerate() {
        if k {
            kept_masks.push(masks[i]);
            kept_pos.push(positions[i]);
        }
    }
    if kept_masks.is_empty() {
        return None;
    }

    // Per-point structures for the bounds and branch enumeration.
    let mut by_point: Vec<Vec<u32>> = vec![Vec::new(); n];
    let mut cov: Vec<u32> = vec![0; n];
    for (ci, &mask) in kept_masks.iter().enumerate() {
        let pc = mask.count_ones();
        let mut r = mask;
        while r != 0 {
            let i = r.trailing_zeros() as usize;
            by_point[i].push(ci as u32);
            cov[i] = cov[i].max(pc);
            r &= r - 1;
        }
    }
    let m_eff = m.max(1);
    let share: Vec<u64> = cov
        .iter()
        .map(|&c| (m_eff as u64 * SHARE_SCALE / c.max(1) as u64).min(SHARE_SCALE))
        .collect();
    // Greedy independent set in the >2·RHO conflict graph (no disk of radius
    // RHO can cover two points farther apart than 2·RHO).
    let conflict2 = (2.0 * RHO) * (2.0 * RHO);
    let mut is_mask: Mask = 0;
    let mut chosen: Vec<usize> = Vec::new();
    for i in 0..n {
        if chosen.iter().all(|&j| {
            let dx = lost[i][0] - lost[j][0];
            let dy = lost[i][1] - lost[j][1];
            dx * dx + dy * dy > conflict2
        }) {
            chosen.push(i);
            is_mask |= 1 << i;
        }
    }

    let maxcov = kept_masks.iter().map(|m| m.count_ones()).max().unwrap_or(1);
    let full: Mask = if n == MAX_EXACT_POINTS {
        Mask::MAX
    } else {
        (1 << n) - 1
    };
    let mut bb = Bb {
        m: m_eff,
        masks: &kept_masks,
        by_point: &by_point,
        share: &share,
        is_mask,
        maxcov,
        nodes: 0,
        best_cost: incumbent_cost,
        best_choice: Vec::new(),
        stack: Vec::new(),
    };
    bb.search(full, 0);
    if bb.nodes > MAX_NODES || bb.best_cost >= incumbent_cost {
        return None;
    }
    let centers = bb
        .best_choice
        .iter()
        .map(|&ci| kept_pos[ci as usize])
        .collect();
    Some((bb.best_cost, centers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{RngExt, SeedableRng, rngs::SmallRng};

    #[test]
    fn exact_covers_pair_with_one_disk_at_m1() {
        // Two points 1.9 apart at m=1: one disk (cost 1) beats abandoning (2).
        let lost = vec![[0.0, 0.0], [1.9, 0.0]];
        let (cost, centers) = solve_window_exact(&lost, 1, usize::MAX).unwrap();
        assert_eq!(cost, 1);
        assert_eq!(centers.len(), 1);
    }

    #[test]
    fn exact_abandons_pair_at_m3() {
        // Same pair at m=3: abandoning both (cost 2) beats one disk (cost 3)
        // — a 2-point cluster at min_points=3 scores worse than 2 uncovered.
        let lost = vec![[0.0, 0.0], [1.9, 0.0]];
        let (cost, centers) = solve_window_exact(&lost, 3, usize::MAX).unwrap();
        assert_eq!(cost, 2);
        assert!(centers.is_empty());
    }

    #[test]
    fn exact_colinear_m1_two_disks() {
        // Three colinear points spaced 1.8 (ends 3.6 apart > 2): needs two
        // disks at m=1, cost 2.
        let lost = vec![[0.0, 0.0], [1.8, 0.0], [3.6, 0.0]];
        let (cost, centers) = solve_window_exact(&lost, 1, usize::MAX).unwrap();
        assert_eq!(cost, 2);
        assert_eq!(centers.len(), 2);
    }

    #[test]
    fn exact_quad_disk_beats_abandonment_at_m3() {
        // Four mutually-close points at m=3: one disk (cost 3) beats
        // abandoning four (cost 4).
        let lost = vec![[0.0, 0.0], [0.5, 0.0], [0.0, 0.5], [0.5, 0.5]];
        let (cost, centers) = solve_window_exact(&lost, 3, usize::MAX).unwrap();
        assert_eq!(cost, 3);
        assert_eq!(centers.len(), 1);
    }

    #[test]
    fn exact_respects_incumbent() {
        // Optimal here is 2 (abandon both at m=3); an incumbent of 2 means
        // no strict win — must return None.
        let lost = vec![[0.0, 0.0], [1.9, 0.0]];
        assert!(solve_window_exact(&lost, 3, 2).is_none());
    }

    /// k tight 16-point blobs spaced far apart; optimal is one disk per blob.
    fn blobs(k: usize) -> Vec<[Precision; 2]> {
        let mut pts = Vec::new();
        for b in 0..k {
            let cx = b as Precision * 10.0;
            for gy in 0..4 {
                for gx in 0..4 {
                    pts.push([cx + gx as Precision * 0.3, gy as Precision * 0.3]);
                }
            }
        }
        pts
    }

    #[test]
    fn exact_solves_80_points_above_old_cap() {
        // 5 blobs × 16 points = 80 > the old 64-point cap.
        let lost = blobs(5);
        let (cost, centers) = solve_window_exact(&lost, 1, usize::MAX).unwrap();
        assert_eq!(cost, 5);
        assert_eq!(centers.len(), 5);
        let (cost3, centers3) = solve_window_exact(&lost, 3, usize::MAX).unwrap();
        assert_eq!(cost3, 15);
        assert_eq!(centers3.len(), 5);
    }

    #[test]
    fn exact_full_128_point_boundary() {
        // 8 blobs × 16 = exactly MAX_EXACT_POINTS; exercises the full-mask
        // (u128::MAX) path.
        let lost = blobs(8);
        assert_eq!(lost.len(), MAX_EXACT_POINTS);
        let (cost, centers) = solve_window_exact(&lost, 1, usize::MAX).unwrap();
        assert_eq!(cost, 8);
        assert_eq!(centers.len(), 8);
        // One past the cap must decline.
        let mut over = blobs(8);
        over.push([100.0, 0.0]);
        assert!(solve_window_exact(&over, 1, usize::MAX).is_none());
    }

    /// Reference solver: memoized exhaustive search over the same candidate
    /// construction, no pruning beyond the memo — independent of the B&B.
    fn brute_force(lost: &[[Precision; 2]], m: usize) -> usize {
        let n = lost.len();
        let r2 = RHO * RHO;
        let mut cands: Vec<[Precision; 2]> = lost.to_vec();
        for a in 0..n {
            for b in (a + 1)..n {
                if let Some(vs) = circle_intersections(lost[a], lost[b], RHO) {
                    cands.push(vs[0]);
                    cands.push(vs[1]);
                }
            }
        }
        let masks: Vec<u128> = cands
            .iter()
            .map(|c| {
                let mut mask = 0u128;
                for (i, p) in lost.iter().enumerate() {
                    if (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r2 {
                        mask |= 1 << i;
                    }
                }
                mask
            })
            .collect();
        fn go(
            rem: u128,
            m: usize,
            masks: &[u128],
            memo: &mut hashbrown::HashMap<u128, usize>,
        ) -> usize {
            if rem == 0 {
                return 0;
            }
            if let Some(&c) = memo.get(&rem) {
                return c;
            }
            let u = rem.trailing_zeros();
            // abandon u
            let mut best = 1 + go(rem & !(1 << u), m, masks, memo);
            for mask in masks {
                if mask & (1 << u) != 0 {
                    best = best.min(m + go(rem & !mask, m, masks, memo));
                }
            }
            memo.insert(rem, best);
            best
        }
        go(
            if n == 128 {
                u128::MAX
            } else {
                (1u128 << n) - 1
            },
            m,
            &masks,
            &mut hashbrown::HashMap::new(),
        )
    }

    #[test]
    fn exact_matches_brute_force_on_random_instances() {
        let mut rng = SmallRng::seed_from_u64(7);
        for trial in 0..40 {
            let n = 4 + (trial % 7);
            let lost: Vec<[Precision; 2]> = (0..n)
                .map(|_| {
                    [
                        rng.random::<Precision>() * 4.0,
                        rng.random::<Precision>() * 4.0,
                    ]
                })
                .collect();
            for m in [1usize, 3] {
                let expected = brute_force(&lost, m);
                let (cost, centers) = solve_window_exact(&lost, m, usize::MAX)
                    .expect("small instances always fit the node budget");
                assert_eq!(cost, expected, "trial {trial} m={m} lost={lost:?}");
                // Verify the returned centers actually achieve the cost.
                let r2 = RHO * RHO;
                let covered = lost
                    .iter()
                    .filter(|p| {
                        centers
                            .iter()
                            .any(|c| (p[0] - c[0]).powi(2) + (p[1] - c[1]).powi(2) <= r2)
                    })
                    .count();
                assert_eq!(m * centers.len() + (n - covered), cost);
            }
        }
    }
}
