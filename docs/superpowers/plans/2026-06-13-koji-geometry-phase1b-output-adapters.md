# KojiGeometry Phase 1B — outbound format adapters

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `KojiGeometryCollection` the full set of 1→M outbound format adapters (coordinate arrays, structs, Poracle, Text, SQL) — extracted directly from `geo::Geometry` + `KojiMeta`, parity-verified against the existing `To*` matrix while it is still alive to serve as the oracle.

**Architecture:** Additive inherent methods on `KojiGeometryCollection` (and the element where a singular form is needed). Each adapter is **matrix-independent** — it reads coordinates from `geo::Geometry` and metadata from `KojiMeta`, never delegating to `FeatureCollection::to_X` (those cells of the N×M matrix die in Phase 2). Correctness is pinned by a **parity test** against the oracle `geojson::FeatureCollection::from(&coll).to_X()`, which is exact today and lets the implementer TDD the direct extraction to a byte-match.

**Tech Stack:** Rust (edition 2024), `geo`/`geo-types`, `geojson` 0.24, `serde`/`serde_json`. Builds on the Phase 1 types (`KojiGeometry`, `KojiGeometryCollection`, `KojiMeta`, `Mode`) and the Phase 1 outbound `From<&KojiGeometryCollection> for geojson::FeatureCollection`.

---

## Scope & phasing notes

- **Additive only.** No deletions, no rewiring. `response.rs` still calls `FeatureCollection::to_X` — these new adapters are unused until the Phase 2 rewire. The existing `To*` traits / impls stay untouched (they are the parity oracle).
- **Matrix-independent by construction.** Adapters must NOT call `FeatureCollection::to_single_vec` / `to_poracle_vec` / etc. internally. They extract from `geo::Geometry` + `KojiMeta`. (The parity *test* uses the oracle; the *implementation* must not.)
- **s2 bridge is NOT in this plan** — it moves to Phase 2 (built with its algorithms consumer). Phase 1B is the outbound adapters only.
- **Parity oracle:** for a `KojiGeometryCollection` value `c`, the oracle for format `X` is `geojson::FeatureCollection::from(&c).to_X(..)`. This composition is exact today (Phase 1 outbound + existing `To*`). Each task asserts the direct adapter equals it.
- **Reference:** spec `docs/superpowers/specs/2026-06-13-koji-geometry-universal-type-design.md` (§4.1, §6, §10). Existing format impls to read while matching: `crates/koji-core/src/geometry/{single_vec.rs, multi_vec.rs, single_struct.rs, multi_struct.rs, poracle.rs, text.rs, collection.rs}` and the `To*` trait defs in `geometry/mod.rs`.

## File structure

| File | Responsibility | Action |
|------|----------------|--------|
| `crates/koji-core/src/geometry/koji_output.rs` | all `KojiGeometryCollection` outbound adapters + their tests | Create |
| `crates/koji-core/src/geometry/mod.rs` | `mod koji_output;` | Modify |

`koji_output.rs` holds inherent `impl KojiGeometryCollection` methods. If it crosses ~400 lines, split per-format into a `koji_output/` directory in a follow-up — not required here.

Coordinate convention note: match whatever the oracle emits (the existing `PointArray` is `[f64; 2]`; confirm lng/lat vs lat/lng ordering against the oracle in Task 1 — do not assume).

---

### Task 1: coordinate arrays — `to_single_vec` / `to_multi_vec`

**Files:**
- Create: `crates/koji-core/src/geometry/koji_output.rs`
- Modify: `crates/koji-core/src/geometry/mod.rs` (`mod koji_output;`)

- [ ] **Step 1: Write the failing parity test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KojiGeometry, KojiGeometryCollection, KojiMeta, Mode};
    use crate::geometry::{ToSingleVec, ToMultiVec}; // the existing oracle traits
    use geo::{LineString, MultiPoint, Point, Polygon, coord};

    /// A representative collection: one polygon + one multipoint, with metadata.
    fn sample() -> KojiGeometryCollection {
        let poly = Polygon::new(
            LineString::from(vec![
                coord! {x:0.0,y:0.0}, coord! {x:2.0,y:0.0},
                coord! {x:2.0,y:2.0}, coord! {x:0.0,y:0.0},
            ]),
            vec![],
        );
        let mp = MultiPoint::from(vec![Point::new(5.0, 6.0), Point::new(7.0, 8.0)]);
        KojiGeometryCollection::new(vec![
            KojiGeometry::new(poly).with_meta(KojiMeta { name: Some("a".into()), mode: Mode::Fort, ..Default::default() }),
            KojiGeometry::new(mp).with_meta(KojiMeta { name: Some("b".into()), ..Default::default() }),
        ])
    }

    /// Oracle: the existing matrix path, exact today.
    fn fc(c: &KojiGeometryCollection) -> geojson::FeatureCollection {
        geojson::FeatureCollection::from(c)
    }

    #[test]
    fn single_vec_matches_oracle() {
        let c = sample();
        assert_eq!(c.to_single_vec(), fc(&c).to_single_vec());
    }

    #[test]
    fn multi_vec_matches_oracle() {
        let c = sample();
        assert_eq!(c.to_multi_vec(), fc(&c).to_multi_vec());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_output::tests::single_vec_matches_oracle koji_output::tests::multi_vec_matches_oracle`
Expected: FAIL — methods `to_single_vec`/`to_multi_vec` not found on `KojiGeometryCollection`.

- [ ] **Step 3: Implement directly from `geo::Geometry`**

Create `koji_output.rs` with `impl KojiGeometryCollection { pub fn to_single_vec(&self) -> SingleVec; pub fn to_multi_vec(&self) -> MultiVec; }`. Extract coordinates from each item's `geo::Geometry` (`Polygon` → exterior ring coords matching the oracle's ring handling; `MultiPoint`/`Point`/`LineString` → their coords; `MultiPolygon` → per-polygon). `to_single_vec` flattens all items' coords into one `Vec<[f64; 2]>`; `to_multi_vec` keeps one inner vec per ring/geometry — match the oracle's grouping exactly.

Do NOT call `FeatureCollection::to_single_vec`. Read `crates/koji-core/src/geometry/single_vec.rs`, `multi_vec.rs`, and `collection.rs`'s `ToSingleVec`/`ToMultiVec` impls to match coordinate **order**, **lng/lat convention**, and **ring closure** (whether the closing point is included). Iterate against the parity test until byte-equal.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_output::tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_output.rs crates/koji-core/src/geometry/mod.rs
git commit -m "feat(geometry): KojiGeometryCollection to_single_vec/to_multi_vec (direct, parity-tested)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: coordinate structs — `to_single_struct` / `to_multi_struct`

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_output.rs` (append methods + tests)

- [ ] **Step 1: Write the failing parity test**

```rust
#[test]
fn single_struct_matches_oracle() {
    use crate::geometry::ToSingleStruct;
    let c = sample();
    assert_eq!(c.to_single_struct(), fc(&c).to_single_struct());
}

#[test]
fn multi_struct_matches_oracle() {
    use crate::geometry::ToMultiStruct;
    let c = sample();
    assert_eq!(c.to_multi_struct(), fc(&c).to_multi_struct());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_output::tests::single_struct_matches_oracle koji_output::tests::multi_struct_matches_oracle`
Expected: FAIL — methods not found.

- [ ] **Step 3: Implement**

Add `to_single_struct(&self) -> SingleStruct` and `to_multi_struct(&self) -> MultiStruct`. `PointStruct { lat, lon }` — reuse `to_single_vec`/`to_multi_vec` from Task 1 and map each `[f64; 2]` to a `PointStruct` with the **same lat/lon assignment the oracle uses** (confirm field order against `single_struct.rs` / the `From<PointArray> for PointStruct` impl). This keeps the struct forms layered over the (already parity-proven) vec forms.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_output::tests`
Expected: PASS (4 tests total).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_output.rs
git commit -m "feat(geometry): KojiGeometryCollection to_single_struct/to_multi_struct

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: `to_text` (covers Text + AltText)

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_output.rs`

Note: `response.rs` calls `to_text(",", "\n", true)` (Text) and `to_text(" ", ",", false)` (AltText) — one method, two parameterizations. Test both.

- [ ] **Step 1: Write the failing parity test**

```rust
#[test]
fn text_matches_oracle_both_param_sets() {
    use crate::geometry::ToText;
    let c = sample();
    assert_eq!(c.to_text(",", "\n", true), fc(&c).to_text(",", "\n", true));
    assert_eq!(c.to_text(" ", ",", false), fc(&c).to_text(" ", ",", false));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_output::tests::text_matches_oracle_both_param_sets`
Expected: FAIL — `to_text` not found.

- [ ] **Step 3: Implement**

Add `to_text(&self, sep_1: &str, sep_2: &str, poly_sep: bool) -> String`. Build it over `to_multi_vec` (Task 1) and reproduce the existing formatting (coordinate precision, `sep_1` between lat/lon, `sep_2` between points, `poly_sep` behavior between polygons) by reading `crates/koji-core/src/geometry/text.rs` (`ToText`). Match precision exactly — coordinate-to-string formatting is the common parity pitfall.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_output::tests`
Expected: PASS (5 tests total).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_output.rs
git commit -m "feat(geometry): KojiGeometryCollection to_text (Text + AltText parity)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: `to_sql`

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_output.rs`

- [ ] **Step 1: Write the failing parity test**

```rust
#[test]
fn sql_matches_oracle() {
    use crate::geometry::ToSql;
    let c = sample();
    assert_eq!(c.to_sql(), fc(&c).to_sql());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_output::tests::sql_matches_oracle`
Expected: FAIL — `to_sql` not found.

- [ ] **Step 3: Implement**

Add `to_sql(&self) -> String`. Reproduce the existing SQL string format from `crates/koji-core/src/geometry/collection.rs` (the `ToSql` impl) over the coords from Task 1. Match delimiters, parentheses, and precision exactly against the oracle.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_output::tests`
Expected: PASS (6 tests total).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_output.rs
git commit -m "feat(geometry): KojiGeometryCollection to_sql

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: `to_poracle_vec` (metadata + coords)

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_output.rs`

Note: `Poracle { id, name, color, group, description, user_selectable, display_in_matches, path: Option<SingleVec>, multipath: Option<MultiVec> }`. One `Poracle` per item; metadata comes from `KojiMeta`, `path`/`multipath` from the geometry. The oracle reads metadata from the geojson `properties` (which Phase 1 outbound populated from `KojiMeta`), so the keys must line up.

- [ ] **Step 1: Write the failing parity test**

```rust
#[test]
fn poracle_vec_matches_oracle() {
    use crate::geometry::ToPoracleVec;
    let c = sample();
    assert_eq!(c.to_poracle_vec(), fc(&c).to_poracle_vec());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_output::tests::poracle_vec_matches_oracle`
Expected: FAIL — `to_poracle_vec` not found.

- [ ] **Step 3: Implement**

Add `to_poracle_vec(&self) -> Vec<Poracle>` — one `Poracle` per item. Pull `id`/`name` (and any other Poracle metadata fields) from each item's `KojiMeta` (typed fields first, then `extra`), and set `path` for single-ring/line geometries vs `multipath` for multi geometries, matching how the oracle decides (read `crates/koji-core/src/geometry/poracle.rs` and `collection.rs`'s `ToPoracleVec`). Reproduce the same `path` vs `multipath` selection and the same metadata-key mapping. (`PoracleSingle` in `response.rs` is just `.first()` of this — no separate method needed.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_output::tests`
Expected: PASS (7 tests total).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_output.rs
git commit -m "feat(geometry): KojiGeometryCollection to_poracle_vec (meta + coords, parity)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Phase 1B verification gate

**Files:** none (verification only)

- [ ] **Step 1: Full workspace build + clippy + fmt + tests**

Run (single batch, background if slow):
- `cargo build --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test --workspace`

Expected: all green. New adapters compile, all parity tests pass, clippy clean (the pre-existing debt was fixed in `8ead6ab`), fmt clean.

- [ ] **Step 2: Confirm additivity + matrix-independence**

Run: `git grep -n "FeatureCollection::from\|\.to_single_vec()\|\.to_poracle_vec()\|\.to_text(\|\.to_sql()" crates/koji-core/src/geometry/koji_output.rs`
Expected: matches appear ONLY inside `#[cfg(test)]` (the oracle). If any appear in non-test adapter code, the adapter is routing through the matrix — fix it to extract directly before this gate passes.

- [ ] **Step 3: Commit (if fmt/clippy fixups were needed)**

```bash
git add -A && git commit -m "chore(geometry): Phase 1B verification fixups

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>" || echo "nothing to commit"
```

---

## Self-review

**Spec coverage (Phase 1B slice):**
- §10 "demoted to output-only adapter (1→M)": Poracle (T5), SingleVec/MultiVec (T1), SingleStruct/MultiStruct (T2), Text (T3), SQL (T4). ✓ All present, all sourced from `KojiGeometry`, none routed through the dying matrix.
- §6 "Outbound (1→M)": the adapters hang off `KojiGeometryCollection`. ✓
- §13 parity safety (golden snapshots of every retained `?rt=` format): each task IS a parity assertion against the live oracle. ✓
- **Deferred (flagged):** s2 bridge centralization → Phase 2 (built with its algorithms consumer). geojson `Feature`/`FeatureCollection`/`GeometryCollection`/`Geometry` outbound already shipped in Phase 1 — not repeated here.

**Placeholder scan:** no TBD/TODO. Implementation steps intentionally specify the direct-extraction approach + the parity contract + the exact oracle and source files to match, rather than pre-baking coordinate-ordering/precision code that must be matched empirically (the parity test is the precise spec; the oracle is exact). Test code is complete.

**Type consistency:** `sample()` and `fc()` helpers shared across all tasks (defined in Task 1, reused — same `koji_output::tests` module). Method names (`to_single_vec`, `to_multi_vec`, `to_single_struct`, `to_multi_struct`, `to_text`, `to_sql`, `to_poracle_vec`) match the oracle trait methods exactly so the parity asserts compile against both sides. Return types match the oracle (`SingleVec`, `MultiVec`, `SingleStruct`, `MultiStruct`, `String`, `Vec<Poracle>`).
