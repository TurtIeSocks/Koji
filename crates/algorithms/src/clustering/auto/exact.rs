//! Exact solver for small LNS windows.
//!
//! Minimizes `m·disks + abandoned` over the window's lost points (those not
//! covered by any disk outside the window). Discretization: an optimal disk
//! position for covering a subset S of the lost points can always be moved to
//! a member of S or a pair-intersection vertex of S, so the candidate set
//! below is exact for the objective. Branch-and-bound with an admissible
//! "k disks cover ≤ k·maxcov points" bound; bails out (None) past the node
//! budget so the caller can fall back to the greedy re-solve.

use super::geometry::circle_intersections;
use super::solve::RHO;

/// Hard caps: windows beyond these fall back to greedy.
pub const MAX_EXACT_POINTS: usize = 64;
const MAX_NODES: usize = 200_000;

struct Bb<'a> {
    m: usize,
    masks: &'a [u64],
    maxcov: u32,
    nodes: usize,
    best_cost: usize,
    best_choice: Vec<u32>,
    stack: Vec<u32>,
}

impl Bb<'_> {
    /// Admissible lower bound for covering/abandoning `rem`.
    fn lb(&self, rem: u64) -> usize {
        let p = rem.count_ones() as usize;
        if p == 0 {
            return 0;
        }
        let maxcov = self.maxcov as usize;
        let mut best = p; // abandon everything
        let mut k = 1;
        while (k - 1) * maxcov < p {
            let covered = (k * maxcov).min(p);
            best = best.min(k * self.m + (p - covered));
            k += 1;
        }
        best
    }

    fn search(&mut self, rem: u64, cost: usize) {
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
        let u = rem.trailing_zeros();
        // Branch 1: place a disk covering u (largest residual coverage first).
        let mut covering: Vec<(u32, u32)> = self
            .masks
            .iter()
            .enumerate()
            .filter(|(_, mask)| **mask & (1 << u) != 0)
            .map(|(ci, mask)| ((mask & rem).count_ones(), ci as u32))
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
    lost: &[[f64; 2]],
    m: usize,
    incumbent_cost: usize,
) -> Option<(usize, Vec<[f64; 2]>)> {
    let n = lost.len();
    if n == 0 || n > MAX_EXACT_POINTS {
        return None;
    }

    // Candidates: the lost points + pair vertices of pairs within 2·RHO.
    let mut cands: Vec<[f64; 2]> = lost.to_vec();
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
    let mut seen: hashbrown::HashMap<u64, u32> = hashbrown::HashMap::new();
    let mut masks: Vec<u64> = Vec::new();
    let mut positions: Vec<[f64; 2]> = Vec::new();
    for c in cands {
        let mut mask = 0u64;
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
    let keep: Vec<bool> = masks
        .iter()
        .map(|&a| !masks.iter().any(|&b| b != a && (a & b) == a))
        .collect();
    let mut kept_masks: Vec<u64> = Vec::new();
    let mut kept_pos: Vec<[f64; 2]> = Vec::new();
    for (i, &k) in keep.iter().enumerate() {
        if k {
            kept_masks.push(masks[i]);
            kept_pos.push(positions[i]);
        }
    }
    if kept_masks.is_empty() {
        return None;
    }

    let maxcov = kept_masks.iter().map(|m| m.count_ones()).max().unwrap_or(1);
    let full: u64 = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
    let mut bb = Bb {
        m: m.max(1),
        masks: &kept_masks,
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
}
