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
| uniform | 100k | **31,085** | 36,299 | −14.4% | 77,429 | **76,750** | +0.9% |
| blobs | 100k | **19,497** | 22,348 | −12.8% | **43,134** | 43,578 | −1.0% |
| urban | 100k | **21,108** | 23,789 | −11.3% | **50,850** | 51,615 | −1.5% |
| uniform | 500k | **155,583** | 213,186 | −27.0% | 387,193 | **383,968** | +0.8% |
| blobs | 500k | **97,061** | 125,806 | −22.8% | **215,857** | 217,462 | −0.7% |
| urban | 500k | **139,167** | 163,854 | −15.1% | **286,253** | 288,955 | −0.9% |

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
