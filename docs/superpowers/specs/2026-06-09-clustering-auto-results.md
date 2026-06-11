# Clustering "auto" — benchmark results

Date: 2026-06-09 · Companion to: 2026-06-09-clustering-auto-evolution.md
Harness: `cargo run --release -p algorithms --bin clusterbench` (seed 42, r = 70 m).
Legacy baseline = the **best score across legacy modes** (fastest/balanced/best)
for each cell — i.e. auto competes against a per-cell oracle mode picker the
real system never had.

## mygod_score (lower = better), auto vs legacy best-of-mode

| dataset | n | m=1 auto | m=1 legacy | Δ | m=3 auto | m=3 legacy | Δ |
|---|---|---|---|---|---|---|---|
| uniform | 100 | **33** | 34 | −3% | **78** | 79 | −1% |
| blobs | 100 | **28** | 29 | −3% | **55** | 56 | −2% |
| urban | 100 | **34** | 36 | −6% | 66 | **65** | +1.5% |
| uniform | 1k | **318** | 333 | −4.5% | 781 | **774** | +0.9% |
| blobs | 1k | **199** | 212 | −6% | **436** | 444 | −1.8% |
| urban | 1k | **200** | 215 | −7% | 537 | **531** | +1.1% |
| uniform | 10k | **3124** | 3332 | −6.2% | 7753 | **7729** | +0.3% |
| blobs | 10k | **1939** | 2088 | −7.1% | **4339** | 4421 | −1.9% |
| urban | 10k | **1852** | 2005 | −7.6% | **4950** | 5025 | −1.5% |
| uniform | 100k | **31,061** | 36,299 | −14.4% | 77,429 | **76,750** | +0.9% |
| blobs | 100k | **19,497** | 22,348 | −12.8% | **43,134** | 43,578 | −1.0% |
| urban | 100k | **21,098** | 23,789 | −11.3% | **50,850** | 51,615 | −1.5% |
| uniform | 500k | **155,530** | 213,186 | −27.1% | 387,193 | **383,968** | +0.8% |
| blobs | 500k | **97,029** | 125,806 | −22.9% | **215,857** | 217,462 | −0.7% |
| urban | 500k | **139,145** | 163,854 | −15.1% | **286,253** | 288,955 | −0.9% |

## Real scanner data (dev DB exports, r = 70 m)

The decisive table — real POI/spawn geometry favors auto far more than the
synthetic generators do:

| dataset | m=1 auto | m=1 legacy | Δ | m=3 auto | m=3 legacy | Δ |
|---|---|---|---|---|---|---|
| pokestops 8k (dev_golbat) | **2,620** | 4,886 | **−46.4%** | 6,289 | **6,280** | +0.14% |
| spawnpoints 41k (dev_golbat) | **3,158** | 6,510 | **−51.5%** | **9,420** | 9,935 | −5.2% |
| pokestops 335k (ne_golbat) | **165,414** | 236,920 | **−30.2%** | **289,454** | 289,720 | −0.09% |
| spawnpoints 471k (rdmct_dev) | **28,193** | 60,155 | **−53.1%** | **84,057** | 89,724 | −6.3% |

Legacy column is again best-of-mode (balanced wins m=1; best wins m=3; legacy
`best` at m=1 melts down on real data — 499,940 clusters on the 471k set vs
auto's 28,193). Auto runtime on the 471k set: ~24 s vs legacy best ~43 s.
Full coverage at m=1 reaches the duplicate-cell ceiling on every set.

The 471k run also flushed out a real bug: duplicated 3→2-swap trio indices
(two centers sharing an S2 L20 cell) double-killed a center, underflowed the
u32 coverage counts, and left 4 cells uncovered at m=1. Fixed (neighbor
dedupe + non-distinct-trio guard + hard liveness assert); a permanent m=1
coverage audit (score-neutral singleton recovery, warn on trigger) and an
opt-in `KOJI_AUTO_PARANOID=1` per-pass invariant checker now guard the
contract.

## Production radii + LNS (final state)

User-confirmed operating points: pokestops r = 78 m, spawnpoints r = 70 m,
min_points up to 5. The LNS ruin-and-recreate pass (windows on uncovered
points, local re-solve, strict-win acceptance; stalls only, no-op at m=1)
plus the exhaustion fix (expensive passes get a post-loop shot when the
round budget drains mid-churn) close every remaining losing cell:

| dataset | m=1 | m=3 | m=5 |
|---|---|---|---|
| stops 8k @78 | **2,351** vs 4,451 (−47%) | **5,785** vs 5,804 (−0.3%) | **7,358** vs 7,397 (−0.5%) |
| stops 335k @78 | **155,021** vs 223,835 (−31%) | **277,284** vs 277,476 (−0.07%) | **318,765** vs 319,173 (−0.13%) |
| spawns 41k @70 | **3,158** vs 6,519 (−52%) | **9,366** vs 9,965 (−6.0%) | **14,659** vs 15,300 (−4.2%) |
| spawns 471k @70 | **28,168** vs 60,184 (−53%) | **83,546** vs 89,639 (−6.8%) | **131,049** vs 138,648 (−5.5%) |

(auto in bold vs legacy best-of-mode.) Auto now wins **every** real-data
cell at every tested min_points. Wall at 471k: ~48 s (m=1), ~70 s (m≥3) vs
legacy best ~39–49 s — the LNS premium buys the m≥3 wins; legacy balanced
remains the speed option at 0.6 s but 1.8–2.6× worse scores.

Future m≥2 ideas, unimplemented: per-window exact ILP/branch-and-bound
(≤150 pts) instead of greedy re-solve; 4→3 swap tier; simulated-annealing
acceptance for LNS windows (would cost determinism).

## True-legacy correction (`--bypass`)

The branch's legacy "best" carries experimental adaptive-partition work and
badly understates the original algorithm. `clusterbench --legacy --bypass`
routes through the pre-partition path (= main-branch algorithm shape). The
honest comparison, after adding the lattice restart variant and full-sweep
LNS for small inputs (auto = final state below; legacy = best of bypass
best/better, which score near-identically):

| dataset | m | auto | true legacy | Δ |
|---|---|---|---|---|
| stops 8k @78 | 1 | **2,347** / 5.3s | 2,434 / 3.1s | −3.6% |
| stops 8k @78 | 3 | 5,781 / 5.3s | **5,724** / 2.6s | +1.0% |
| stops 8k @78 | 5 | 7,357 / 5.3s | 7,356 / 2.5s | ±0 |
| spawns 41k @70 | 1 | **3,158** / 7.2s | 3,457 / 4.9s | −8.6% |
| spawns 41k @70 | 3 | **9,369** / 13.2s | 9,646 / 5.4s | −2.9% |
| spawns 41k @70 | 5 | **14,658** / 21.3s | 14,983 / 5.2s | −2.2% |
| stops 335k @78 | 1 | **155,021** / 13.3s | OOM (>120 GB) | — |
| stops 335k @78 | 3 | **277,237** / 13.8s | OOM | — |
| stops 335k @78 | 5 | **318,756** / 13.1s | OOM | — |
| spawns 471k @70 | 1 | **28,168** / 45s | 31,719 / 97s | −11.2% |
| spawns 471k @70 | 3 | **83,524** / 76s | 88,219 / 77s | −5.3% |
| spawns 471k @70 | 5 | **131,020** / 73s | 136,744 / 52s | −4.2% |

Key corrections to the earlier narrative: the −46…−53% m=1 "wins" were
mostly an artifact of the broken experimental branch path — honest m=1 wins
are 3.6–11%. The true legacy is genuinely strong on small dense POI sets
(beats auto +1.0% on stops-8k m=3; exact per-window solving is the known
counter). Its fatal flaw is memory: the wide-bbox 335k pokestop set needs
>120 GB and dies, while auto runs it in ~14 s — which is what the
experimental partition work on this branch was trying (and failing) to fix.

## Optimization round (2026-06-10)

Seven changes, in order, each tested + benchmarked: (1) propose-parallel /
commit-serial refinement (~6×), (2) dirty-region scheduling (epoch-bumped S2
cells, pure scheduling), (3) GeoGrid bucket index replacing the refiner's
rstar tree, (4) provable m=1 lower bound in clusterbench, (5) exact B&B
window solver (optimal disks + abandonment, ≤64 lost points), (6) two-tier
swap (cheap 3→2 heuristic + exact group-of-4 repack — group-exact alone
regressed m=1 because dense quads exceed the 64-point cap), (7) flag-gated
deterministic annealing (`KOJI_AUTO_ANNEAL=1` — never worse via best-state
restore, but zero measured gain; kept as an experiment harness).

Final state (auto, production radii, score / wall / provable LB):

| dataset | m=1 | m=3 | m=5 |
|---|---|---|---|
| stops 8k @78 | **2,341** / 1.1s (LB 2,007) | 5,764 / 1.3s | **7,355** / 1.2s |
| spawns 41k @70 | **3,160** / 1.5s (LB 2,290) | **9,233** / 1.8s | **14,527** / 1.4s |
| stops 335k @78 | **154,997** / 3.8s (LB 147,009) | **276,496** / 3.4s | **318,664** / 3.0s |
| spawns 471k @70 | **28,105** / 9.1s (LB 21,932) | **81,965** / 10.9s | **129,446** / 7.9s |

Versus the true legacy: auto now wins every cell except stops-8k m=3
(5,764 vs 5,724, +0.7%) — and stops-8k m=5 flipped to a 1-point win. At
471k: 10.7× faster than legacy-better at m=1 (9.1s vs 97s) with −11.4%
score; 7× faster at m=3/m=5 with −7.1% / −5.3%. Every cell improved over
the pre-optimization state in BOTH score and wall (4–9× faster).
stops-335k m=1 is provably within 5.4% of optimal.

**Split-until-exact addendum**: lost sets decompose into 2·RHO-connected
components (provably independent), and windows whose components exceed the
exact cap descend into S2 children until everything fits. This made the
exact tier near-universal: 471k m=3 → 81,086 (−8.1% vs legacy), m=5 →
127,776 (−6.6%); 41k m=3 → 9,125 (−5.4%), m=5 → 14,357; stops-8k m=3 →
5,756 (legacy gap +0.56%, the single remaining loss). A parent-level
boundary sweep (closest S2 analogue of a shifted grid) measured zero on
these datasets — kept as free insurance. Next lever for the last cell:
u128 exact masks (≤128-point components) or an LP-strength B&B bound.

r = 80 m spot-check (10k, user's upper radius): auto wins all four cells —
urban m=1 1571 vs 1711, urban m=3 4246 vs 4318, blobs m=1 1707 vs 1839,
blobs m=3 3885 vs 3907.

## Runtime (wall, M-series 14-core)

| n | auto | legacy `best` | legacy `balanced` |
|---|---|---|---|
| 10k | 0.6–4.5 s | 0.6–0.9 s | 0.06–0.2 s |
| 100k | 2.5–24 s | 4.6–14 s | 0.2–0.6 s |
| 500k | **14–27 s** | 48–162 s | 0.6–6.9 s |

Deterministic: identical input → identical output (verified across runs;
legacy was RNG-jittered).

## Reading the table

- **min_points = 1 (the common case): auto wins every cell, by 3–27%,** with
  the margin growing with n (legacy's grid candidates degrade at scale; the
  component/chunk machinery doesn't).
- **min_points = 3: auto wins all blob/urban cells** (realistic POI shapes)
  by ~1–2%; legacy keeps a 0.3–1% edge on uniform-random scatter and tiny
  urban/uniform instances. That residual is a greedy-trajectory artifact of
  set-packing — attacked with density tie-breaks, post-refine-scored restarts,
  net-gain relocates, 3→2 swaps, and lattice candidates; only the restarts and
  swaps moved it, and none closed it fully. Accepted: uniform-random scatter
  at m≥2 is the least realistic workload, and the trade buys the m=1 and
  realistic-m=3 wins plus 3–6× faster large-n runtime.

## Negative results (kept for the next person)

1. **Commit-time SEC recentering** (polish each committed disk to its claim's
   SEC): strictly worse everywhere. Boundary placements leave a better
   remainder than centered ones.
2. **Net-gain relocate with shedding** (abandon k exclusives to capture >k
   uncovered): each move is a real −1, but stretched exclusive sets block −m
   merges downstream; net regression at m≥2. Loss-free relocates only.
3. **Per-job variant selection by pre-refine score**: mispredicts post-refine
   outcomes; variant choice only pays when scored after refinement.
4. **Dense interior lattice candidates for m≥2**: zero movement on the stuck
   uniform cells, slight harm elsewhere. Points + pair vertices suffice.
5. **Gnomonic `project.rs::Plane` as the solver frame**: ~2.5e-3 relative
   distance error (ellipsoid-scale bias) breaks the 1e-3 margin. The scorer is
   spherical Haversine, so the frame is a pure-sphere azimuthal equidistant
   projection (error ≤ ~5e-5 at 50 km).

## koji_score v2 (2026-06-11)

`Stats` now carries `score_v2` + `score_components` alongside the untouched
`mygod_score`: Tier 1 `cluster_cost + uncovered_cost` (≡ mygod with default
weights), Tier 2 route estimate (S2-sorted tour, approximate cooldown curve),
Tier 3 `knife_edge` (covered points with <5% radius margin), plus the
independent-set `lb` and a normalized `quality = lb/score` ratio (m=1).
Composite weights are env-tunable (`KOJI_SCORE_LAMBDA_ROUTE/KNIFE`, default 0
→ wire-identical scores). The optimizer still targets Tier 1 — route terms
are reported, not optimized (locality). First real read: spawns-41k m=1 is
≥72.5% of provable optimal with 17% knife-edge coverage — the robustness
dimension mygod_score never saw.

## Round 3 (2026-06-11): scoring-driven levers

With koji_score v2 components measurable, seven more levers shipped (and two
honest negatives). All numbers real datasets, stops @78 m, spawns @70 m.

**Landed:**

1. **u128 exact windows + three-way B&B bound** — exact cap 64→128 points;
   bound = max(global k·maxcov, per-point dual-fitting, >2r independent-set
   conflict); per-point candidate index in the branch loop. Dense m≥2
   neighborhoods that used to fall back to greedy now solve optimally:
   spawns-471k m=3 −1,213, m=5 −1,999.
2. **`KOJI_SCORE_LAMBDA_OVERLAP`** — overlap priced into score_v2 *and* the
   spread pass acceptance ((lost − captured) + λ·Δshared < 0). λ=0 stays
   bit-identical/score-neutral. 41k m=3 @λ=0.5: −32 excess for +2 score.
3. **Center-pool recombination** — restart variants pooled (S2 L20 dedupe),
   CELF re-selection (keep while marginal > m), one extra refine, kept only
   on strict internal-score win. stops-8k m=3 −28 in one shot.
4. **Margin pass (knife-edge hardening)** — post-spread, recenter each disk
   on the SEC of its covered set; accept iff worst covered distance strictly
   shrinks, shared coverage doesn't grow, no band loss. Knife-edge −11…−58%
   across cells, score never worse (band captures): stops-8k m=1 knife
   2,484→1,042, score −4.
5. **Warm-start (`Auto::run_seeded`)** — refine a previous solution against
   churned points; construction skipped, `finish()` (m=1 audit + cap)
   shared. 5% churn: 8k 7.4× faster (+1.4% score), 41k m=3 1.6× faster and
   *better* (−82), 471k ~1.1× (refine dominates; use cold there).
   clusterbench: `--warm --churn <pct>`.
6. **Two-variant portfolio for 20k–50k cells** — natural + lattice orderings
   + recombination. 41k: m=1 −31, m=3 −118, m=5 −151 (≈ −1…−1.3%) at
   1.5→3.5–4.6 s wall.
7. **`clusterbench --sweep`** — λ_overlap grid + (score, excess) Pareto
   frontier lines. First read: λ=0.25 strictly dominates λ=0 on 41k m=3.

**Negative results (appended to the list above):**

6. **Ejection via waste-seeded LNS windows** (cells holding centers with
   exclusive < m get ruin windows on big inputs): zero score change on all
   six m≥2 big cells, one +2 noise regression. Wasteful centers surviving
   refinement are already locally optimal within window radius. Reverted.
7. **Route tie-breaking in relocate/spread**: measured first — spread+margin
   already *improve* route_m (−0.1…−0.75%), and final positions are owned by
   the single-candidate spread/margin passes, so mid-refinement tie-breaks
   can't stick. Not implemented.

### Final table (round 3, all 12 real cells vs true legacy)

| dataset | m | auto score / wall | true legacy | Δ |
|---|---|---|---|---|
| stops 8k @78 | 1 | **2,325** / 1.0s | 2,434 / 3.1s | −4.5% |
| stops 8k @78 | 3 | **5,723** / 1.1s | 5,724 / 2.6s | −0.02% |
| stops 8k @78 | 5 | **7,348** / 1.1s | 7,356 / 2.5s | −0.1% |
| spawns 41k @70 | 1 | **3,128** / 3.5s | 3,457 / 4.9s | −9.5% |
| spawns 41k @70 | 3 | **8,987** / 4.6s | 9,646 / 5.4s | −6.8% |
| spawns 41k @70 | 5 | **14,186** / 4.4s | 14,983 / 5.2s | −5.3% |
| stops 335k @78 | 1 | **154,997** / 4.0s | OOM (>120 GB) | — |
| stops 335k @78 | 3 | **276,426** / 3.3s | OOM | — |
| stops 335k @78 | 5 | **318,626** / 2.8s | OOM | — |
| spawns 471k @70 | 1 | **28,089** / 9.0s | 31,719 / 97s | −11.4% |
| spawns 471k @70 | 3 | **80,721** / 13.1s | 88,219 / 77s | −8.5% |
| spawns 471k @70 | 5 | **127,405** / 13.1s | 136,744 / 52s | −6.8% |

**Auto now beats true legacy on every runnable cell — 12/12** (the last
holdout, stops-8k m=3, flipped via recombination + margin-pass captures).
Provable quality at m=1: 0.732–0.948 of the independent-set lower bound.
Knife-edge down 11–58% and overlap_excess down 46–96% vs the pre-spread
state, at equal-or-better scores.
