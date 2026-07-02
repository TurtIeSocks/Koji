# tsp-mt Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Koji's clean-room S2-seeded TSP router with the published tsp-mt crates (standalone `SortBy::Tsp` default + new `SortBy::TspHybrid`), and delete all or-tools/C++ machinery.

**Architecture:** Two repos. Phase A adds a seed-tour `refine()` API to tsp-ils/tsp-geo in `/Users/rin/GitHub/tsp-mt` and publishes v0.2.0/v0.3.0 to crates.io. Phase B wires Koji's `crates/algorithms` to `tsp-geo`, swaps the `SortBy::Tsp` dispatch, adds `TspHybrid`, flips the Route default, and purges or-tools. Plugin machinery (`koji-plugins`, `SortBy::Custom`) stays for external users.

**Tech Stack:** Rust 2024 (1.85+), tsp-ils/tsp-geo (Apache-2.0, crates.io), rayon (native only), wasm32 via nightly `-Z build-std`.

**Spec:** `docs/superpowers/specs/2026-07-02-tsp-mt-integration-design.md`

## Global Constraints

- No wire-exposed solver knobs. Fixed config: `max_neighbors = 20`, `time_limit = None` (auto 2–120 s from n), `seed = 12345` (crate default), `threads = 0` (all cores), everything else tsp-ils defaults.
- `SolverConfig` is `#[non_exhaustive]` — construct via `SolverConfig::default()` then mutate fields; struct literals won't compile outside the crate.
- Koji dep: `tsp-geo = { version = "0.3", default-features = false, features = ["std"] }`. **No `geo-types` feature** — Koji routing operates on `SingleVec = Vec<[Precision; 2]>` (`[lat, lng]` raw arrays, `Precision = f64`); use `GeoPoint::from_lat_lng(c[0], c[1])` directly. (Deviation from spec's Cargo snippet, amended in spec.)
- `tsp-geo/parallel` only via the algorithms crate's `native` feature — wasm builds must stay rayon-free here (koji-wasm brings its own wasm-bindgen-rayon for other code; tsp solve runs sequentially on wasm).
- Koji server crate's Cargo package name is `koji`, NOT `koji-server` (`-p koji-server` errors). Never pipe cargo through `| tail` (masks exit code).
- Determinism caveat: the solver's wall-clock budget makes round counts machine-dependent — same seed does NOT guarantee identical output across runs. Do not write same-output determinism tests against `solve`/`refine` with `std` timing. Assert permutation + length bounds instead.
- tsp-mt repo commits: normal conventional commits. Koji repo: commit freely per project CLAUDE.md.
- Tag-push/publish (Task 4) is outward-facing: give the user a heads-up and wait for go-ahead before `git push origin v0.3.0`.

---

## Phase A — tsp-mt repo (`/Users/rin/GitHub/tsp-mt`)

### Task 1: tsp-ils `refine()` — seed-tour entry point

**Files:**
- Modify: `crates/tsp_ils/src/solve.rs` (solve → solve_with refactor, refine, multi_start signature)
- Modify: `crates/tsp_ils/src/lib.rs` (export + tests)

**Interfaces:**
- Consumes: existing `solve_inner`, `multi_start`, `segment_rounds`, `greedy_tour`, `TourState::new(pts, cand, order, frozen, or_opt_max_len, kick_window, rng)`.
- Produces: `pub fn refine<const D: usize>(pts: &[[f64; D]], initial_tour: &[u32], cfg: &SolverConfig) -> Solution` — panics if `initial_tour` is not a permutation of `0..pts.len()`. Task 2 wraps it.

- [ ] **Step 1: Write failing tests** in `crates/tsp_ils/src/lib.rs` tests module (reuse existing `fast_cfg()` / `assert_permutation` helpers there):

```rust
#[test]
fn refine_returns_permutation_and_never_worsens() {
    // 64 points on a circle; scrambled-but-valid initial tour (27 coprime 64).
    let pts: Vec<[f64; 2]> = (0..64)
        .map(|i| {
            let t = i as f64 / 64.0 * core::f64::consts::TAU;
            [t.sin(), t.cos()]
        })
        .collect();
    let initial: Vec<u32> = (0..64u32).map(|i| (i * 27) % 64).collect();
    let before = cycle_length(&pts, &initial);
    let sol = refine(&pts, &initial, &fast_cfg());
    assert_permutation(&sol.tour, 64);
    assert!(sol.length <= before + 1e-9, "refine must not worsen the tour");
    // Circle optimum is the perimeter; ILS should recover it from a scramble.
    let perimeter: f64 = (0..64)
        .map(|i| dist(&pts[i], &pts[(i + 1) % 64]))
        .sum();
    assert!(sol.length <= perimeter * 1.001, "got {} want ~{perimeter}", sol.length);
}

#[test]
fn refine_tiny_input_passes_through() {
    let pts = [[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]];
    let sol = refine(&pts, &[2, 0, 1], &fast_cfg());
    assert_eq!(sol.tour, vec![2, 0, 1]);
}

#[test]
#[should_panic(expected = "permutation")]
fn refine_rejects_non_permutation() {
    let pts = [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
    refine(&pts, &[0, 1, 2, 2], &fast_cfg());
}
```

Add `refine` and `dist` to the `use super::*;`-visible imports if not already covered (they are — `pub use kdtree::dist` exists; `refine` comes in Step 3).

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p tsp-ils refine 2>&1 | tail -20`  *(tail OK here — checking compile error text, and rerun without tail if ambiguous)*
Expected: FAIL — `cannot find function refine`

- [ ] **Step 3: Implement** in `crates/tsp_ils/src/solve.rs`:

Replace the body of `solve` and add `refine` + `solve_with` + `assert_valid_tour`:

```rust
pub fn solve<const D: usize>(pts: &[[f64; D]], cfg: &SolverConfig) -> Solution {
    solve_with(pts, cfg, None)
}

/// Improve a caller-supplied tour with the same ILS pipeline as [`solve`],
/// skipping greedy construction for the main walker. `initial_tour` must be
/// a permutation of `0..pts.len()` (panics otherwise — see [`is_valid_tour`]
/// -style guarding in higher-level crates for a fallible wrapper).
pub fn refine<const D: usize>(
    pts: &[[f64; D]],
    initial_tour: &[u32],
    cfg: &SolverConfig,
) -> Solution {
    assert_valid_tour(initial_tour, pts.len());
    solve_with(pts, cfg, Some(initial_tour.to_vec()))
}

fn solve_with<const D: usize>(
    pts: &[[f64; D]],
    cfg: &SolverConfig,
    seed_tour: Option<Vec<u32>>,
) -> Solution {
    let n = pts.len();
    if n <= 3 {
        let tour = seed_tour.unwrap_or_else(|| (0..n as u32).collect());
        let length = cycle_length(pts, &tour);
        return Solution { tour, length };
    }
    // Custom pools are unsupported on wasm (wasm-bindgen-rayon initializes
    // the global pool instead), and pool creation can fail under OS resource
    // pressure; in both cases fall back to the current/global pool rather
    // than panicking.
    #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
    match rayon::ThreadPoolBuilder::new()
        .num_threads(cfg.threads)
        .build()
    {
        Ok(pool) => return pool.install(move || solve_inner(pts, cfg, seed_tour)),
        Err(err) => log::warn!("solver: thread pool build failed ({err}); using global pool"),
    }
    solve_inner(pts, cfg, seed_tour)
}

/// Panics when `tour` is not a permutation of `0..n`.
fn assert_valid_tour(tour: &[u32], n: usize) {
    assert_eq!(
        tour.len(),
        n,
        "initial tour length {} != point count {n}; not a permutation",
        tour.len()
    );
    let mut seen = vec![false; n];
    for &v in tour {
        let i = v as usize;
        assert!(
            i < n && !seen[i],
            "initial tour is not a permutation of 0..{n}"
        );
        seen[i] = true;
    }
}
```

Change `solve_inner` to take the seed and construct greedy only when needed (delete the old unconditional `let initial = greedy_tour(...)` block and the `let mut order = if ...` dispatch; keep everything else — deadline, candidates log, final polish — identical):

```rust
fn solve_inner<const D: usize>(
    pts: &[[f64; D]],
    cfg: &SolverConfig,
    seed_tour: Option<Vec<u32>>,
) -> Solution {
    let n = pts.len();
    let start = Instant::now();
    let deadline = start + cfg.budget(n);

    let tree = KdTree::build(pts);
    let cand = Candidates::build(pts, &tree, cfg.max_neighbors, cfg.max_candidates);
    log::info!(
        "solver: candidates built n={n} k={} in {:.2}s",
        cfg.max_neighbors,
        start.elapsed().as_secs_f64()
    );

    let seg_capable = n / cfg.min_segment_len.max(4) >= 2;
    let use_segments = n > cfg.multi_start_max && seg_capable;

    let mut order = match seed_tour {
        // Segment rounds refine the seed directly; greedy is never built.
        Some(seed) if use_segments => segment_rounds(pts, &cand, seed, cfg, deadline),
        // Multi-start: walker 0 refines the seed, the rest explore from greedy.
        Some(seed) => {
            let greedy = greedy_tour(pts, &cand, &tree);
            multi_start(pts, &cand, seed, greedy, cfg, deadline)
        }
        None => {
            let greedy = greedy_tour(pts, &cand, &tree);
            log::info!(
                "solver: greedy tour len={:.0} in {:.2}s",
                cycle_length(pts, &greedy),
                start.elapsed().as_secs_f64()
            );
            if use_segments {
                segment_rounds(pts, &cand, greedy, cfg, deadline)
            } else {
                multi_start(pts, &cand, greedy.clone(), greedy, cfg, deadline)
            }
        }
    };
    // ... rest of the function unchanged (final polish, spike_pass, Solution)
```

Change `multi_start` to two tours (walker 0 → `primary`, others → `alt`; when both are the same greedy tour, behavior is identical to today):

```rust
/// One independent iterated-local-search walker per core; best tour wins.
/// Walker 0 starts from `primary`, all other walkers from `alt`.
fn multi_start<const D: usize>(
    pts: &[[f64; D]],
    cand: &Candidates,
    primary: Vec<u32>,
    alt: Vec<u32>,
    cfg: &SolverConfig,
    deadline: Instant,
) -> Vec<u32> {
```

and inside the `walker` closure replace `initial.clone()` with:

```rust
        let start_tour = if run == 0 { primary.clone() } else { alt.clone() };
```

(`TourState::new(pts, cand, start_tour, ...)` — rest unchanged.)

Export the new entry point in `crates/tsp_ils/src/lib.rs`:

```rust
pub use solve::{Solution, SolverConfig, cycle_length, refine, solve};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p tsp-ils`
Expected: all pass, including the three new tests and every pre-existing test (no behavior change for `solve`).

- [ ] **Step 5: Commit**

```bash
git add crates/tsp_ils && git commit -m "feat(tsp-ils): refine() — seed-tour entry point sharing the ILS pipeline"
```

### Task 2: tsp-geo `refine_order()`

**Files:**
- Modify: `crates/tsp_geo/src/solve.rs`
- Modify: `crates/tsp_geo/src/lib.rs` (export)

**Interfaces:**
- Consumes: `tsp_ils::refine` (Task 1), existing `GeoPoint::is_valid` / `unit_sphere_meters`, `Error::invalid_input`.
- Produces: `pub fn refine_order(points: &[GeoPoint], initial_tour: &[u32], cfg: &SolverConfig) -> Result<Vec<u32>>` — `Err(Error::InvalidInput)` on bad coords or non-permutation tour, never panics. Koji Task 7 calls this.

- [ ] **Step 1: Write failing tests** in `crates/tsp_geo/src/solve.rs` tests module (reuse `fast_config()`):

```rust
#[test]
fn refine_order_improves_a_scrambled_ring() {
    let n = 48;
    let center = (52.52, 13.405);
    let nodes: Vec<GeoPoint> = (0..n)
        .map(|i| {
            let t = i as f64 / n as f64 * std::f64::consts::TAU;
            GeoPoint::from_lat_lng(center.0 + 0.01 * t.sin(), center.1 + 0.016 * t.cos())
        })
        .collect();
    let scrambled: Vec<u32> = (0..n as u32).map(|i| (i * 11) % n as u32).collect();
    let tour_len = |order: &[u32]| -> f64 {
        (0..n)
            .map(|i| nodes[order[i] as usize].dist(&nodes[order[(i + 1) % n] as usize]))
            .sum()
    };
    let before = tour_len(&scrambled);
    let refined = super::refine_order(&nodes, &scrambled, &fast_config()).expect("refine");
    assert_eq!(refined.len(), n);
    let mut seen = vec![false; n];
    for &idx in &refined {
        assert!(!seen[idx as usize]);
        seen[idx as usize] = true;
    }
    assert!(tour_len(&refined) < before, "must shorten a scrambled ring");
}

#[test]
fn refine_order_rejects_bad_tour() {
    let nodes = vec![
        GeoPoint::from_lat_lng(10.0, 20.0),
        GeoPoint::from_lat_lng(11.0, 21.0),
        GeoPoint::from_lat_lng(12.0, 22.0),
        GeoPoint::from_lat_lng(13.0, 23.0),
    ];
    for bad in [vec![0u32, 1, 2], vec![0, 1, 2, 2], vec![0, 1, 2, 9]] {
        let err = super::refine_order(&nodes, &bad, &fast_config())
            .expect_err("non-permutation must be rejected");
        assert!(err.to_string().contains("permutation"), "{err}");
    }
}

#[test]
fn refine_order_rejects_invalid_points() {
    let nodes = vec![
        GeoPoint::from_lat_lng(10.0, 20.0),
        GeoPoint::from_lat_lng(91.0, 20.0),
        GeoPoint::from_lat_lng(11.0, 21.0),
        GeoPoint::from_lat_lng(12.0, 22.0),
    ];
    let err = super::refine_order(&nodes, &[0, 1, 2, 3], &fast_config())
        .expect_err("invalid point should fail");
    assert!(err.to_string().contains("point 1"), "{err}");
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p tsp-geo refine`
Expected: FAIL — `cannot find function refine_order`

- [ ] **Step 3: Implement** in `crates/tsp_geo/src/solve.rs`. Extract the coord check shared with `solve_order` and add:

```rust
fn check_points(points: &[GeoPoint]) -> Result<()> {
    if let Some(bad) = points.iter().position(|p| !p.is_valid()) {
        return Err(Error::invalid_input(alloc::format!(
            "point {bad} has invalid lat/lng values: {}",
            points[bad]
        )));
    }
    Ok(())
}

/// Refine a caller-supplied visiting order for `points` with the full ILS
/// pipeline, returning an improved order (indices into `points`).
///
/// Fails if any point has out-of-range or non-finite coordinates, or if
/// `initial_tour` is not a permutation of `0..points.len()`.
pub fn refine_order(
    points: &[GeoPoint],
    initial_tour: &[u32],
    cfg: &SolverConfig,
) -> Result<Vec<u32>> {
    check_points(points)?;
    if !is_permutation(initial_tour, points.len()) {
        return Err(Error::invalid_input(alloc::format!(
            "initial tour is not a permutation of 0..{}",
            points.len()
        )));
    }
    let pts: Vec<[f64; 3]> = points.iter().map(GeoPoint::unit_sphere_meters).collect();
    let solution = tsp_ils::refine(&pts, initial_tour, cfg);
    log::info!(
        "solver: refined n={} chord_length_m={:.0}",
        points.len(),
        solution.length
    );
    Ok(solution.tour)
}

fn is_permutation(tour: &[u32], n: usize) -> bool {
    if tour.len() != n {
        return false;
    }
    let mut seen = alloc::vec![false; n];
    tour.iter().all(|&v| {
        let i = v as usize;
        i < n && !core::mem::replace(&mut seen[i], true)
    })
}
```

Rewrite `solve_order`'s inline validation to call `check_points(points)?;` (dedup). Export in `crates/tsp_geo/src/lib.rs`:

```rust
pub use solve::{SolverConfig, refine_order, solve, solve_order, solve_order_of};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p tsp-geo`
Expected: all pass (new + pre-existing).

- [ ] **Step 5: Commit**

```bash
git add crates/tsp_geo && git commit -m "feat(tsp-geo): refine_order() — fallible seed-tour refinement"
```

### Task 3: Version bumps + docs + full verify

**Files:**
- Modify: `crates/tsp_ils/Cargo.toml` (`version = "0.2.0"`)
- Modify: `crates/tsp_geo/Cargo.toml` (`version = "0.3.0"`; both `tsp-ils` dep AND dev-dep `version = "0.2.0"`)
- Modify: `README.md` (root), `crates/tsp_ils/src/lib.rs` + `crates/tsp_geo/src/lib.rs` module docs

**Interfaces:**
- Produces: publishable tsp-ils 0.2.0 / tsp-geo 0.3.0. Task 4 tags them.

- [ ] **Step 1:** Bump `crates/tsp_ils/Cargo.toml` version to `0.2.0`. In `crates/tsp_geo/Cargo.toml`: version to `0.3.0`, `tsp-ils = { path = "../tsp_ils", version = "0.2.0", default-features = false }`, dev-dep `tsp-ils = { path = "../tsp_ils", version = "0.2.0" }`.
- [ ] **Step 2:** Root `README.md`: the Koji note (~line 77, "copy binary to plugin folder") is stale — replace with: Koji ≥ v2 consumes `tsp-ils`/`tsp-geo` as library crates directly (`SortBy::Tsp` standalone, `SortBy::TspHybrid` S2-seeded refine); the CLI binary remains for standalone use. Mention `refine`/`refine_order` in the crate feature lists of both `lib.rs` module docs (one line each: "Seed-tour refinement — `refine`/`refine_order` improve a caller-supplied order with the same pipeline").
- [ ] **Step 3:** Full verify (parallel, background if >5 s): `cargo test --workspace` and `cargo clippy --workspace --all-targets` and `cargo build --release`.
Expected: all green, zero clippy warnings.
- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "chore: bump tsp-ils 0.2.0, tsp-geo 0.3.0 for the refine API"
```

### Task 4: Publish gate (USER HEADS-UP REQUIRED)

- [ ] **Step 1:** Tell the user: about to `git push` tsp-mt main + tag `v0.3.0`, which triggers the crates.io publish workflow for both crates. Wait for go-ahead.
- [ ] **Step 2:** After approval:

```bash
git push origin main && git tag v0.3.0 && git push origin v0.3.0
```

- [ ] **Step 3:** Watch the workflow: `gh run watch --repo TurtIeSocks/tsp-mt` (or `gh run list --repo TurtIeSocks/tsp-mt --workflow=publish.yml`). Expected: publish job green, both crates published (tag `v0.3.0` matches tsp-geo; tsp-ils 0.2.0 is new so it publishes too, in dependency order).
- [ ] **Step 4:** Verify availability: `cargo info tsp-geo` shows 0.3.0 (retry ~30 s intervals; index propagation is quick). Blocks Task 5.

---

## Phase B — Koji repo (`/Users/rin/GitHub/Koji`, branch claude/v2)

### Task 5: Cargo wiring

**Files:**
- Modify: `Cargo.toml` (workspace deps)
- Modify: `crates/algorithms/Cargo.toml`

**Interfaces:**
- Produces: `tsp_geo` crate importable from `algorithms` with `parallel` on native, sequential on wasm. Tasks 6–7 build on it.

- [ ] **Step 1:** Root `Cargo.toml` `[workspace.dependencies]`, external shared section, alphabetical near `s2`:

```toml
# Sequential core by default so koji-wasm stays rayon-free here; the
# algorithms crate's `native` feature re-adds `tsp-geo/parallel`.
tsp-geo = { version = "0.3", default-features = false, features = ["std"] }
```

- [ ] **Step 2:** `crates/algorithms/Cargo.toml`: add `tsp-geo = { workspace = true }` to `[dependencies]`; extend the feature:

```toml
native = ["dep:sysinfo", "dep:koji-plugins", "dep:colored", "tsp-geo/parallel"]
```

- [ ] **Step 3:** `cargo check -p algorithms` — compiles (dep resolves from crates.io, `Cargo.lock` updates).
- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock crates/algorithms/Cargo.toml
git commit -m "build(algorithms): depend on tsp-geo 0.3 (parallel behind native feature)"
```

### Task 6: `SortBy::TspHybrid` variant

**Files:**
- Modify: `crates/algorithms/src/routing/sort_by.rs`

**Interfaces:**
- Produces: `SortBy::TspHybrid`, canonical string `"tsphybrid"`, alias `"hybrid"`. Task 7 dispatches it; the `StrEnum` derive supplies `from_str_opt`/`Display`.

- [ ] **Step 1: Write failing test** (extend existing tests in the file):

```rust
#[test]
fn tsp_hybrid_parses_and_displays() {
    assert_eq!(SortBy::from_str_opt("tsphybrid"), Some(SortBy::TspHybrid));
    assert_eq!(SortBy::from_str_opt("hybrid"), Some(SortBy::TspHybrid));
    assert_eq!(format!("{}", SortBy::TspHybrid), "tsphybrid");
}
```

- [ ] **Step 2:** Run: `cargo test -p algorithms sort_by` — Expected: FAIL, no variant `TspHybrid`.
- [ ] **Step 3:** Add the variant after `Tsp` (before the `#[str(default)] Custom` arm — the default-catchall must stay last):

```rust
    #[str("tsp", alias("optimized", "2opt"))]
    Tsp,
    #[str("tsphybrid", alias("hybrid"))]
    TspHybrid,
```

Note: this makes the enum non-exhaustive for match sites — `routing::main` in `mod.rs` will stop compiling until Task 7 adds the arm. Fine if Tasks 6+7 land in one worktree session; otherwise add a temporary `SortBy::TspHybrid => clusters,` arm and let Task 7 replace it (the plan assumes same-session sequential execution — no temporary arm needed, Step 4 runs the sort_by tests only).
- [ ] **Step 4:** Run: `cargo test -p algorithms sort_by` — compile may fail on `mod.rs` match: if so proceed straight to Task 7 (verify at Task 7 Step 4); if it compiles, expect PASS.
- [ ] **Step 5:** Commit together with Task 7 (single unit — dispatch + variant).

### Task 7: tsp adapter module + dispatch + delete two_opt

**Files:**
- Create: `crates/algorithms/src/routing/tsp.rs`
- Delete: `crates/algorithms/src/routing/two_opt.rs`
- Modify: `crates/algorithms/src/routing/mod.rs`

**Interfaces:**
- Consumes: `tsp_geo::{GeoPoint, SolverConfig, solve_order, refine_order}` (0.3.0), `super::sorting::sort_s2`, `koji_core::SingleVec`.
- Produces: `pub(super) fn solve(clusters: SingleVec) -> SingleVec` and `pub(super) fn refine(clusters: SingleVec) -> SingleVec` in `routing::tsp`; `routing::main` dispatches `Tsp`/`TspHybrid`; `all_routing_options()` includes `"tsphybrid"`.

- [ ] **Step 1:** Create `crates/algorithms/src/routing/tsp.rs`:

```rust
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
/// Solver rejection (non-finite coords) logs and falls back to the S2 sort.
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
```

- [ ] **Step 2:** `crates/algorithms/src/routing/mod.rs`: replace `mod two_opt;` with `mod tsp;`; replace the dispatch arm and add the hybrid arm:

```rust
        SortBy::Tsp => tsp::solve(clusters),
        SortBy::TspHybrid => tsp::refine(sort_s2(clusters)),
```

In `all_routing_options()` add after the `"tsp"` push:

```rust
    options.push("tsphybrid".to_string());
```

Add to the mod.rs tests (mirrors `tsp_preserves_all_clusters_and_routes`):

```rust
    #[test]
    fn tsp_hybrid_preserves_all_clusters_and_routes() {
        let (data, clusters) = sample_data();
        let mut stats = Stats::new("t".into(), 1);
        let out = main(
            &data,
            clusters.clone(),
            70.0,
            &make_cfg(SortBy::TspHybrid),
            &mut stats,
        );
        assert_eq!(out.len(), clusters.len());
        assert!(stats.total_distance > 0.0);
    }
```

and extend `all_routing_options_includes_built_ins` with:

```rust
        assert!(opts.contains(&"tsphybrid".to_string()));
```

- [ ] **Step 3:** Delete the old refiner: `git rm crates/algorithms/src/routing/two_opt.rs`
- [ ] **Step 4:** Run: `cargo test -p algorithms` (background; the tsp tests take ~2 s each — solver time floor). Expected: all pass, no references to `two_opt` remain (`grep -rn two_opt crates/ apps/` → only `.claude/worktrees` copies).
- [ ] **Step 5: Commit** (covers Tasks 6+7)

```bash
git add -A
git commit -m "feat(algorithms): tsp-mt is the TSP router — SortBy::Tsp standalone, new SortBy::TspHybrid; drop clean-room two_opt"
```

### Task 8: Route default flip

**Files:**
- Modify: `crates/koji-service/src/public/v2/calc.rs:236-239` (+ comment at :124, :222-224)

**Interfaces:**
- Consumes: `SortBy::Tsp` (in scope already).
- Produces: Route calc with unset sort routes via standalone tsp-mt in-process — no plugin subprocess.

- [ ] **Step 1:** Replace at `calc.rs:236-239`:

```rust
    // `route` defaults to the standalone tsp-mt sort when none was supplied.
    if is_route && routing_config.sort_by == SortBy::Unset {
        routing_config.sort_by = SortBy::Tsp;
    }
```

Update the stale doc comments: line 124's `` `sort_by Unset -> Custom("tsp")` override (spec §2 table)`` and lines 222-224's ```sort_by Unset -> Custom("tsp")` override (spec §2 defaults table)`` → both say `` `sort_by Unset -> Tsp` override``.
- [ ] **Step 2:** Run: `cargo test -p koji-service` (background). Expected: PASS. (Package really is `koji-service`; only the server app is named `koji`.) If any test asserted the old `Custom("tsp")` default, update it to `SortBy::Tsp`.
- [ ] **Step 3: Commit**

```bash
git add crates/koji-service && git commit -m "feat(service): Route default sort is SortBy::Tsp (in-process tsp-mt, no plugin)"
```

### Task 9: Purge or-tools + docs sweep

**Files:**
- Delete: `or-tools/` (whole dir: `src/`, `install.sh`, untracked `9.5.2237/`), `plugins/` (only content is `tsp/plugin.toml`)
- Modify: `Dockerfile`, `.gitignore`, `crates/koji-plugins/src/protocol.rs:37`, `crates/koji-plugins/src/manifest.rs:57`, `docker-compose.example.yml` (comment only, if any names or-tools)

**Interfaces:** none produced — pure removal. Plugin machinery (`koji-plugins` crate, `SortBy::Custom`, `KOJI_PLUGINS_DIR`) explicitly SURVIVES.

- [ ] **Step 1:** `git rm -r or-tools plugins && rm -rf or-tools` (the second command clears gitignored build leftovers like `or-tools/9.5.2237`).
- [ ] **Step 2:** `Dockerfile`: delete the entire `FROM debian:bookworm AS or-tools` stage; in the `runner` stage delete both `COPY --from=or-tools ...` lines and `COPY plugins ./plugins`; replace the plugin-registry comment block with:

```dockerfile
# External plugins: mount a directory at /plugins (see docker-compose.example.yml)
# and the koji-plugins registry scans it at startup. No plugins ship in-image —
# routing/clustering are native Rust (tsp-mt, crucible).
ENV KOJI_PLUGINS_DIR=/plugins
```

- [ ] **Step 3:** Verify the registry tolerates a missing/empty `KOJI_PLUGINS_DIR` (read `crates/koji-plugins/src/registry.rs::load`, ~:55-116). Expected: it logs and returns an empty registry. If it hard-errors on a missing dir, change it to `log::info!` + empty `Vec` — server boot must not require the dir.
- [ ] **Step 4:** `.gitignore`: delete the or-tools block (lines 26-29: `# or-tools`, `or-tools/*`, `!or-tools/src`, `or-tools.tar.gz`). Keep the `crates/algorithms/src/**/plugins/**` block (external-plugin dev convention).
- [ ] **Step 5:** Comment updates: `protocol.rs:37` "Legacy line protocol of the bundled OR-Tools `tsp` router: whitespace-…" → "Legacy whitespace `lat,lng` line protocol (predates the JSON protocol; kept for external plugins that speak it)."; `manifest.rs:57` similar — drop the OR-Tools mention.
- [ ] **Step 6:** Docs sweep: `grep -rn -i "or.tools" --include="*.md" --include="*.yml" --include="*.toml" . | grep -v -e "\.claude" -e target -e node_modules` — update every remaining hit (README, docs/, workflow files). Historical specs/plans under `docs/superpowers/` stay untouched (they document the past).
- [ ] **Step 7:** Sanity: `docker build --target server .` is NOT required (slow); instead `grep -n "or-tools" Dockerfile` → no hits, and `cargo check -p koji-plugins` green.
- [ ] **Step 8: Commit**

```bash
git add -A && git commit -m "chore: dump or-tools — C++ plugin, Dockerfile stage, manifests; plugin machinery stays"
```

### Task 10: Example 3-way comparison

**Files:**
- Modify: `crates/algorithms/examples/tsp_route.rs`

**Interfaces:**
- Consumes: `SortBy::TspHybrid` (Task 6), existing `route()` helper in the example.

- [ ] **Step 1:** Update the header doc: "route the cluster centers with the S2 seed sort, the standalone `SortBy::Tsp` solver, and the `SortBy::TspHybrid` S2-seeded refiner, and compare."
- [ ] **Step 2:** In `main()` §3, add the hybrid run alongside the existing two and extend the metrics/printout (mirror the existing S2-vs-Tsp comparison lines — the file prints a two-column table; make it three: `s2`, `tsp`, `hybrid`, each with total km, closing-leg km, and duration; keep the SVG rendering fed by the standalone `tsp_order`):

```rust
    let (s2_order, s2_dur) = route(&clusters, SortBy::S2Cell);
    let (tsp_order, tsp_dur) = route(&clusters, SortBy::Tsp);
    let (hyb_order, hyb_dur) = route(&clusters, SortBy::TspHybrid);
    let (s2_total, s2_close) = metrics(&s2_order);
    let (tsp_total, tsp_close) = metrics(&tsp_order);
    let (hyb_total, hyb_close) = metrics(&hyb_order);
```

- [ ] **Step 3:** Run: `cargo build --release --example tsp_route` — compiles. (A data-file smoke run is optional; if a `lat,lon` CSV is handy: `cargo run --release --example tsp_route -- <file>` and eyeball that hybrid ≤ s2 total.)
- [ ] **Step 4: Commit**

```bash
git add crates/algorithms/examples && git commit -m "docs(example): tsp_route compares s2 vs tsp vs tsphybrid"
```

### Task 11: Final verification (phase gate)

**Files:** none (verification only) — plus `docs/user-stories`/backlog notes if promised elsewhere (none known).

- [ ] **Step 1 (parallel, background):**
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets`
  - `bash crates/koji-wasm/build-wasm.sh` (nightly + `-Z build-std`; plain `cargo build --target wasm32-unknown-unknown` is a known false-failure TLS-link trap — don't use it)
- [ ] **Step 2:** Expected: all green. wasm build proves `tsp-geo` std-only path compiles for wasm32 and `native`-gated rayon stays out.
- [ ] **Step 3:** Live smoke (optional but recommended): boot the server, `POST /api/v2/calculate` with `{"mode":"route", ...}` on a small area, confirm the response routes and the log shows `solver: candidates built` (tsp-mt in-process) instead of plugin spawn. Web map calc panel's Routing select should list `tsphybrid` (options come from `GET /algorithms` → `all_routing_options`).
- [ ] **Step 4:** Confirm spec ↔ implementation match; note any deviations in the final report. Update memory (`koji-bootstrap-routing.md` — two_opt router replaced by tsp-mt; new memory line for tsp-mt integration).
- [ ] **Step 5:** Nothing to commit unless fixes landed; report done + offer merge options (superpowers:finishing-a-development-branch).
