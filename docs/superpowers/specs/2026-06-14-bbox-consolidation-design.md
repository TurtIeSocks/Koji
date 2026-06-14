# Bbox Consolidation — Design

- **Date:** 2026-06-14
- **Branch:** `claude/v2` (local, unpushed — rides the API security rework)
- **Status:** Approved (design); pending implementation plan
- **Follows:** the `KojiGeometry` universal-type refactor (Phases 1, 1B, 2 — complete)

## Problem

An audit found **nine** in-house bounding-box representations scattered across the
workspace (plus two foreign ones we cannot own). The user suspected "at least 3";
the reality is worse, and the real hazard is not the type count but the **six-plus
coordinate orderings** those types disagree on.

### Inventory

| # | Type | Where | Backing | Field order | Live? |
|---|------|-------|---------|-------------|-------|
| 1 | `BBox` | koji-core `geometry/mod.rs:57` | struct `min_x/min_y/max_x/max_y` | x/y (=lon/lat) | yes — 1 caller (koji-service `load_collection`); carries the buggy `get_geojson_bbox` |
| 2 | `BBox` | algorithms `clustering/candidates.rs:12` | struct `min_lat/max_lat/min_lon/max_lon` | lat-first | yes — module-private |
| 3 | `BoundingBox` | algorithms `clustering/fastest.rs:10` | struct `min_x/min_y/max_x/max_y` (geo::Coord) | x/y | yes — module-private |
| 4 | `LatLonBBox` | algorithms `clustering/partition.rs:158` | struct `min_lat/max_lat/min_lon/max_lon` | lat-first | yes — `pub(crate)` |
| 5 | `BoundsArg` | koji-core `query_args.rs:120` | struct `min_lat/min_lon/max_lat/max_lon` + extras | lat-first | yes — **serializable** API input filter |
| 6 | `geo::Rect<f64>` | koji-core `koji_geometry.rs:52` | geo native | x=lon / y=lat | yes — `KojiGeometryCollection.bbox`, de-facto canonical |
| 7 | `[f64;4]` `bbox_of()` | koji-core `koji_output.rs:362` | array | `[min_lon,min_lat,max_lon,max_lat]` | yes — `to_sql` |
| 8 | `geojson::Bbox` (`Vec<f64>`) | `GetBbox` trait | vec | `[min_lon,min_lat,max_lon,max_lat]` | yes — trait, 3 impls |
| 9 | `[f64;4]` `viewbox` | nominatim search/lookup | array | `[x1,y1,x2,y2]` | yes — **foreign** nominatim API contract |
| F1 | `rstar::AABB` | algorithms rtree | foreign | — | foreign (rstar `RTreeObject` trait) |
| F2 | `s2::Rect` | koji-core s2 | foreign | — | foreign (s2 crate region) |

### Defects surfaced by the audit

- **Order bug (live).** `BBox::get_geojson_bbox()` (mod.rs:104) returns
  `[min_x, max_x, min_y, max_y]`, but the `GetBbox` trait doc promises
  `[min_lon, min_lat, max_lon, max_lat]`. Positions 1 and 2 are swapped. It feeds the
  `bbox` member on Features/FeatureCollections built in koji-service `load_collection`
  — a real, wire-visible wrong value.
- **Twin-`BBox` landmine.** Two structs both named `BBox` with opposite axes
  (koji-core x/y vs algorithms lat/lon). Importing the wrong one silently swaps
  coordinates.

## Goals

- Collapse the in-house bbox zoo to **one Koji-owned type** plus the unavoidable
  `geo::Rect<f64>`, with a single, well-tested bridge between them.
- Eliminate the coordinate-order chaos by giving the domain-facing type explicit,
  self-documenting named fields.
- Fix the `get_geojson_bbox` order bug and pin the correct order with a parity test.
- Connect the canonical bbox to `KojiGeometry` / `KojiGeometryCollection` without
  introducing stored, stale-able state.

## Non-goals

- Touching foreign types `rstar::AABB` (#F1) or `s2::Rect` (#F2) — they are required
  by external trait/crate contracts.
- Changing the nominatim `viewbox` param (#9) — it is nominatim's external API contract.
- Any change to serialized API wire formats. The `BoundsArg` JSON and the geojson
  `bbox` member shape stay byte-identical (the geojson *values* get corrected, but the
  shape does not change).

## Decisions (signed off)

1. **Layered, not one-type.** `geo::Rect<f64>` remains the geometry-layer internal
   (x=lon / y=lat; geo's `union`/`contains`/`bounding_rect` come for free). One
   Koji-owned `KojiBbox` (lat-first, serializable) serves the compute/boundary layer.
   This mirrors the project's established `KojiGeometry`-vs-`SingleVec`
   boundary/internal split.
2. **On-demand connection, no stored state.** Drop the cached
   `KojiGeometryCollection.bbox` field; expose `bbox()` and `koji_bbox()` accessors on
   both the collection and the item, computed on demand. (`KojiGeometry::bbox()` is
   already on-demand via geo's `bounding_rect()`; this makes the collection match.)
3. **`BoundsArg` embeds `KojiBbox` via `#[serde(flatten)]`** — its first four fields
   are exactly a `KojiBbox`, and flatten keeps the wire JSON identical.

## Design

### The type — `koji-core`, new `geometry/koji_bbox.rs`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KojiBbox {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

impl KojiBbox {
    // The ONE bridge to the geometry-layer canonical (geo::Rect is x=lon, y=lat).
    pub fn from_rect(r: geo::Rect<f64>) -> Self;
    pub fn to_rect(self) -> geo::Rect<f64>;

    // Build from the internal [lat, lon] compute currency (SingleVec / PointArray).
    pub fn from_points(pts: &[[f64; 2]]) -> Option<Self>;

    // Ops — absorb the algo structs' methods.
    pub fn union(self, other: Self) -> Self;
    pub fn expand(self, deg: f64) -> Self;
    pub fn contains(&self, lat: f64, lon: f64) -> bool;

    // Boundary outputs — CORRECT geojson order (kills the bug).
    pub fn to_geojson_bbox(self) -> [f64; 4];     // [min_lon, min_lat, max_lon, max_lat]
    pub fn to_geojson_bbox_vec(self) -> Vec<f64>; // same, for geojson::Bbox slots
}
```

`from_rect`/`to_rect` is the single conversion between the two layers; everything else
routes through it. `union`/`expand`/`contains`/`from_points` absorb the methods the
deleted algo structs carried.

### Connection — `koji_geometry.rs`

```rust
impl KojiGeometry {
    pub fn bbox(&self) -> Option<Rect<f64>>;     // already exists (geo bounding_rect)
    pub fn koji_bbox(&self) -> Option<KojiBbox>; // bbox().map(KojiBbox::from_rect)
}
impl KojiGeometryCollection {
    pub fn bbox(&self) -> Option<Rect<f64>>;     // on-demand fold of items via union_rect
    pub fn koji_bbox(&self) -> Option<KojiBbox>;
}
```

The constructor simplifies to `Self { items }`. The `union_rect` helper stays as the
fold used by the on-demand `bbox()`. Only **two tests** read the old `.bbox` field;
they repoint to `.bbox()`. No struct-literal constructs the field, nothing hot reads
it, so removal is safe.

### Migration map

| # | Today | Fate |
|---|-------|------|
| 1 | koji-core `BBox` (x/y) + buggy `get_geojson_bbox` | **deleted** — `load_collection` routes its points' bounds through `KojiBbox` → `to_geojson_bbox_vec()` (via `from_rect` of the geo bounding rect), fixing the order bug |
| 2 | algorithms `BBox` (candidates) | **deleted** → `KojiBbox` (exact field match; `from_points`/`expand`) |
| 4 | algorithms `LatLonBBox` (partition) | **deleted** → `KojiBbox`; `cell_bbox_lat_lon` returns `KojiBbox` |
| 3 | algorithms `BoundingBox` (fastest, x/y) | → `geo::Rect<f64>` (the x/y canonical; a coord-space AABB, no lat/lon confusion) |
| 5 | `BoundsArg` 4 fields | `#[serde(flatten)] bbox: KojiBbox` + extras — wire identical |
| 7 | `bbox_of()` `[f64;4]` (to_sql) | → `KojiBbox::from_points(...).to_geojson_bbox()` |
| 8 | `GetBbox` trait + 3 impls | **deleted** — callers (greedy/util/service) take `KojiBbox`; geojson `Geometry.bbox`/`Feature.bbox` members fed by `to_geojson_bbox_vec()` |

### Scope boundaries (untouched, by decision)

- `rstar::AABB` (#F1), `s2::Rect` (#F2) — foreign; required by external contracts.
- nominatim `viewbox` (#9) — external API contract. An optional
  `KojiBbox::to_nominatim_viewbox()` helper is deferred under YAGNI until a caller
  needs it.

### The bug fix (load-bearing)

`get_geojson_bbox` → `[min_x, max_x, min_y, max_y]` (wrong). Replacement
`KojiBbox::to_geojson_bbox()` → `[min_lon, min_lat, max_lon, max_lat]` (correct). A
parity test pins the order so it cannot regress. Because the buggy output fed live
Feature `bbox` members, this is a genuine wire correction: any client that adapted to
the *wrong* order will now see corrected values. Flag in the PR / changelog.

## Task sequence (for the implementation plan)

1. `KojiBbox` type + methods + unit tests (pure addition — nothing else depends on it yet).
2. Collection/item accessors (`bbox()`, `koji_bbox()`); drop the stored field; repoint 2 tests.
3. `BoundsArg` flatten + constructor/field-access updates; confirm OpenAPI schema unchanged.
4. algorithms migration: #2 and #4 → `KojiBbox`; #3 → `geo::Rect<f64>`.
5. Retire koji-core `BBox` + `GetBbox`; route `load_collection`, `bbox_of`, and the
   geojson `bbox` members through `KojiBbox`; fix and parity-test the order bug.
6. Verification gate.

## Verification

- `cargo build` / `clippy` / `fmt` / `test` across the workspace (run in parallel).
- Full algorithms parity (the clustering structs are on the hot path — confirm the
  swap to `KojiBbox` / `geo::Rect` preserves results).
- The new order-parity test asserting `to_geojson_bbox() == [min_lon, min_lat, max_lon, max_lat]`.
- Grep proof: zero remaining `BBox` / `BoundingBox` / `LatLonBBox` / `GetBbox` /
  `get_geojson_bbox` outside the deletions; one `KojiBbox` definition.

## Risks

- **Wire correction, not wire change.** The geojson `bbox` member shape is unchanged,
  but its *values* are corrected. Downstream consumers that compensated for the bug
  need a heads-up.
- **algorithms hot path.** The clustering structs (#2/#3/#4) are performance-sensitive.
  The migration must be result-preserving — covered by the parity gate. `geo::Rect`
  lacks an in-place "expand to include point"; the #3 swap needs a small rebuild helper.
- **`serde(flatten)` + extras** on `BoundsArg` is standard, but every Rust constructor /
  field access on `BoundsArg` changes to the nested `bbox` shape.
