# Radius comb visiting order — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit radius bootstrap points in a comb (interleaved boustrophedon-loop) order so the device's loop has no long closing leg — it ends one row from where it started — at zero extra routing cost.

**Architecture:** A pure helper `comb_order(rows: Vec<Vec<Point>>) -> Vec<Point>` reorders the per-row points (even rows descending, odd rows ascending, each row oriented to chain to the previous one). `generate_circles` is refactored to collect one `Vec<Point>` per lattice row instead of a flat snake, then returns `comb_order(rows)`. No routing or config changes; the default `SortBy::Unset` passthrough carries the order to output.

**Tech Stack:** Rust, `geo` (`Point`, `Haversine`), existing `crates/algorithms` test harness.

**Spec:** `docs/superpowers/specs/2026-06-16-radius-comb-ordering-design.md`

---

## File Structure

- Modify: `crates/algorithms/src/bootstrap/radius.rs`
  - Add free fns `sq_dist` and `comb_order` (near the existing `dot` / `distance_to_segment` helpers).
  - Refactor `generate_circles` to collect `rows: Vec<Vec<Point>>` and return `comb_order(rows)`.
  - Add unit + integration tests in the existing `#[cfg(test)] mod tests`.

No other files change. `flatten_circles`, the keep-condition, the lattice math, `split_multipolygon`, and all routing code are untouched.

---

## Task 1: comb_order + rewire generate_circles (TDD)

**Files:**
- Modify: `crates/algorithms/src/bootstrap/radius.rs`
- Test: same file, `#[cfg(test)] mod tests`

- [ ] **Step 1: Write the failing tests**

Add these to `mod tests` in `crates/algorithms/src/bootstrap/radius.rs`. They reference `comb_order`, which does not exist yet, so the crate will not compile — that is the expected failure.

```rust
    // ── comb_order helpers ────────────────────────────────────────────────────

    /// row r sits at latitude (n_rows - r) so row 0 is the top (highest lat);
    /// columns 0..n_cols at increasing longitude, stored left→right.
    fn build_grid_rows(n_rows: usize, n_cols: usize) -> Vec<Vec<Point>> {
        (0..n_rows)
            .map(|r| {
                (0..n_cols)
                    .map(|c| Point::new(c as f64, (n_rows - r) as f64))
                    .collect()
            })
            .collect()
    }

    /// Multiset key for permutation comparison (coords are small, scale to int).
    fn sorted_xy(pts: &[Point]) -> Vec<(i64, i64)> {
        let mut v: Vec<(i64, i64)> = pts
            .iter()
            .map(|p| ((p.x() * 1000.0).round() as i64, (p.y() * 1000.0).round() as i64))
            .collect();
        v.sort();
        v
    }

    #[test]
    fn comb_order_is_permutation() {
        let rows = build_grid_rows(4, 3);
        let flat: Vec<Point> = rows.iter().flatten().copied().collect();
        let out = comb_order(rows);
        assert_eq!(out.len(), 12, "comb_order must not add or drop points");
        assert_eq!(sorted_xy(&out), sorted_xy(&flat), "same multiset as input");
    }

    #[test]
    fn comb_order_empty_is_empty() {
        assert!(comb_order(vec![]).is_empty());
    }

    #[test]
    fn comb_order_single_row_unchanged() {
        let rows = build_grid_rows(1, 4);
        let out = comb_order(rows.clone());
        assert_eq!(out, rows[0], "single row passes through in order");
    }

    #[test]
    fn comb_order_starts_top_ends_in_adjacent_row() {
        // 5 rows: top lat = 5, second-from-top lat = 4.
        let out = comb_order(build_grid_rows(5, 3));
        assert_eq!(out.first().unwrap().y(), 5.0, "tour starts in the top row");
        assert_eq!(
            out.last().unwrap().y(),
            4.0,
            "comb returns adjacent to the start (row 1), not the far bottom row"
        );
    }

    #[test]
    fn comb_order_filters_empty_rows() {
        let mut rows = build_grid_rows(3, 2); // lats 3, 2, 1
        rows.insert(1, vec![]); // empty row must not shift even/odd parity
        let out = comb_order(rows);
        assert_eq!(out.len(), 6, "empty rows contribute nothing");
        for p in &out {
            assert!([1.0, 2.0, 3.0].contains(&p.y()), "no phantom points: {:?}", p);
        }
    }

    #[test]
    fn comb_closing_leg_is_short_on_tall_polygon() {
        // Tall, skinny rectangle: ~2 km of latitude, ~150 m of longitude.
        // The snake order loops back across the full ~2 km height; the comb ends
        // one row from the start, so the closing leg is a few row pitches.
        let feature = rect_feature(-74.0009, 39.990, -73.9991, 40.010);
        let radius = 70.0;
        let pts = BootstrapRadius::new(&feature, radius).result();
        assert!(pts.len() > 4, "expected a multi-row fill, got {}", pts.len());

        let first = Point::new(pts.first().unwrap()[1], pts.first().unwrap()[0]);
        let last = Point::new(pts.last().unwrap()[1], pts.last().unwrap()[0]);
        let closing = Haversine.distance(first, last);
        assert!(
            closing < 4.0 * radius,
            "comb closing leg {closing:.0} m should be a few row pitches, not the ~2 km height"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p algorithms --lib bootstrap::radius`
Expected: FAIL — compile error `cannot find function 'comb_order' in this scope`.

- [ ] **Step 3: Add `sq_dist` and `comb_order`**

Insert these free functions in `crates/algorithms/src/bootstrap/radius.rs`, just below the existing `point_line_distance` function (before the `#[cfg(test)] mod tests`):

```rust
/// Squared planar distance in lon/lat degrees. Only used for *local*
/// nearest-end comparisons inside `comb_order`, where monotonicity with true
/// distance is all that matters (no geodesic needed at row scale).
fn sq_dist(a: Point, b: Point) -> Precision {
    let dx = a.x() - b.x();
    let dy = a.y() - b.y();
    dx * dx + dy * dy
}

/// Reorder `rows` (emitted top→bottom, each the kept centers of one lattice
/// row) into a comb / boustrophedon-loop visiting order: down the even-indexed
/// rows, back up the odd-indexed rows, orienting each row to enter at the end
/// nearest the previous row's exit. The tour ends in the row adjacent to where
/// it began, so the closing leg (last→first, the device's loop-back) is ~one
/// row pitch instead of the full polygon height. Pure permutation of the input
/// points.
fn comb_order(rows: Vec<Vec<Point>>) -> Vec<Point> {
    let rows: Vec<Vec<Point>> = rows.into_iter().filter(|r| !r.is_empty()).collect();
    let n = rows.len();
    if n <= 1 {
        return rows.into_iter().flatten().collect();
    }

    // Visiting order of row indices: evens ascending, then odds descending.
    // e.g. 6 rows -> [0,2,4, 5,3,1]; 5 rows -> [0,2,4, 3,1].
    let mut order: Vec<usize> = (0..n).step_by(2).collect();
    order.extend((0..n).filter(|i| i % 2 == 1).rev());

    // Emit greedily: orient each row so its entry end is nearest the previous
    // row's exit. Try both orientations of the first row and keep whichever
    // yields the shorter closing leg (last→first).
    let emit = |reverse_first: bool| -> Vec<Point> {
        let mut out: Vec<Point> = Vec::with_capacity(rows.iter().map(|r| r.len()).sum());
        for &idx in &order {
            let mut row = rows[idx].clone();
            match out.last().copied() {
                None if reverse_first => row.reverse(),
                None => {}
                Some(prev) => {
                    let back = *row.last().unwrap();
                    let front = row[0];
                    if sq_dist(prev, back) < sq_dist(prev, front) {
                        row.reverse();
                    }
                }
            }
            out.extend(row);
        }
        out
    };

    let a = emit(false);
    let b = emit(true);
    match (a.first(), a.last(), b.first(), b.last()) {
        (Some(&a0), Some(&an), Some(&b0), Some(&bn)) if sq_dist(bn, b0) < sq_dist(an, a0) => b,
        _ => a,
    }
}
```

- [ ] **Step 4: Rewire `generate_circles` to collect rows and comb them**

Replace the body of `generate_circles` in `crates/algorithms/src/bootstrap/radius.rs`. The ONLY changes vs the current version: `circles: Vec<Point>` becomes `rows: Vec<Vec<Point>>`; a per-row `current_row` is collected and pushed each outer iteration; the function returns `comb_order(rows)`. Lattice constants, margins, keep-condition, south-step, and bearing flip are byte-for-byte unchanged.

```rust
    fn generate_circles(&self, geometry: &Geometry) -> Vec<Point> {
        let mut rows: Vec<Vec<Point>> = vec![];

        let polygon = Polygon::<Precision>::try_from(geometry).unwrap();
        let external_points = polygon.exterior().points().collect::<Vec<Point>>();
        let internal_points: Vec<_> = polygon
            .interiors()
            .iter()
            .map(|interior| interior.points().collect::<Vec<Point>>())
            .collect();

        let x_mod = 0.75_f64.sqrt();
        // 0.5625 = 0.75²: row pitch = 2·√0.5625·r = 1.5r exactly, the pointy-top
        // hex covering pitch. (Was 0.568 → 1.5073r, over-spacing rows ~0.5% and
        // leaving thin uncovered slivers between every trio of circles.)
        let y_mod = 0.5625_f64.sqrt();

        let extremes = polygon.extremes().unwrap();
        let max = Point::new(extremes.x_max.coord.x, extremes.y_max.coord.y);
        let min = Point::new(extremes.x_min.coord.x, extremes.y_min.coord.y);

        let start = Haversine.destination(max, 90.0, self.radius * 1.5);
        let end = Haversine.destination(min, 270., self.radius * 1.5);
        let end = Haversine.destination(end, 180., self.radius);

        let mut row = 0;
        let mut bearing = 270.;
        let mut current = max;

        while current.y() > end.y() {
            let mut current_row: Vec<Point> = vec![];
            while (bearing == 270. && current.x() > end.x())
                || (bearing == 90. && current.x() < start.x())
            {
                if polygon.contains(&current)
                    || point_line_distance(&external_points, &current) <= self.radius
                    || internal_points
                        .par_iter()
                        .any(|internal| point_line_distance(internal, &current) <= self.radius)
                {
                    current_row.push(current);
                }
                current = Haversine.destination(current, bearing, x_mod * self.radius * 2.)
            }
            if !current_row.is_empty() {
                rows.push(current_row);
            }
            current = Haversine.destination(current, 180., y_mod * self.radius * 2.);

            if row % 2 == 1 {
                bearing = 270.;
            } else {
                bearing = 90.;
            }
            current = Haversine.destination(current, bearing, x_mod * self.radius * 3.);

            row += 1;
        }

        comb_order(rows)
    }
```

- [ ] **Step 5: Run the radius tests to verify they pass**

Run: `cargo test -p algorithms --lib bootstrap::radius`
Expected: PASS — all existing radius tests plus the 6 new tests (`comb_order_is_permutation`, `comb_order_empty_is_empty`, `comb_order_single_row_unchanged`, `comb_order_starts_top_ends_in_adjacent_row`, `comb_order_filters_empty_rows`, `comb_closing_leg_is_short_on_tall_polygon`) green.

- [ ] **Step 6: Lint**

Run: `cargo clippy -p algorithms --lib`
Expected: no warnings, no errors. (`comb_order` is reachable from `generate_circles`, so no dead-code warning.)

- [ ] **Step 7: Commit**

```bash
git add crates/algorithms/src/bootstrap/radius.rs
git commit -m "feat(bootstrap): comb visiting order for radius bootstrap

generate_circles now collects one Vec per lattice row and returns
comb_order(rows): even rows descending, odd rows ascending, each row
oriented to chain to the previous one. The tour ends adjacent to its
start, so the device's loop-back closing leg is ~one row pitch instead
of the full polygon height. Pure reorder — same points, same coverage.
Default SortBy::Unset passthrough carries the order to output.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

## Task 2: Wider verification gate

**Files:** none (verification only).

- [ ] **Step 1: Run the full algorithms suite + lint in parallel**

Run (single batch):
- `cargo test -p algorithms`
- `cargo clippy -p algorithms --all-targets`

Expected: all tests pass; clippy clean. If anything fails, fix inline and re-run before proceeding.

- [ ] **Step 2: Confirm no behavioral leak into routing**

Read `crates/algorithms/src/bootstrap/mod.rs` lines around the `CalculationMode::Radius` arm and confirm `new_radius.sort(routing)` is unchanged and still the only routing call. The comb is a generation-order change only.

Expected: no edit needed — just a confirmation that the change is contained to `radius.rs`.

---

## Self-Review

**Spec coverage:**
- Invariant (pure reorder) → `comb_order_is_permutation`, `comb_order_filters_empty_rows`. ✓
- Comb algorithm (even-down/odd-up, greedy orientation, wrap handling) → `comb_order` impl + `comb_order_starts_top_ends_in_adjacent_row`. ✓
- Code placement (rows collected in `generate_circles`, returns `comb_order`) → Task 1 Step 4. ✓
- Routing interaction (default Unset preserves; no new SortBy/config) → no routing edit; Task 2 Step 2 confirms. ✓
- Multipolygon (per-polygon comb via existing `flat_map`) → unchanged `flatten_circles`; each sub-polygon's `generate_circles` now returns comb order. ✓
- Edge cases (0/1 row) → `comb_order_empty_is_empty`, `comb_order_single_row_unchanged`. ✓
- Testing (permutation, closing-leg property, edge cases, determinism) → covered. Determinism is inherent (no RNG; `sort` is stable; `step_by`/`filter` deterministic).

**Placeholder scan:** no TBD/TODO; all code shown in full. Test helpers are `build_grid_rows` and `sorted_xy`; both fully defined.

**Type consistency:** `comb_order(Vec<Vec<Point>>) -> Vec<Point>`, `sq_dist(Point, Point) -> Precision`, `Point` from `geo` (`.x()` = lon, `.y()` = lat). `result()` yields `SingleVec` = `Vec<[lat, lon]>`, so the integration test reads `[1]` as lon and `[0]` as lat into `Point::new(lon, lat)`. Consistent throughout.
