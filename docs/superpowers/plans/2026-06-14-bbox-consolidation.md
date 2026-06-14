# Bbox Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse the nine in-house bounding-box representations into one Koji-owned `KojiBbox` (lat-first, serializable) layered over `geo::Rect<f64>`, fixing the live geojson-order bug along the way.

**Architecture:** `geo::Rect<f64>` stays the geometry-layer internal canonical (x=lon, y=lat); a new `KojiBbox` serves the compute/boundary layer with explicit named lat/lon fields. A single `from_rect`/`to_rect` bridge connects them. The cached `KojiGeometryCollection.bbox` field becomes an on-demand accessor (no stale state). `BoundsArg` embeds `KojiBbox` via `#[serde(flatten)]` (wire-identical). The duplicated `BBox` / `LatLonBBox` / `GetBbox` / `bbox_of` paths fold into `KojiBbox`.

**Tech Stack:** Rust (edition 2024), `geo` / `geojson` crates, `serde`, sea-orm; workspace crates `koji-core`, `algorithms`, `koji-service`, `koji-scanner`.

**Spec:** [`docs/superpowers/specs/2026-06-14-bbox-consolidation-design.md`](../specs/2026-06-14-bbox-consolidation-design.md)

---

## Deviations from spec (flagged for reviewer)

1. **fastest `BoundingBox` (#3) → `geo::Rect` is DEFERRED, not done.** The spec listed converting it. Planning revealed it is projected-`Plane` x/y space (not lat/lon), fully module-private (so it never participated in the cross-crate twin-`BBox` name collision), and threaded through a hot merge loop (`fastest.rs:207-234`). Converting it is pure churn with parity risk and **zero unification value** — it would become a lone `geo::Rect` in x/y space, joining nothing. Deleting koji-core `BBox` (Task 6) already fully resolves the name collision. Left as a documented module-local type. If the reviewer wants it done anyway, it is a self-contained follow-up.
2. **`KojiBbox::union` and `::contains` (listed in spec) are OMITTED (YAGNI).** No caller exists for either. Add when first needed. Every method in this plan has a real call site.

These are surfaced here and in the handoff so they can be vetoed during plan review.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `crates/koji-core/src/geometry/koji_bbox.rs` | The `KojiBbox` type + bridge + projections | **Create** |
| `crates/koji-core/src/geometry/mod.rs` | Declare/export `koji_bbox`; later delete `BBox` + `GetBbox` | Modify |
| `crates/koji-core/src/geometry/koji_geometry.rs` | On-demand `bbox()`/`koji_bbox()`; drop stored field | Modify |
| `crates/koji-core/src/geometry/koji_output.rs` | Add `geojson_bbox()`; route `to_sql` through `KojiBbox`; later delete `bbox_of`/`trim6` | Modify |
| `crates/koji-core/src/geometry/single_vec.rs` | Later: delete `GetBbox for SingleVec` | Modify |
| `crates/koji-core/src/geometry/feature.rs` | Later: delete `GetBbox for Feature` | Modify |
| `crates/koji-core/src/geometry/geometry.rs` | Route member-bbox through helper; later delete `GetBbox for Geometry` | Modify |
| `crates/koji-core/src/query_args.rs` | `BoundsArg` embeds `KojiBbox` (flatten) | Modify |
| `crates/koji-core/src/util.rs` | Route `sql_raw`/`sql_raw_bbox` through `KojiBbox`; drop `GetBbox` import | Modify |
| `crates/algorithms/src/clustering/candidates.rs` | Delete local `BBox`; use `KojiBbox` | Modify |
| `crates/algorithms/src/clustering/partition.rs` | Delete `LatLonBBox`; use `KojiBbox` | Modify |
| `crates/algorithms/src/clustering/greedy.rs` | Route `get_bbox` calls through `KojiBbox` | Modify |
| `crates/koji-service/src/utils/mod.rs` | Route `load_collection`/`create_or_find_collection` through `KojiBbox` (bug fix) | Modify |
| `crates/koji-service/src/public/{v1/s2.rs,v2/geo.rs}` | `bounds.<f>` → `bounds.bbox.<f>` | Modify |
| `crates/koji-scanner/src/entities/{station,spawnpoint,gym,pokestop}.rs` | `payload.<f>` → `payload.bbox.<f>` | Modify |

---

## Task 1: Create the `KojiBbox` type

**Files:**
- Create: `crates/koji-core/src/geometry/koji_bbox.rs`
- Modify: `crates/koji-core/src/geometry/mod.rs`

- [ ] **Step 1: Declare and export the module**

In `crates/koji-core/src/geometry/mod.rs`, add the module declaration (alphabetical, after `koji_geometry`):

```rust
mod koji_bbox;
```

And add to the `pub use` block (near the other `pub use koji_*` lines, e.g. after line 20 `pub use koji_meta::KojiMeta;`):

```rust
pub use koji_bbox::KojiBbox;
```

(`crates/koji-core/src/lib.rs:39` already does `pub use geometry::*;`, so `koji_core::KojiBbox` resolves crate-wide once this lands.)

- [ ] **Step 2: Write the failing tests**

Create `crates/koji-core/src/geometry/koji_bbox.rs` with ONLY the test module first (the type comes in Step 4):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use geo::{coord, Rect};

    #[test]
    fn from_points_computes_lat_lon_extremes() {
        // points are [lat, lon]
        let pts = [[1.0, 2.0], [3.0, -4.0], [-5.0, 6.0]];
        let b = KojiBbox::from_points(&pts).unwrap();
        assert_eq!(b.min_lat, -5.0);
        assert_eq!(b.max_lat, 3.0);
        assert_eq!(b.min_lon, -4.0);
        assert_eq!(b.max_lon, 6.0);
    }

    #[test]
    fn from_points_empty_is_none() {
        let empty: [[f64; 2]; 0] = [];
        assert_eq!(KojiBbox::from_points(&empty), None);
    }

    #[test]
    fn rect_bridge_round_trips() {
        let b = KojiBbox { min_lat: 1.0, min_lon: 2.0, max_lat: 3.0, max_lon: 4.0 };
        let r: Rect<f64> = b.to_rect();
        // geo::Rect is x=lon, y=lat.
        assert_eq!(r.min(), coord! { x: 2.0, y: 1.0 });
        assert_eq!(r.max(), coord! { x: 4.0, y: 3.0 });
        assert_eq!(KojiBbox::from_rect(r), b);
    }

    /// THE bug guard: geojson bbox order is [min_lon, min_lat, max_lon, max_lat].
    /// The deleted `BBox::get_geojson_bbox` emitted [min_lon, max_lon, min_lat, max_lat].
    #[test]
    fn to_geojson_bbox_is_lon_first_min_max_interleaved() {
        let b = KojiBbox { min_lat: 1.0, min_lon: 2.0, max_lat: 3.0, max_lon: 4.0 };
        assert_eq!(b.to_geojson_bbox(), [2.0, 1.0, 4.0, 3.0]);
        assert_eq!(b.to_geojson_bbox_vec(), vec![2.0, 1.0, 4.0, 3.0]);
        // The buggy interleave must NOT appear.
        assert_ne!(b.to_geojson_bbox(), [2.0, 4.0, 1.0, 3.0]);
    }

    #[test]
    fn expand_grows_every_side() {
        let b = KojiBbox { min_lat: 0.0, min_lon: 0.0, max_lat: 10.0, max_lon: 10.0 }.expand(1.0);
        assert_eq!(b, KojiBbox { min_lat: -1.0, min_lon: -1.0, max_lat: 11.0, max_lon: 11.0 });
    }

    #[test]
    fn center_lat_is_midpoint() {
        let b = KojiBbox { min_lat: 0.0, min_lon: 0.0, max_lat: 8.0, max_lon: 0.0 };
        assert_eq!(b.center_lat(), 4.0);
    }

    #[test]
    fn trim_rounds_to_precision() {
        let b = KojiBbox { min_lat: 1.23456789, min_lon: 2.0, max_lat: 3.0, max_lon: 4.0 }.trim(6);
        assert_eq!(b.min_lat, 1.234568);
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p koji-core --lib koji_bbox`
Expected: FAIL to compile — `cannot find type KojiBbox` / `KojiBbox` not defined.

- [ ] **Step 4: Write the type**

Prepend the implementation above the test module in `crates/koji-core/src/geometry/koji_bbox.rs`:

```rust
//! `KojiBbox` — the Koji-owned, serializable bounding box in explicit lat/lon
//! degrees. It serves the compute and boundary layers; the geometry layer's
//! internal canonical is `geo::Rect<f64>` (x=lon, y=lat), and `from_rect`/
//! `to_rect` is the single bridge between the two. Fields are named, never
//! positional, to retire the coordinate-order ambiguity the old
//! `BBox`/`BoundingBox`/`LatLonBBox` zoo bred.

use geo::{coord, Rect};
use serde::{Deserialize, Serialize};

use crate::TrimPrecision;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KojiBbox {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl KojiBbox {
    /// Bridge in from the geometry-layer canonical (`geo::Rect` is x=lon, y=lat).
    pub fn from_rect(r: Rect<f64>) -> Self {
        Self {
            min_lat: r.min().y,
            min_lon: r.min().x,
            max_lat: r.max().y,
            max_lon: r.max().x,
        }
    }

    /// Bridge out to the geometry-layer canonical.
    pub fn to_rect(self) -> Rect<f64> {
        Rect::new(
            coord! { x: self.min_lon, y: self.min_lat },
            coord! { x: self.max_lon, y: self.max_lat },
        )
    }

    /// Build from the internal `[lat, lon]` compute currency (`SingleVec` /
    /// `PointArray`). `None` for an empty set. Full precision — apply [`KojiBbox::trim`]
    /// for the 6-decimal geojson/SQL output convention.
    pub fn from_points(points: &[[f64; 2]]) -> Option<Self> {
        if points.is_empty() {
            return None;
        }
        let mut b = Self {
            min_lat: f64::INFINITY,
            min_lon: f64::INFINITY,
            max_lat: f64::NEG_INFINITY,
            max_lon: f64::NEG_INFINITY,
        };
        for &[lat, lon] in points {
            b.min_lat = b.min_lat.min(lat);
            b.max_lat = b.max_lat.max(lat);
            b.min_lon = b.min_lon.min(lon);
            b.max_lon = b.max_lon.max(lon);
        }
        Some(b)
    }

    /// Grow the box outward by `deg` degrees on every side.
    pub fn expand(self, deg: f64) -> Self {
        Self {
            min_lat: self.min_lat - deg,
            min_lon: self.min_lon - deg,
            max_lat: self.max_lat + deg,
            max_lon: self.max_lon + deg,
        }
    }

    /// Latitude midpoint.
    pub fn center_lat(&self) -> f64 {
        0.5 * (self.min_lat + self.max_lat)
    }

    /// Round all four corners to `precision` decimals (the geojson/SQL output
    /// convention — matches `TrimPrecision for f64`).
    pub fn trim(self, precision: u32) -> Self {
        Self {
            min_lat: self.min_lat.trim_precision(precision),
            min_lon: self.min_lon.trim_precision(precision),
            max_lat: self.max_lat.trim_precision(precision),
            max_lon: self.max_lon.trim_precision(precision),
        }
    }

    /// GeoJSON-standard bbox array `[min_lon, min_lat, max_lon, max_lat]`. Correct
    /// order — replaces the buggy `BBox::get_geojson_bbox`.
    pub fn to_geojson_bbox(self) -> [f64; 4] {
        [self.min_lon, self.min_lat, self.max_lon, self.max_lat]
    }

    /// GeoJSON-standard bbox as a `Vec<f64>` (the `geojson::Bbox` slot shape).
    pub fn to_geojson_bbox_vec(self) -> Vec<f64> {
        self.to_geojson_bbox().to_vec()
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p koji-core --lib koji_bbox`
Expected: PASS (7 tests).

- [ ] **Step 6: Commit**

```bash
git add crates/koji-core/src/geometry/koji_bbox.rs crates/koji-core/src/geometry/mod.rs
git commit -m "feat(geometry): add KojiBbox — lat-first serializable bbox + Rect bridge

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: On-demand bbox accessors; drop the stored field

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_geometry.rs`
- Modify: `crates/koji-core/src/geometry/koji_output.rs` (add `geojson_bbox`)

- [ ] **Step 1: Update the failing tests**

In `crates/koji-core/src/geometry/koji_geometry.rs`, change the two tests that read the `.bbox` field to call the new method, and add a `koji_bbox` test. Replace the body of `collection_bbox_unions_items` (currently asserts `c.bbox`) and `empty_collection_has_no_bbox`:

```rust
    #[test]
    fn collection_bbox_unions_items() {
        let c = KojiGeometryCollection::new(vec![
            KojiGeometry::new(Point::new(0.0, 0.0)),
            KojiGeometry::new(Point::new(10.0, 5.0)),
        ]);
        assert_eq!(c.items.len(), 2);
        assert_eq!(
            c.bbox(),
            Some(Rect::new(coord! {x:0.0,y:0.0}, coord! {x:10.0,y:5.0}))
        );
    }

    #[test]
    fn collection_koji_bbox_is_lat_lon() {
        // Points are geo (x=lon, y=lat): (lon=0,lat=0) and (lon=10,lat=5).
        let c = KojiGeometryCollection::new(vec![
            KojiGeometry::new(Point::new(0.0, 0.0)),
            KojiGeometry::new(Point::new(10.0, 5.0)),
        ]);
        assert_eq!(
            c.koji_bbox(),
            Some(crate::KojiBbox { min_lat: 0.0, min_lon: 0.0, max_lat: 5.0, max_lon: 10.0 })
        );
    }

    #[test]
    fn empty_collection_has_no_bbox() {
        assert_eq!(KojiGeometryCollection::default().bbox(), None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p koji-core --lib koji_geometry`
Expected: FAIL to compile — `no method named bbox`/`koji_bbox` found; `field bbox` removed references (after Step 3) — the point is these reference APIs not yet present.

- [ ] **Step 3: Drop the field, simplify the constructor, add accessors**

In `crates/koji-core/src/geometry/koji_geometry.rs`:

Change the struct (remove the `bbox` field):

```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KojiGeometryCollection {
    pub items: Vec<KojiGeometry>,
}
```

Change the constructor (remove the bbox computation):

```rust
    pub fn new(items: Vec<KojiGeometry>) -> Self {
        Self { items }
    }
```

Add `koji_bbox` to the `impl KojiGeometry` block (after the existing `bbox` method, ~line 30):

```rust
    /// The element's bounds as a lat/lon [`KojiBbox`] (the boundary view of
    /// [`KojiGeometry::bbox`]).
    pub fn koji_bbox(&self) -> Option<super::KojiBbox> {
        self.bbox().map(super::KojiBbox::from_rect)
    }
```

Add on-demand `bbox` + `koji_bbox` to the `impl KojiGeometryCollection` block (e.g. right after `new`):

```rust
    /// Smallest `geo::Rect` covering every item, computed on demand (no stored
    /// state). `None` if the collection has no boundable geometry.
    pub fn bbox(&self) -> Option<Rect<f64>> {
        self.items
            .iter()
            .filter_map(KojiGeometry::bbox)
            .reduce(union_rect)
    }

    /// The collection bounds as a lat/lon [`KojiBbox`].
    pub fn koji_bbox(&self) -> Option<super::KojiBbox> {
        self.bbox().map(super::KojiBbox::from_rect)
    }
```

The `simplify` method calls `Self::new(...)` and is unaffected (it no longer needs to "rebuild the bbox" — fix the doc comment): change the doc line `/// [`KojiGeometry::simplify`]). Rebuilds the collection bbox.` to `/// [`KojiGeometry::simplify`]).`. `union_rect` (line ~102) stays — it is now the `bbox()` fold helper.

- [ ] **Step 4: Add the `geojson_bbox` projection helper**

This is the trimmed geojson-member projection that replaces the `GetBbox` delegation in Task 5. In `crates/koji-core/src/geometry/koji_output.rs`, inside the existing `impl KojiGeometryCollection` block (the one starting ~line 139 with `to_single_vec`), add:

```rust
    /// `[min_lon, min_lat, max_lon, max_lat]` (trimmed to 6 decimals) over every
    /// item's points — the geojson `bbox` member projection. Replaces the deleted
    /// `GetBbox` delegation (Feature/Geometry → `to_single_vec` → `get_bbox`).
    pub fn geojson_bbox(&self) -> Option<Vec<f64>> {
        super::KojiBbox::from_points(&self.to_single_vec()).map(|b| b.trim(6).to_geojson_bbox_vec())
    }
```

(`SingleVec` is `Vec<[f64; 2]>`, so `&self.to_single_vec()` coerces to `&[[f64; 2]]`.)

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p koji-core --lib`
Expected: PASS. If any OTHER test in koji-core referenced `.bbox` as a field, update it to `.bbox()` (there should be none beyond the two changed here — verify with `rg -n '\.bbox\b' crates/koji-core/src` showing only method calls `.bbox()` and the geojson `Geometry.bbox` member).

- [ ] **Step 6: Commit**

```bash
git add crates/koji-core/src/geometry/koji_geometry.rs crates/koji-core/src/geometry/koji_output.rs
git commit -m "refactor(geometry): KojiGeometryCollection bbox on-demand, drop stored field

Add koji_bbox() to item + collection; add geojson_bbox() projection helper.
Eliminates the stale-able cached field.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: `BoundsArg` embeds `KojiBbox` via `#[serde(flatten)]`

**Files:**
- Modify: `crates/koji-core/src/query_args.rs`
- Modify: `crates/koji-service/src/public/v2/geo.rs`, `crates/koji-service/src/public/v1/s2.rs`
- Modify: `crates/koji-scanner/src/entities/{station,spawnpoint,gym,pokestop}.rs`

- [ ] **Step 1: Write the failing wire-parity test**

In `crates/koji-core/src/query_args.rs`, add a test module at the end of the file (or extend an existing one):

```rust
#[cfg(test)]
mod bounds_arg_tests {
    use super::*;

    #[test]
    fn bounds_arg_wire_is_flat_and_identical() {
        let json = r#"{"min_lat":1.0,"min_lon":2.0,"max_lat":3.0,"max_lon":4.0,"last_seen":5}"#;
        let parsed: BoundsArg = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.bbox.min_lat, 1.0);
        assert_eq!(parsed.bbox.min_lon, 2.0);
        assert_eq!(parsed.bbox.max_lat, 3.0);
        assert_eq!(parsed.bbox.max_lon, 4.0);
        assert_eq!(parsed.last_seen, Some(5));
        // Re-serialization stays flat (flatten preserves the wire shape).
        let back = serde_json::to_value(&parsed).unwrap();
        assert_eq!(back["min_lat"], 1.0);
        assert_eq!(back["max_lon"], 4.0);
        assert_eq!(back["last_seen"], 5);
        assert!(back.get("bbox").is_none(), "bbox must be flattened, not nested");
    }
}
```

(If `serde_json` is not already a dev-dependency of koji-core, add it under `[dev-dependencies]` in `crates/koji-core/Cargo.toml`: `serde_json = "1"` — check first with `rg serde_json crates/koji-core/Cargo.toml`.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core --lib bounds_arg`
Expected: FAIL to compile — `no field bbox on BoundsArg`.

- [ ] **Step 3: Embed `KojiBbox` in `BoundsArg`**

In `crates/koji-core/src/query_args.rs`, change the struct (currently lines 120-129). Ensure `KojiBbox` is in scope — add `use crate::geometry::KojiBbox;` near the top imports if not already present:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundsArg {
    #[serde(flatten)]
    pub bbox: KojiBbox,
    pub last_seen: Option<u32>,
    pub ids: Option<Vec<String>>,
    pub tth: Option<SpawnpointTth>,
}
```

- [ ] **Step 4: Update the field-access call sites**

Each `<var>.min_lat/min_lon/max_lat/max_lon` becomes `<var>.bbox.<field>`. The `last_seen`/`ids`/`tth` accesses are unchanged (they stay top-level).

`crates/koji-service/src/public/v2/geo.rs:146-149` — change `bounds.min_lat, bounds.min_lon, bounds.max_lat, bounds.max_lon` to `bounds.bbox.min_lat, bounds.bbox.min_lon, bounds.bbox.max_lat, bounds.bbox.max_lon`.

`crates/koji-service/src/public/v1/s2.rs:98-101` — same change (`bounds.min_lat` → `bounds.bbox.min_lat`, etc.).

`crates/koji-scanner/src/entities/station.rs:85-86`, `spawnpoint.rs:64-65`, `gym.rs:87-88`, `pokestop.rs:95-96` — change `payload.min_lat`/`payload.max_lat`/`payload.min_lon`/`payload.max_lon` to `payload.bbox.min_lat`/`.bbox.max_lat`/`.bbox.min_lon`/`.bbox.max_lon`. Leave `payload.last_seen` and `payload.tth` unchanged.

- [ ] **Step 5: Verify OpenAPI is unchanged**

`#[serde(flatten)]` keeps the JSON shape identical, so the hand-maintained schema needs no edit. Confirm the `BoundsArg` schema is still flat:

Run: `rg -n -A8 'BoundsArg:' crates/koji-service/openapi.yaml`
Expected: properties `min_lat, min_lon, max_lat, max_lon, last_seen, ...` at the top level (no `bbox` nesting). No change required.

- [ ] **Step 6: Run tests + typecheck to verify**

Run these in parallel:
```bash
cargo test -p koji-core --lib bounds_arg
cargo build -p koji-service -p koji-scanner
```
Expected: test PASS; both crates compile.

- [ ] **Step 7: Commit**

```bash
git add crates/koji-core/src/query_args.rs crates/koji-service/src/public crates/koji-scanner/src/entities
git commit -m "refactor(query): BoundsArg embeds KojiBbox via serde(flatten)

Wire JSON unchanged; callers read .bbox.<field>. Folds the 5th hand-rolled
bbox shape into the canonical type.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: Fold the algorithms lat/lon bbox structs into `KojiBbox`

**Files:**
- Modify: `crates/algorithms/src/clustering/candidates.rs` (delete `BBox`)
- Modify: `crates/algorithms/src/clustering/partition.rs` (delete `LatLonBBox`)

- [ ] **Step 1: Migrate `candidates.rs`**

Delete the local `BBox` struct + impl (lines 10-56: the `/// Represents a bounding box...` doc, the `struct BBox { ... }`, and the entire `impl BBox { fn from_points ... fn expand ... }`). Add `KojiBbox` to the koji-core import (line 3):

```rust
use koji_core::{KojiBbox, Precision, SingleVec};
```

In `generate_cluster_candidates_grid` (line 83), change `BBox::from_points(points)` to `KojiBbox::from_points(points)`:

```rust
    let bbox = match KojiBbox::from_points(points) {
        Some(b) => b.expand(cluster_radius_deg),
        None => return Vec::new(),
    };
```

The downstream field reads (`bbox.max_lat`, `bbox.min_lat`, `bbox.max_lon`, `bbox.min_lon` at lines 89-119) are unchanged — `KojiBbox` uses the identical field names. `b.expand(...)` is unchanged (`KojiBbox::expand` has the same signature and semantics).

- [ ] **Step 2: Build candidates to verify**

Run: `cargo build -p algorithms`
Expected: compiles (partition still has `LatLonBBox` — that is Step 3).

- [ ] **Step 3: Migrate `partition.rs`**

Delete the `LatLonBBox` struct + impl (lines 157-178: the `#[derive(...)] pub(crate) struct LatLonBBox { ... }` and `impl LatLonBBox { fn center_lat ... fn expand ... }`). Add `KojiBbox` to the file's koji-core import (find the existing `use koji_core::...` line and add `KojiBbox`).

Rewrite `cell_bbox_lat_lon` (lines 180-199) to build and return a `KojiBbox`:

```rust
pub(crate) fn cell_bbox_lat_lon(cell: CellID) -> KojiBbox {
    let c = Cell::from(&cell);
    // Use vertex extremes; avoids rect_bound() API churn between s2 versions.
    let mut bb = KojiBbox {
        min_lat: Precision::INFINITY,
        max_lat: Precision::NEG_INFINITY,
        min_lon: Precision::INFINITY,
        max_lon: Precision::NEG_INFINITY,
    };
    for i in 0..4 {
        let v = c.vertex(i);
        let lat = v.latitude().deg();
        let lon = v.longitude().deg();
        bb.min_lat = bb.min_lat.min(lat);
        bb.max_lat = bb.max_lat.max(lat);
        bb.min_lon = bb.min_lon.min(lon);
        bb.max_lon = bb.max_lon.max(lon);
    }
    bb
}
```

The consumers in `gather_halo` (lines 221-228: `bbox.center_lat()`, `bbox.expand(...)`, `expanded.min_lat`/`min_lon`/`max_lat`/`max_lon`) and `crucible/refine.rs:1296-1298` (`bbox.center_lat()`, `bbox.expand(margin)`) are unchanged — `KojiBbox` provides `center_lat()` and `expand()` with identical signatures and field names. The test at `partition.rs:467-470` (`cell_bbox_lat_lon_is_finite_and_ordered`) and `545`, and `refine.rs:1289` (`use ...partition::cell_bbox_lat_lon`) keep working; if any test referenced the type name `LatLonBBox` directly, change it to `KojiBbox` (check: `rg -n 'LatLonBBox' crates/algorithms` must return nothing after this step).

- [ ] **Step 4: Build + test algorithms to verify parity**

Run in parallel:
```bash
cargo build -p algorithms
rg -n 'LatLonBBox|struct BBox' crates/algorithms
```
Then:
```bash
cargo test -p algorithms
```
Expected: build OK; the `rg` returns nothing (both structs gone); `cargo test -p algorithms` PASS — the clustering golden/parity tests confirm the swap is result-preserving.

- [ ] **Step 5: Commit**

```bash
git add crates/algorithms/src/clustering/candidates.rs crates/algorithms/src/clustering/partition.rs
git commit -m "refactor(algorithms): fold candidates BBox + LatLonBBox into KojiBbox

Both were lat-first structs identical to KojiBbox. cell_bbox_lat_lon now
returns KojiBbox; center_lat/expand/field reads unchanged.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: Route every bbox computation through `KojiBbox` (incl. the bug fix)

After this task, `GetBbox` (trait + 3 impls), koji-core `BBox`, and `bbox_of` are all still *defined* but *unused* — they get deleted in Task 6. This keeps every intermediate commit green. **This task fixes the live geojson-order bug** (`create_or_find_collection`).

**Files:**
- Modify: `crates/algorithms/src/clustering/greedy.rs`
- Modify: `crates/koji-core/src/util.rs`
- Modify: `crates/koji-core/src/geometry/koji_output.rs` (`to_sql`)
- Modify: `crates/koji-core/src/geometry/geometry.rs` (member-bbox helper)
- Modify: `crates/koji-service/src/utils/mod.rs` (`load_collection` + `create_or_find_collection` bug fix)

- [ ] **Step 1: `greedy.rs` — replace `points.get_bbox()`**

`get_honeycomb_clusters` (lines 536-556). Replace the `let bbox = points.get_bbox(); let bbox_unwrap = bbox.clone().unwrap();` pair and the Feature construction so the ring + members come from `KojiBbox`. Add `use koji_core::KojiBbox;` to the file's imports if not present. New body for the bbox/feature setup:

```rust
    fn get_honeycomb_clusters(&self, points: &SingleVec) -> SingleVec {
        let kb = KojiBbox::from_points(points)
            .map(|b| b.trim(6))
            .unwrap_or(KojiBbox { min_lat: 0.0, min_lon: 0.0, max_lat: 0.0, max_lon: 0.0 });
        let arr = kb.to_geojson_bbox(); // [min_lon, min_lat, max_lon, max_lat]
        let bbox = Some(kb.to_geojson_bbox_vec());

        let feat = Feature {
            bbox: bbox.clone(),
            geometry: Some(Geometry {
                bbox,
                foreign_members: None,
                value: geojson::Value::Polygon(vec![vec![
                    vec![arr[0], arr[1]],
                    vec![arr[2], arr[1]],
                    vec![arr[2], arr[3]],
                    vec![arr[0], arr[3]],
                    vec![arr[0], arr[1]],
                ]]),
            }),
            ..Default::default()
        };
        radius::BootstrapRadius::new(&feat, self.radius).result()
    }
```

(The `unwrap_or` zero-box preserves the old `get_bbox` behavior of returning `[0,0,0,0]` for empty input.)

`get_s2_clusters` (lines 571-580). Replace `let bbox = points.get_bbox().unwrap();` and the index-juggled `get_region_cells` call with named fields:

```rust
    #[time()]
    fn get_s2_clusters(&self, points: &SingleVec, point_tree: &'a RTree<Point>) -> SingleVec {
        let kb = KojiBbox::from_points(points).map(|b| b.trim(6)).unwrap();
        s2::get_region_cells(kb.min_lat, kb.max_lat, kb.min_lon, kb.max_lon, 16)
            .0
            .into_par_iter()
            .flat_map(|cell| self.flat_map_cells(cell, point_tree))
            .map(|cell| cell.point_array())
            .collect()
    }
```

- [ ] **Step 2: `util.rs` — replace `feature_single_vec(...).get_bbox()`**

`sql_raw` (lines 38-42). Add `use crate::geometry::KojiBbox;` (or extend the existing `use crate::geometry::{...}` line). The else-branch computes a `Vec<f64>` to match `feature.bbox`'s type:

```rust
        let bbox = if let Some(bbox) = feature.bbox.clone() {
            bbox
        } else {
            KojiBbox::from_points(&feature_single_vec(feature))
                .map(|b| b.trim(6).to_geojson_bbox_vec())
                .unwrap_or_else(|| vec![0.0, 0.0, 0.0, 0.0])
        };
```

`sql_raw_bbox` (lines 68-74):

```rust
        let bbox = if let Some(bbox) = feature.bbox.as_ref() {
            bbox.clone()
        } else if let Some(bbox) =
            KojiBbox::from_points(&feature_single_vec(feature)).map(|b| b.trim(6).to_geojson_bbox_vec())
        {
            bbox
        } else {
            continue;
        };
```

Remove `GetBbox` from the `use crate::geometry::{...}` import on line 5 (it is no longer used in this file after the two replacements).

- [ ] **Step 3: `koji_output.rs` `to_sql` — replace `bbox_of`**

In `to_sql` (line 249), replace `let bbox = bbox_of(&single_group(&item.geometry));` with the `KojiBbox` path (returns `[f64;4]`, same indices `[0]=min_lon,[1]=min_lat,[2]=max_lon,[3]=max_lat`):

```rust
            let bbox = super::KojiBbox::from_points(&single_group(&item.geometry))
                .map(|b| b.trim(6).to_geojson_bbox())
                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
```

(`bbox_of` and `trim6` are now unused in this file — left for deletion in Task 6 so this commit stays green.)

- [ ] **Step 4: `geometry.rs` — member-bbox via helper**

The `GeometryHelpers::simplify` (line 75) and `TrimPrecision::trim_precision` (line 95) impls set `geometry.bbox = geometry.get_bbox();`. Add a private helper at the bottom of `crates/koji-core/src/geometry/geometry.rs` that computes the same trimmed geojson bbox without the trait:

```rust
/// The geojson `bbox` member `[min_lon, min_lat, max_lon, max_lat]` (trimmed to 6)
/// for a geojson `Geometry`, via the Koji-native path. Replaces the `GetBbox for
/// Geometry` delegation.
fn geometry_geojson_bbox(g: &Geometry) -> Option<geojson::Bbox> {
    let feature = geojson::Feature {
        geometry: Some(g.clone()),
        ..Default::default()
    };
    KojiGeometry::try_from(feature)
        .ok()
        .and_then(|kg| KojiGeometryCollection::new(vec![kg]).geojson_bbox())
}
```

Then change both member-sets (lines 75 and 95) from `geometry.bbox = geometry.get_bbox();` to:

```rust
        geometry.bbox = geometry_geojson_bbox(&geometry);
```

(`KojiGeometry` / `KojiGeometryCollection` are already imported via `use super::*`.)

- [ ] **Step 5: `koji-service/src/utils/mod.rs` — `load_collection` + the bug fix**

`load_collection` (lines 31-38). Replace `let bbox = feature.get_bbox();` with the collection-based projection (byte-identical to the old `Feature::get_bbox`, which also did `to_single_vec → get_bbox` trimmed to 6):

```rust
        Ok(feature) => {
            let bbox = koji_core::KojiGeometry::try_from(feature.clone())
                .ok()
                .and_then(|kg| koji_core::KojiGeometryCollection::new(vec![kg]).geojson_bbox());
            Ok(FeatureCollection {
                bbox: bbox.clone(),
                features: vec![Feature { bbox, ..feature }.ensure_first_last()],
                foreign_members: None,
            })
        }
```

`create_or_find_collection` (lines 61-76) — **THE BUG FIX**. The old path built a koji-core `BBox` from `geo::Point`s and called the buggy `get_geojson_bbox` (`[min_lon, max_lon, min_lat, max_lat]`). Replace the `data_points` branch body with a `KojiBbox` built from the `[lat, lon]` `data_points`, emitting the correct geojson order:

```rust
    if !data_points.is_empty() {
        // data_points is [lat, lon]; KojiBbox emits the correct geojson order.
        let kb = KojiBbox::from_points(data_points)
            .expect("data_points is non-empty in this branch")
            .trim(6);
        Ok(FeatureCollection {
            bbox: Some(kb.to_geojson_bbox_vec()),
            features: vec![Feature {
                bbox: Some(kb.to_geojson_bbox_vec()),
                geometry: Some(Geometry {
                    value: Value::Polygon(vec![vec![
                        vec![kb.min_lon, kb.min_lat],
                        vec![kb.min_lon, kb.max_lat],
                        vec![kb.max_lon, kb.max_lat],
                        vec![kb.max_lon, kb.min_lat],
                        vec![kb.min_lon, kb.min_lat],
                    ]]),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Feature::default()
            }],
            ..FeatureCollection::default()
        })
    } else if !area.features.is_empty() {
```

Update the imports on `crates/koji-service/src/utils/mod.rs:5`: remove `BBox` and `GetBbox`, add `KojiBbox`. The `Point` import (`geo::Point`, line 3) is now unused in this file (the `let points: Vec<Point>` line is deleted) — remove it if the compiler flags it unused.

- [ ] **Step 6: Build the affected crates**

Run in parallel:
```bash
cargo build -p algorithms -p koji-core -p koji-service
```
Expected: all compile. (Warnings about unused `BBox`/`GetBbox`/`bbox_of` are expected and resolved in Task 6.)

- [ ] **Step 7: Run tests for the touched crates**

Run in parallel:
```bash
cargo test -p koji-core
cargo test -p algorithms
```
Expected: PASS. The koji-core `to_sql`/sql-parity goldens and the algorithms clustering goldens confirm the `KojiBbox` routing is byte-preserving.

- [ ] **Step 8: Commit**

```bash
git add crates/algorithms/src/clustering/greedy.rs crates/koji-core/src/util.rs crates/koji-core/src/geometry/koji_output.rs crates/koji-core/src/geometry/geometry.rs crates/koji-service/src/utils/mod.rs
git commit -m "refactor(geometry): route all bbox computation through KojiBbox + fix order bug

greedy/util/to_sql/geometry-members/service now compute via KojiBbox::from_points.
create_or_find_collection no longer emits the swapped [min_lon,max_lon,min_lat,
max_lat] from the buggy BBox::get_geojson_bbox — it now emits the correct
[min_lon,min_lat,max_lon,max_lat]. GetBbox/BBox/bbox_of left dead for Task 6.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: Delete the dead `GetBbox` trait, koji-core `BBox`, and `bbox_of`

Everything below is now unused (verified by Task 5 still compiling with them present). Delete in one sweep; the compiler proves nothing references them.

**Files:**
- Modify: `crates/koji-core/src/geometry/mod.rs` (delete `GetBbox` trait + `BBox` struct/impls)
- Modify: `crates/koji-core/src/geometry/single_vec.rs` (delete `GetBbox for SingleVec`)
- Modify: `crates/koji-core/src/geometry/feature.rs` (delete `GetBbox for Feature`)
- Modify: `crates/koji-core/src/geometry/geometry.rs` (delete `GetBbox for Geometry`)
- Modify: `crates/koji-core/src/geometry/koji_output.rs` (delete `bbox_of` + `trim6`)

- [ ] **Step 1: Delete the trait and struct in `mod.rs`**

In `crates/koji-core/src/geometry/mod.rs`:
- Delete the `GetBbox` trait (lines 43-46, including the `/// [min_lon, min_lat, max_lon, max_lat]` doc).
- Delete the `BBox` struct (lines 56-62), its `impl Default for BBox` (64-73), and its `impl BBox { ... }` (75-106: `new`, `update`, `get_poly`, `get_geojson_bbox`).
- Remove now-unused imports on lines 34-35: `use geo::Point;` and `use geojson::Bbox;` (verify they are unused elsewhere in the file first: `rg -n 'Point|Bbox' crates/koji-core/src/geometry/mod.rs`).

- [ ] **Step 2: Delete the three `GetBbox` impls**

- `crates/koji-core/src/geometry/single_vec.rs`: delete `impl GetBbox for SingleVec { ... }` (lines 21-51). If `use crate::TrimPrecision;` (line 1) is now unused in the file, remove it (check `rg -n 'trim_precision|TrimPrecision' crates/koji-core/src/geometry/single_vec.rs`).
- `crates/koji-core/src/geometry/feature.rs`: delete `impl GetBbox for Feature { ... }` (lines 32-44).
- `crates/koji-core/src/geometry/geometry.rs`: delete `impl GetBbox for Geometry { ... }` (lines 100-117). The `geometry_geojson_bbox` helper added in Task 5 stays.

- [ ] **Step 3: Delete `bbox_of` + `trim6` in `koji_output.rs`**

Delete the `bbox_of` function (lines 362-396) and the `trim6` helper (lines 398-405) — `to_sql` no longer calls them (Task 5). Verify no other caller: `rg -n 'bbox_of|trim6' crates/koji-core/src`.

- [ ] **Step 4: Build + verify nothing referenced the deleted items**

Run:
```bash
cargo build -p koji-core
rg -n 'GetBbox|get_bbox|get_geojson_bbox|\bBBox\b|bbox_of' crates/koji-core crates/algorithms crates/koji-service
```
Expected: koji-core compiles clean. The `rg` returns **nothing** (every `BBox`/`GetBbox`/`get_bbox`/`bbox_of` is gone). If the build flags an unused import (`geojson::Bbox`, `geo::Point`, `TrimPrecision`), remove it.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry
git commit -m "refactor(geometry): delete dead GetBbox trait, BBox struct, bbox_of

All bbox computation routes through KojiBbox now. Removes the twin-BBox name
collision and the buggy get_geojson_bbox for good.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 7: Verification gate

**Files:** none (verification only).

- [ ] **Step 1: Full build, clippy, fmt, tests in parallel**

Run these concurrently (independent, read-only of each other):

```bash
cargo build --workspace
cargo clippy --workspace --all-targets
cargo fmt --check
cargo test --workspace
```

Expected: all exit 0. Background the long ones if needed (`cargo test --workspace` and clippy are the slow ones). **Do NOT** build the `wasm32` target here — `koji-wasm` requires nightly + `-Z build-std` and a plain `cargo build --target wasm32` is a false-failure trap; the host `--workspace` build covers `koji-wasm`'s host compile.

- [ ] **Step 2: Consolidation proof greps**

```bash
# Want ZERO: every legacy bbox type/trait/fn is gone.
rg -n 'GetBbox|get_geojson_bbox|bbox_of|struct BBox|struct BoundingBox|LatLonBBox' crates
# Want exactly ONE: the canonical type definition.
rg -n 'pub struct KojiBbox' crates
# Want ZERO field reads of the dropped collection bbox field (only method calls `.bbox()`).
rg -n 'KojiGeometryCollection' crates  # spot-check no `.bbox` field access remains
```

Expected: first grep — only `BoundingBox` in `fastest.rs` (the documented deferred type, Deviation #1) and nothing else; second — exactly one hit; third — no `<collection>.bbox` field reads.

- [ ] **Step 3: Manual confirmation of the bug fix**

Confirm the order-bug regression test from Task 1 is present and passing:

```bash
cargo test -p koji-core --lib to_geojson_bbox_is_lon_first
```

Expected: PASS — pins `[min_lon, min_lat, max_lon, max_lat]` so the swapped order can never return.

- [ ] **Step 4: Final commit (if any fmt/clippy fixups were needed)**

```bash
git add -A
git commit -m "chore(geometry): bbox consolidation verification gate green

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

(Skip if Steps 1-3 required no changes.)

---

## Done criteria

- One `KojiBbox` definition; `geo::Rect<f64>` retained as the geometry-layer internal.
- `BBox` (both), `BoundingBox` (lat/lon — fastest's x/y one deferred per Deviation #1), `LatLonBBox`, `GetBbox`, `bbox_of` all gone.
- `KojiGeometryCollection.bbox` is an on-demand method; no stored field.
- `BoundsArg` wire JSON unchanged; reads `.bbox.<field>`.
- The geojson-order bug is fixed and pinned by a regression test.
- `cargo build/clippy/fmt/test --workspace` all green.
