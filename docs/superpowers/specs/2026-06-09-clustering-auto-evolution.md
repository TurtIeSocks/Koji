# Clustering: next evolution ("auto") — design spec

> **Renamed to Crucible (2026-06-12).** The algorithm shipped in code as the
> `crucible` module / `Crucible` struct; "auto" throughout this dated spec is
> the development-era name, kept as-is for the historical record.

Date: 2026-06-09
Status: approved-by-delegate-mode (assumptions listed at bottom; async review trail)
Replaces: greedy.rs quality modes (`Fast`/`Balanced`/`Better`/`Best`)

## 1. Problem statement

Given `n` geographic points (lat/lng, WGS84; 100 ≤ n ≤ 500,000), radius `r` meters,
`min_points` `m`, `max_clusters` cap: choose disk centers (any coordinates) to minimize

```
mygod_score = m * |centers| + |uncovered points|        (source of truth: stats.rs::Stats::get_score)
```

A point is covered iff Haversine distance to some center ≤ r. Coverage counting
dedupes points by S2 level-20 cell (~7 m) — covering one point in a cell covers
the cell once, and intra-cell duplicates are irreducibly "uncovered" for every
algorithm. When `m == 1`, contract says every point ends up covered (singleton
clusters are score-neutral).

This is weighted partial unit-disk cover (NP-hard). Score math that drives all
design decisions below:

- adding a cluster covering `k` new points: Δ = m − k → win iff k > m, neutral at k = m
- removing a cluster with `u` uniquely-covered points: Δ = u − m → win iff u < m
- merging two clusters into one without losing coverage: Δ = −m (always win)

## 2. Why the legacy greedy loses points

1. **Candidate set is blind to disk geometry.** Jittered grids + S2 level-21 cell
   centers never propose the classical optimal placements: positions where the
   disk boundary passes through 2 points (pair circle-intersections). Those are
   exactly the placements that grab two far-apart points with one disk.
2. **Greedy ordering quirk.** Within a coverage level, a candidate is skipped
   entirely if *any* of its pre-sorted unique points got blocked — even when it
   still has ≥ level unique points. Ordering-dependent loss.
3. **No local search.** Merge pass exists but is disabled (geodesic SEC per pair
   too slow); no re-center, no prune, no swap.
4. **Gap-fill is a weak afterthought** — input-order single pass, strict-> only.
5. **Nondeterministic** (unseeded TLS RNG jitter in candidate grid).

## 3. New algorithm: `auto`

Mode-less; every knob derived from `n`, density, and `r`. One pipeline, scale-
adaptive. All geometry decisions are made with an effective radius
`r_eff = r * (1 − 1e-3)` (7 cm at 70 m) so planar approximations can never
produce a center whose true Haversine distance exceeds `r` (knife-edge safety:
pair-intersection candidates sit at distance exactly-r by construction, so
backing off 0.1% makes them strictly inside).

### 3.1 Pipeline

```
dedupe (S2 L20)
  → S2 chunking + halo            (bounds memory, parallelism, projection error)
  → per chunk:
      gnomonic frame (project.rs Plane; planar units, 1.0 = r_eff)
      grid hash (cell = 1.0)
      candidates = dedup points ∪ pair circle-intersections (auto-capped K)
      lazy greedy, integer bucket queue, gains recomputed on pop (exact greedy)
      ownership filter (center parents into chunk cell)
  → global stitch passes (exact Haversine coverage, iterated ≤3 rounds):
      prune        (drop cluster when unique < m)
      re-center    (planar Welzl SEC of required points; frees slack → captures strays)
      merge 2→1    (SEC of *required* union ≤ r_eff; stronger than legacy full-union test)
      gap-fill     (same greedy machinery over still-uncovered points)
  → m==1: singleton recovery for any leftovers (score-neutral, contract)
  → max_clusters enforcement (keep greedy-prefix by gain)
```

Key upgrades over legacy, in expected score-impact order:

1. **Pair-intersection candidates** — covers the "2 mutually-far points, 1 disk"
   placements the grid can't express. For each point, up to K nearest neighbors
   within 2·r_eff contribute 2 intersection candidates each.
2. **Exact lazy greedy** — bucket queue indexed by integer gain; pop max, recompute
   gain (monotone non-increasing ⇒ lazy evaluation is exact), commit iff still max
   bucket. No level-skip quirk. Near-linear after candidates.
3. **Required-set merge** — a pair merges if the points *only they* cover fit one
   disk (points also covered by a third cluster may be abandoned by the pair).
   Legacy required the full union to fit. Planar Welzl SEC in a local frame
   (pair extent ≤ 4r ≈ 300 m ⇒ projection error ~1e-9) — fast enough to enable.
4. **Re-center on SEC of required points** — slack reclaimed each round, feeds
   merge and gap-fill.
5. **Deterministic** — no RNG anywhere; ties break by stable candidate index.

### 3.2 Memory model (500k points)

Candidates store only `[f64;2]` + cached gain (no covered-point lists; gains are
recomputed from the grid on pop). 1M candidates ≈ 24 MB. Coverage bitmap is
per-chunk `Vec<bool>`. Chunks bound peak: budget ~24k owned points; rayon runs
≤ NCPU chunks concurrently.

### 3.3 Chunking + projection error budget

S2 recursive split until `owned ≤ budget` AND `level ≥ 8` (~≤ 40 km extent).
Gnomonic `Plane` per chunk: planar-vs-Haversine relative error ≤ ~1e-5 at 40 km
≪ the 1e-3 r_eff margin (verified by a unit test sampling random pairs).
Halo = points within 2r of chunk bbox (envelope query on a global rtree) so
boundary candidates see cross-chunk points; ownership filter prevents duplicate
emission; global stitch passes fix boundary double-coverage (prune/merge see
exact global coverage).

### 3.4 Auto-tuning (replaces ClusterMode)

| knob | rule |
|---|---|
| K (intersection neighbors/point) | 16 if chunk points ≤ 20k; 10 if ≤ 60k; 8 above |
| chunk budget | 24k owned points (level ≥ 8 floor) |
| refine rounds | 3, early-exit when a full round makes no improvement |
| gap-fill | always on (it's part of the greedy queue, not a separate pass) |

`ClusterMode` stays in the API for backward compat: `Fast | Balanced | Better |
Best` all route to `auto` (one deprecation log line). `Fastest`, `Honeycomb`,
`Custom(plugin)` keep their existing paths (different semantics, not quality
modes). Escape hatch: env `KOJI_LEGACY_GREEDY=1` routes back to legacy greedy
for A/B.

## 4. Benchmark plan

`crates/algorithms/src/bin/clusterbench.rs` — seeded synthetic datasets:

- `uniform` — uniform in bbox at fixed density (pts/km²)
- `blobs` — gaussian POI clusters (σ 30–150 m) + sparse background
- `urban` — street-grid lattice + downtown blob + noise (closest to real POI data)

n ∈ {100, 1k, 10k, 100k, 500k} × min_points {1, 3} × r = 70 m. Score via
`Stats::cluster_stats` + `get_score` (identical semantics to prod). Baselines:
legacy Balanced/Better/Best + Fastest. Acceptance: `auto` ≤ legacy-best score on
every cell of the matrix, runtime within ~2× legacy Best (and ≤ ~10 min at 500k).

## 5. Assumptions (delegate mode)

1. **ClusterMode stays in the wire API**, quality modes converge to `auto` —
   dropping the field from request structs would break clients for no score win.
2. **Legacy greedy code is kept** behind `KOJI_LEGACY_GREEDY=1` (and the existing
   `bypass_adaptive_partition` dev flag keeps meaning "legacy, pre-partition").
3. **`Fastest` and `Honeycomb` are out of scope** — different contracts (speed
   king / uniform lattice), not mygod-score modes.
4. ~~Synthetic benchmarks suffice~~ — RESOLVED: user granted DB access; validated
   against real exports (8k/41k/335k/471k stops + spawnpoints). See the results
   doc — real data wins are larger than synthetic (−30 to −53% at m=1).
5. **`center_clusters` post-step unchanged** (orthogonal to clustering).
6. **Determinism is a feature** (same input → same output), accepted even though
   legacy was randomized.
7. Routing/TSP downstream effects out of scope; mygod_score is the only target.
