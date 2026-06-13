//! Crucible — the mode-less next evolution of the greedy clusterer. Melts each
//! neighborhood down (LNS ruin-and-recreate, simulated annealing) and reforges
//! it, fusing the best of many restart variants into one cover.
//!
//! Minimizes `mygod_score = min_points · clusters + uncovered` (stats.rs) with
//! every knob derived from the input; `ClusterMode` is no longer consulted.
//! Design doc: docs/superpowers/specs/2026-06-09-clustering-auto-evolution.md
//!
//! Pipeline: dedupe (S2 L20) → 2r-connectivity components → S2 chunking with
//! halo for big components → per-chunk planar solve (point + pair-intersection
//! candidates, exact lazy greedy) → global refinement (drop / relocate / merge
//! / 3→2 swap / gap-fill on exact-Haversine coverage) → max_clusters cap.

mod components;
mod exact;
mod frame;
mod geometry;
mod refine;
mod solve;

use rayon::prelude::*;
use s2::{cellid::CellID, latlng::LatLng};

use hashbrown::HashMap;
use koji_core::{PointArray, Precision, SingleVec};

use crate::rtree;

use super::partition::{contains_latlng, gather_halo};
use frame::Frame;
use refine::Refiner;
use solve::{SolveParams, solve_chunk};

/// Max owned points per solve chunk. Bounds per-chunk candidate memory and
/// keeps the planar frame extent small.
const CHUNK_BUDGET: usize = 24_000;
/// Components whose bbox fits within this extent solve as one chunk.
const SINGLE_CHUNK_EXTENT_M: Precision = 50_000.0;
/// S2 level bounds for chunk splitting (level 8 ≈ ≤ 40 km cells).
const SPLIT_START_LEVEL: u64 = 8;
const SPLIT_MAX_LEVEL: u64 = 16;
/// Global refinement rounds (early exit when converged; the expensive 3→2
/// swap pass only runs at stalls, so extra rounds are cheap).
const REFINE_ROUNDS: usize = 8;
/// Inputs up to this many distinct cells run all restart variants (scored
/// post-refinement, so the choice reflects the real outcome).
const RESTART_MAX_CELLS: usize = 20_000;
/// Medium inputs run a two-variant portfolio (plus recombination) instead.
const RESTART_PAIR_MAX_CELLS: usize = 50_000;

pub struct Crucible {
    pub radius: Precision,
    pub min_points: usize,
    pub max_clusters: usize,
}

impl Default for Crucible {
    fn default() -> Self {
        Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
        }
    }
}

struct Job {
    /// Owning cell for boundary filtering; `None` for whole-component jobs.
    cell: Option<CellID>,
    /// Rep indices owned by this job.
    owned: Vec<u32>,
}

impl Crucible {
    pub fn run(&self, points: &SingleVec) -> SingleVec {
        if points.is_empty() {
            return vec![];
        }
        let m = self.min_points.max(1);
        let (reps, _cells) = components::dedupe(points);
        log::info!(
            "crucible: {} points → {} distinct cells",
            points.len(),
            reps.len()
        );

        let comps = components::components(&reps, self.radius);
        log::info!("crucible: {} connectivity components", comps.len());

        // Component id per rep, to keep halos within their own component.
        let mut comp_of = vec![u32::MAX; reps.len()];
        for (ci, comp) in comps.iter().enumerate() {
            for &r in comp {
                comp_of[r as usize] = ci as u32;
            }
        }

        // Global tree over reps for halo gathering (point radius only affects
        // envelope padding; halo over-inclusion is harmless).
        let global_tree = rtree::spawn(self.radius, &reps);
        let id_to_idx: HashMap<u64, u32> = reps
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                (id.0, i as u32)
            })
            .collect();

        // Build jobs: small/compact components solve whole; large ones split
        // into S2 cells with halo.
        let mut jobs: Vec<(u32, Job)> = Vec::new();
        for (ci, comp) in comps.iter().enumerate() {
            if comp.len() <= CHUNK_BUDGET
                && component_extent_m(&reps, comp) <= SINGLE_CHUNK_EXTENT_M
            {
                jobs.push((
                    ci as u32,
                    Job {
                        cell: None,
                        owned: comp.clone(),
                    },
                ));
            } else {
                for job in split_component(&reps, comp) {
                    jobs.push((ci as u32, job));
                }
            }
        }
        log::info!("crucible: {} solve jobs", jobs.len());

        let k_cap = if reps.len() <= 100_000 { 14 } else { 10 };

        // Small inputs are cheap: take the best of several deterministic
        // greedy orderings (the greedy is order-sensitive on packing-style
        // instances, especially for min_points ≥ 2).
        let variants: &[u8] = if reps.len() <= RESTART_MAX_CELLS {
            &[0, 1, 2, 3, 4]
        } else if reps.len() <= RESTART_PAIR_MAX_CELLS {
            // Natural + lattice orderings are the most structurally diverse
            // pair; recombination then merges their best neighborhoods.
            &[0, 4]
        } else {
            &[0]
        };

        let mut best: Option<(usize, SingleVec)> = None;
        // Centers from every variant's refined solution, for recombination.
        let mut pool: Vec<PointArray> = Vec::new();
        for &variant in variants {
            let centers: Vec<PointArray> = jobs
                .par_iter()
                .flat_map(|(ci, job)| {
                    let mut idxs: Vec<u32> = job.owned.clone();
                    if let Some(cell) = job.cell {
                        // Halo: nearby points (same component) that this job
                        // can cover but never owns.
                        let halo = gather_halo(cell, &global_tree, 2.0 * self.radius);
                        for p in halo {
                            if let Some(&r) = id_to_idx.get(&p.cell_id.0)
                                && comp_of[r as usize] == *ci
                                && !contains_latlng(cell, p.center)
                            {
                                idxs.push(r);
                            }
                        }
                        idxs.sort_unstable();
                        idxs.dedup();
                    }
                    let coords: SingleVec = idxs.iter().map(|&r| reps[r as usize]).collect();
                    let (frame, planar) = Frame::project(&coords, self.radius);
                    // (Per-job variant selection by pre-refine score was
                    // tried and mispredicts the post-refine outcome; variant
                    // choice only pays when scored after refinement.)
                    let centers_planar = solve_chunk(
                        &planar,
                        &SolveParams {
                            m,
                            k_cap,
                            variant,
                            pre_covered: &[],
                        },
                    );
                    centers_planar
                        .into_iter()
                        .map(|xy| frame.unproject(xy))
                        .filter(|c| match job.cell {
                            Some(cell) => contains_latlng(cell, *c),
                            None => true,
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            log::info!("crucible[v{variant}]: {} centers pre-refine", centers.len());

            let refined =
                Refiner::new(centers, &reps, self.radius, m).run(self.radius, REFINE_ROUNDS);
            let score = self.internal_score(&refined, &reps, &global_tree, m);
            log::info!(
                "crucible[v{variant}]: {} centers post-refine, internal score {score}",
                refined.len()
            );
            if variants.len() > 1 {
                pool.extend(refined.iter().copied());
            }
            if best.as_ref().is_none_or(|(s, _)| score < *s) {
                best = Some((score, refined));
            }
        }
        let (best_score, mut refined) = best.expect("at least one variant ran");

        // Column recombination: restart variants are diverse but selection is
        // winner-take-all — different orderings win different neighborhoods.
        // Pool every variant's centers, greedily re-select a cover from the
        // pool, refine that, and keep it when it beats the best single
        // variant.
        if !pool.is_empty() {
            let mut keyed: Vec<(u64, PointArray)> = pool
                .iter()
                .map(|p| {
                    let id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                    (id.0, *p)
                })
                .collect();
            keyed.sort_unstable_by_key(|a| a.0);
            keyed.dedup_by_key(|(id, _)| *id);
            let pool_pts: SingleVec = keyed.into_iter().map(|(_, p)| p).collect();
            let selected = refine::select_from_pool(&pool_pts, &reps, self.radius, m);
            if !selected.is_empty() {
                let recombined =
                    Refiner::new(selected, &reps, self.radius, m).run(self.radius, REFINE_ROUNDS);
                let score = self.internal_score(&recombined, &reps, &global_tree, m);
                log::info!(
                    "crucible[recombine]: pool {} → {} centers, internal score {score} (best variant {best_score})",
                    pool_pts.len(),
                    recombined.len()
                );
                if score < best_score {
                    refined = recombined;
                }
            }
        }

        self.finish(refined, &reps, m)
    }

    /// Warm-start: refine a previous solution against the current points
    /// instead of constructing from scratch. Intended for re-clustering
    /// after incremental point churn — the refiner drops centers that no
    /// longer pay, gap-fills new areas and repacks changed neighborhoods at
    /// a fraction of the cold-run cost. An empty seed falls back to `run`.
    pub fn run_seeded(&self, points: &SingleVec, seed: &SingleVec) -> SingleVec {
        if points.is_empty() {
            return vec![];
        }
        if seed.is_empty() {
            return self.run(points);
        }
        let m = self.min_points.max(1);
        let (reps, _cells) = components::dedupe(points);
        // Dedupe seed centers at S2 L20 (duplicate centers in one cell break
        // refine-pass invariants and never help).
        let mut keyed: Vec<(u64, PointArray)> = seed
            .iter()
            .map(|p| {
                let id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                (id.0, *p)
            })
            .collect();
        keyed.sort_unstable_by_key(|a| a.0);
        keyed.dedup_by_key(|(id, _)| *id);
        let seeds: SingleVec = keyed.into_iter().map(|(_, p)| p).collect();
        log::info!(
            "crucible[warm]: {} seed centers vs {} distinct cells",
            seeds.len(),
            reps.len()
        );
        let refined = Refiner::new(seeds, &reps, self.radius, m).run(self.radius, REFINE_ROUNDS);
        self.finish(refined, &reps, m)
    }

    /// Shared tail: m=1 coverage audit + max_clusters cap.
    ///
    /// The audit enforces the m=1 contract — every distinct cell must end up
    /// covered. Score-neutral singletons recover any stragglers; a warn here
    /// means a refine pass left a hole (worth investigating, never worth
    /// losing coverage over).
    fn finish(&self, mut refined: SingleVec, reps: &SingleVec, m: usize) -> SingleVec {
        if m == 1 {
            let r_eff = self.radius * (1.0 - 1e-3);
            let audit_tree = rtree::spawn(r_eff, &refined);
            let missing: Vec<PointArray> = reps
                .par_iter()
                .filter(|p| audit_tree.locate_at_point(p).is_none())
                .copied()
                .collect();
            if !missing.is_empty() {
                log::warn!(
                    "crucible: m=1 audit found {} uncovered cells; adding singletons",
                    missing.len()
                );
                for p in missing.iter().take(8) {
                    use geo::{Distance, Haversine};
                    let nearest = audit_tree
                        .nearest_neighbor(p)
                        .map(|c| {
                            Haversine.distance(
                                geo::Point::new(p[1], p[0]),
                                geo::Point::new(c.center[1], c.center[0]),
                            )
                        })
                        .unwrap_or(f64::NAN);
                    log::warn!(
                        "crucible: audit miss at [{:.7}, {:.7}], nearest center {:.3} m (r={})",
                        p[0],
                        p[1],
                        nearest,
                        self.radius
                    );
                }
                refined.extend(missing);
            }
        }

        self.enforce_max_clusters(refined, reps)
    }

    /// Internal mygod-score proxy over distinct cells (the per-input-point
    /// duplicate penalty is constant across solutions, so this ranks
    /// restart variants identically to the real score).
    fn internal_score(
        &self,
        centers: &SingleVec,
        reps: &SingleVec,
        tree: &rstar::RTree<crate::rtree::point::Point>,
        m: usize,
    ) -> usize {
        let mut covered = vec![false; reps.len()];
        let id_to_idx: HashMap<u64, u32> = reps
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let id = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(20);
                (id.0, i as u32)
            })
            .collect();
        for c in centers {
            for pt in tree.locate_all_at_point(c) {
                if let Some(&r) = id_to_idx.get(&pt.cell_id.0) {
                    covered[r as usize] = true;
                }
            }
        }
        let uncovered = covered.iter().filter(|c| !**c).count();
        centers.len() * m + uncovered
    }

    /// Rare path: keep the most valuable centers when over the cap.
    fn enforce_max_clusters(&self, centers: SingleVec, reps: &SingleVec) -> SingleVec {
        if centers.len() <= self.max_clusters {
            return centers;
        }
        let tree = rtree::spawn(self.radius, reps);
        let mut scored: Vec<(usize, usize)> = centers
            .par_iter()
            .enumerate()
            .map(|(i, c)| (tree.locate_all_at_point(c).count(), i))
            .collect();
        scored.sort_unstable_by(|a, b| b.cmp(a));
        let mut kept: Vec<PointArray> = scored
            .into_iter()
            .take(self.max_clusters)
            .map(|(_, i)| centers[i])
            .collect();
        kept.sort_by(|a, b| a.partial_cmp(b).unwrap());
        log::warn!(
            "crucible: truncated {} → {} clusters to honor max_clusters",
            centers.len(),
            kept.len()
        );
        kept
    }
}

/// Approximate bbox extent of a component in meters (max of lat/lon spans).
fn component_extent_m(reps: &SingleVec, comp: &[u32]) -> Precision {
    let mut min_lat = Precision::INFINITY;
    let mut max_lat = Precision::NEG_INFINITY;
    let mut min_lon = Precision::INFINITY;
    let mut max_lon = Precision::NEG_INFINITY;
    for &r in comp {
        let p = reps[r as usize];
        min_lat = min_lat.min(p[0]);
        max_lat = max_lat.max(p[0]);
        min_lon = min_lon.min(p[1]);
        max_lon = max_lon.max(p[1]);
    }
    let mid_lat = ((min_lat + max_lat) / 2.0).to_radians();
    let lat_m = (max_lat - min_lat) * 111_132.0;
    let lon_m = (max_lon - min_lon) * 111_320.0 * mid_lat.cos().abs();
    lat_m.max(lon_m)
}

/// Split a large component into S2 cell jobs, descending until each cell owns
/// ≤ CHUNK_BUDGET reps (or the max split level is reached).
fn split_component(reps: &SingleVec, comp: &[u32]) -> Vec<Job> {
    let group = |idxs: &[u32], level: u64| -> Vec<(u64, Vec<u32>)> {
        let mut map: HashMap<u64, Vec<u32>> = HashMap::new();
        for &r in idxs {
            let p = reps[r as usize];
            let cell = CellID::from(LatLng::from_degrees(p[0], p[1])).parent(level);
            map.entry(cell.0).or_default().push(r);
        }
        let mut v: Vec<(u64, Vec<u32>)> = map.into_iter().collect();
        v.sort_by_key(|(id, _)| *id);
        v
    };

    let mut jobs: Vec<Job> = Vec::new();
    let mut frontier: Vec<(u64, Vec<u32>)> = group(comp, SPLIT_START_LEVEL);
    while let Some((cell_raw, owned)) = frontier.pop() {
        let cell = CellID(cell_raw);
        if owned.len() <= CHUNK_BUDGET || cell.level() >= SPLIT_MAX_LEVEL {
            jobs.push(Job {
                cell: Some(cell),
                owned,
            });
        } else {
            frontier.extend(group(&owned, cell.level() + 1));
        }
    }
    jobs.sort_by_key(|j| j.cell.map(|c| c.0));
    jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Distance, Haversine};

    fn hav(a: PointArray, b: PointArray) -> f64 {
        Haversine.distance(geo::Point::new(a[1], a[0]), geo::Point::new(b[1], b[0]))
    }

    fn score(points: &SingleVec, centers: &SingleVec, radius: f64, m: usize) -> usize {
        // Mirrors stats.rs::get_score semantics (cell-20 dedup via direct
        // distance check per input point).
        let covered = points
            .iter()
            .filter(|p| centers.iter().any(|c| hav(**p, *c) <= radius))
            .count();
        centers.len() * m + (points.len() - covered)
    }

    #[test]
    fn empty_input() {
        let crucible = Crucible::default();
        assert!(crucible.run(&vec![]).is_empty());
    }

    #[test]
    fn single_point_m1() {
        let crucible = Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
        };
        let pts = vec![[40.0, -74.0]];
        let out = crucible.run(&pts);
        assert_eq!(out.len(), 1);
        assert!(hav(out[0], pts[0]) <= 70.0);
    }

    #[test]
    fn full_coverage_at_m1() {
        // 400-point jittered grid, every point must be covered.
        let mut pts: SingleVec = vec![];
        for i in 0..20 {
            for j in 0..20 {
                pts.push([
                    40.0 + i as f64 * 0.0009 + ((i * 7 + j * 13) % 10) as f64 * 1e-5,
                    -74.0 + j as f64 * 0.0009,
                ]);
            }
        }
        let crucible = Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
        };
        let centers = crucible.run(&pts);
        for p in &pts {
            assert!(
                centers.iter().any(|c| hav(*p, *c) <= 70.0),
                "uncovered point at m=1"
            );
        }
        // Must use far fewer centers than points.
        assert!(centers.len() < pts.len() / 2, "got {}", centers.len());
    }

    #[test]
    fn beats_or_matches_trivial_solution() {
        // Pairs of points 100 m apart, in groups separated by 1 km: optimal is
        // one center per pair; per-point singletons would be 2× worse.
        let mut pts: SingleVec = vec![];
        for g in 0..10 {
            let lat = 40.0 + g as f64 * 0.01;
            pts.push([lat, -74.0]);
            pts.push([lat, -74.0 + 0.00118]); // ~100 m east
        }
        let crucible = Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
        };
        let centers = crucible.run(&pts);
        assert_eq!(centers.len(), 10, "one disk per pair");
        assert_eq!(score(&pts, &centers, 70.0, 1), 10);
    }

    #[test]
    fn max_clusters_respected() {
        let mut pts: SingleVec = vec![];
        for i in 0..50 {
            pts.push([40.0 + i as f64 * 0.01, -74.0]);
        }
        let crucible = Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: 5,
        };
        let centers = crucible.run(&pts);
        assert!(centers.len() <= 5);
    }

    /// Diagnostic: where does legacy-Best find gain that crucible misses at m=3?
    /// Run with: cargo test -p algorithms debug_m3_gap --release -- --ignored --nocapture
    #[test]
    #[ignore]
    fn debug_m3_gap() {
        use crate::clustering::greedy::Greedy;
        use koji_core::ClusterMode;

        let mut pts: SingleVec = vec![];
        let mut x: u64 = 42;
        for _ in 0..1000 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = 40.0 + ((x >> 11) as f64 / (1u64 << 53) as f64) * 0.0285;
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let b = -74.0 + ((x >> 11) as f64 / (1u64 << 53) as f64) * 0.0375;
            pts.push([a, b]);
        }
        let crucible = Crucible {
            radius: 70.0,
            min_points: 3,
            max_clusters: usize::MAX,
        };
        let a_centers = crucible.run(&pts);
        let mut greedy = Greedy::default();
        greedy
            .set_cluster_mode(ClusterMode::Best)
            .set_radius(70.0)
            .set_min_points(3);
        let l_centers = greedy.run(&pts);

        let covered_by = |centers: &SingleVec, r: f64| -> Vec<bool> {
            pts.iter()
                .map(|p| centers.iter().any(|c| hav(*p, *c) <= r))
                .collect()
        };
        let a_cov = covered_by(&a_centers, 70.0);
        let score = |centers: &SingleVec| -> usize {
            centers.len() * 3 + covered_by(centers, 70.0).iter().filter(|c| !**c).count()
        };
        eprintln!(
            "crucible: {} clusters score {} | legacy: {} clusters score {}",
            a_centers.len(),
            score(&a_centers),
            l_centers.len(),
            score(&l_centers)
        );
        // For each legacy center: how many crucible-uncovered points does it
        // reach at full r vs effective r?
        let mut hist_full = [0usize; 16];
        let mut hist_eff = [0usize; 16];
        for c in &l_centers {
            let full = pts
                .iter()
                .zip(&a_cov)
                .filter(|(p, cov)| !**cov && hav(**p, *c) <= 70.0)
                .count();
            let eff = pts
                .iter()
                .zip(&a_cov)
                .filter(|(p, cov)| !**cov && hav(**p, *c) <= 70.0 * 0.999)
                .count();
            hist_full[full.min(15)] += 1;
            hist_eff[eff.min(15)] += 1;
        }
        eprintln!("legacy centers by #crucible-uncovered covered (full r): {hist_full:?}");
        eprintln!("legacy centers by #crucible-uncovered covered (r_eff):  {hist_eff:?}");
    }

    #[test]
    fn deterministic_end_to_end() {
        let mut pts: SingleVec = vec![];
        let mut x: u64 = 7;
        for _ in 0..2000 {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let a = 40.0 + ((x >> 11) as f64 / (1u64 << 53) as f64) * 0.05;
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let b = -74.0 + ((x >> 11) as f64 / (1u64 << 53) as f64) * 0.05;
            pts.push([a, b]);
        }
        let crucible = Crucible {
            radius: 70.0,
            min_points: 1,
            max_clusters: usize::MAX,
        };
        assert_eq!(crucible.run(&pts), crucible.run(&pts));
    }
}
