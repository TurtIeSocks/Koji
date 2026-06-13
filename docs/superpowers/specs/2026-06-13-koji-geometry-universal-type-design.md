# KojiGeometry — universal geometry interface (design)

**Status:** approved-in-principle (brainstorm complete), pending spec review
**Date:** 2026-06-13
**Branch:** `claude/goofy-tu-5372ed` (local, unpushed)
**Supersedes the geometry half of:** the P1 `FeatureCtx` + `To*` trait conversion layer in `koji-core`

---

## 1. Problem

Koji has no real universal geometry type. The de-facto interface is geojson `FeatureCollection`/`Feature`, threaded through every crate, with `GeoFormats` (a tagged union) at the API edge and a large web of conversion traits underneath.

Quantified, from the current tree:

- **~21 conversion traits** (`ToPointArray`, `ToSingleVec`, `ToMultiVec`, `ToPointStruct`, `ToFeature`, `ToCollection`, `ToPoracle`, `ToText`, `ToGeometry`, `ToSql`, plus `EnsurePoints`/`EnsureProperties`/`GetBbox`/`ValueHelpers`/`GeometryHelpers`/`FeatureHelpers`), **80+ impl blocks, ~1876 lines** in `crates/koji-core/src/geometry/`, plus a `wrapper_conversions!` macro generating four impls per type.
- It is an **N×M matrix**: every homegrown coordinate type (`PointArray` = `[f64;2]`, `SingleVec`, `MultiVec`, `PointStruct`, `Poracle`, `Text`, …) implements conversion to every other format.
- Homegrown coordinate types predate / run parallel to **geo-types** (georust). Algorithms already compute on `geo::` types; s2 (`0.0.13`) already bridges `CellID → geo::Polygon`.
- The "fence type" enum (`Type` in the DB, `FenceType` in core) is RDM-derived sludge with 12 variants, and is **abused to infer geometry shape** in several places — a documented source of confusion.
- The parent/children system is **one level deep**, fronted by a pile of boolean/string query args (`parentstart`, `parentend`, `parentreplace`, `group`, `ignoremanualparent`, `excludeparents`).

The friction the maintainer feels — "less friction between S2, georust, geojson" — comes from this N×M matrix and the type→shape inference, not from format I/O performance.

## 2. Goals

1. One internal type, `KojiGeometry`, used across all crates as the universal geometry interface.
2. API requests and DB reads normalize **into** it; outputs convert **out of** it.
3. Collapse the N×M conversion matrix to **N + M** (normalize through one canonical type).
4. Centralize the S2 ↔ vector-geometry bridge to one place.
5. Replace the RDM `Type`/`FenceType` enum with a clean four-variant `Mode`, and delete all geometry-shape-from-type inference.
6. Unify geofences and routes onto the one interface type.
7. Replace the one-level parent system + boolean-arg zoo with a recursive, depth-parameterized hierarchy.

## 3. Non-goals

- Adopting `geozero` (see §4.4).
- Migrating the DB `geometry` column from JSON to native WKB/MySQL geometry.
- Unifying the `geofence` and `route` **storage tables** (they stay separate — see §8).
- Reworking API security / auth (rides the separate security rework).
- Rewriting the admin client beyond the minimum needed to keep it compiling.

## 4. Locked decisions

### 4.1 Output-format scope — preserve all (bare arrays → standard geojson wrappers)
Koji's multi-format I/O (`?rt=` / `/convert`: Feature, Poracle, arrays, structs, Text, SQL) is a product feature and stays. The N×M matrix dies; the output formats survive as **1→M** adapters hanging off the collection type.

Two non-standard bare-array **wire encodings** are dropped in favor of their standard geojson wrappers (no capability lost):
- `FeatureVec` — bare `[Feature, …]` → `FeatureCollection` (identical payload, properties included).
- `GeometryVec` — bare `[Geometry, …]` → geojson `GeometryCollection`. This **promotes `GeometryCollection` to a first-class output** (Koji already accepts it on input, flattening to points). `GeometryCollection` is property-less by definition — it carries no per-feature metadata — the same tradeoff `GeometryVec` already had; it is the property-less multi-geometry option.

Single `Feature` and single `Geometry` outputs are retained.

### 4.2 Canonical core — geo-types
`KojiGeometry` wraps `geo::Geometry<f64>`. georust becomes the canonical math representation. Rationale: algorithms (clustering, bootstrap, routing) already compute on `geo::` types and get the whole geo ecosystem for free (`Contains`, `Area`, `Haversine`, `simplify`, `booleanop`, `BoundingRect`); s2 already converts to `geo::Polygon`; geojson↔geo-types is a built-in `From`/`TryFrom` in the geojson crate. Hot-path conversions happen once at the boundary instead of per-algorithm.

### 4.3 Metadata — hybrid
`geo::Geometry` carries no properties. `KojiMeta` is a hybrid: typed fields for what Koji branches on, plus a `#[serde(flatten)] extra` bag for lossless geojson-property passthrough.

### 4.4 geozero — not adopted
geozero is a geometry **I/O** library (WKB/WKT/GeoJSON/FlatGeobuf/GDAL ↔ geo-types streaming, sqlx WKB). It sits *below* geo-types (our canonical), does nothing for s2, cannot serialize Koji's bespoke formats (Poracle, the coordinate arrays), and its DB win only pays off with native-WKB columns we are not adopting. The one thing it would help — geojson↔geo-types — is already a cheap built-in at geofence-scale volumes. Revisit only if Koji later moves to WKB columns or needs FlatGeobuf/GDAL ingest.

### 4.5 Mode enum
Replace `Type` (DB enum on `geofence.mode` and `route.mode`) and `FenceType` (core mirror) with one `Mode { Unset, Pokemon, Fort, Quest }` in `koji-core`, shared by geofences and routes. See §7.

### 4.6 Fence/route unification
One `KojiGeometry` element type + one `KojiGeometryCollection` interface type represent both fences and routes. Storage tables stay separate. Fence-vs-route is contextual (table/endpoint), never inferred from geometry shape. See §8.

### 4.7 Recursive hierarchy
Keep the existing `geofence.parent` self-FK (adjacency list); add recursive descent via MySQL `WITH RECURSIVE`. Replace the boolean-arg zoo with `?depth=N`. Results are a flat `FeatureCollection` where each feature carries `parent_id` + an `ancestors:[names]` path. See §9.

## 5. The type model

All in `koji-core`.

```rust
/// The element: one geometry + its metadata. The building block — a DB row maps
/// to one of these, an algorithm emits one per feature, a single-entity event
/// payload is one of these.
pub struct KojiGeometry {
    pub geometry: geo::Geometry<f64>,
    pub meta: KojiMeta,
}

/// Hybrid metadata. Typed fields for what Koji branches on; `extra` is a
/// lossless passthrough for arbitrary geojson properties.
pub struct KojiMeta {
    pub id: Option<u32>,               // geofence/route id (DB-native; stringified only at the geojson edge)
    pub name: Option<String>,
    pub mode: Mode,                    // scan purpose; replaces FeatureCtx.fence_type
    pub parent_id: Option<u32>,        // hierarchy edge (geofences)
    pub ancestors: Vec<String>,        // ordered names root -> parent (populated on hierarchy reads)
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The universal interface type. Every API boundary speaks this. Newtype (not a
/// bare Vec) so the 1->M output conversions and a custom geojson Serialize have
/// a home (orphan rule), and so it can carry a collection-level bbox.
pub struct KojiGeometryCollection {
    pub items: Vec<KojiGeometry>,
    pub bbox: Option<geo::Rect<f64>>,  // computed on demand, not stored
}

/// Scan purpose. Shared by geofences and routes. Self-describing geometry means
/// this NEVER decides geometry shape.
pub enum Mode { Unset, Pokemon, Fort, Quest }
```

`geo::Geometry` already covers `Point` / `LineString` / `Polygon` / `MultiPolygon` / `MultiPoint` / `GeometryCollection`, so one `geometry` field handles every shape Koji has — including routes (an ordered `MultiPoint` / `LineString`).

The exact typed field set in `KojiMeta` is pinned during implementation against `crates/koji-db/src/db/geofence.rs:126` (`Model::to_feature`). Anything not promoted to a typed field lands in `extra`.

## 6. Conversion architecture

The hub. Everything normalizes through `KojiGeometry` / `KojiGeometryCollection`; nothing converts directly between two edge formats.

- **Inbound (N→1):** each source implements `TryFrom<X>` (or `From` when infallible) for the element/collection:
  - geojson `Feature` / `FeatureCollection`
  - `geo::Geometry`
  - the raw coordinate forms (`SingleVec` / `MultiVec` / structs) — inbound arms only
  - DB `geofence::Model` / `route::Model`
  - s2 cells (`CellID` / coverings)
- **Outbound (1→M):** `KojiGeometryCollection` (and `KojiGeometry` for singletons) converts to each output format:
  - geojson `FeatureCollection` / `Feature`, `GeometryCollection` / single `Geometry`
  - Poracle, `SingleArray`/`MultiArray`, `SingleStruct`/`MultiStruct`, Text, SQL
- **Total = N + M**, replacing the ~80-impl cross-product.
- **s2 bridge centralized** in `koji-core` (which already depends on s2): `KojiGeometry::from_s2_cells(…)`, `.s2_covering(level)` — one location, not scattered through `algorithms`.
- **geojson edge** rides the geojson crate's built-in `From<geo::Geometry>` / `TryFrom<&geojson::Geometry>` — no homegrown glue.
- The `?rt=` dispatcher (`crates/koji-service/src/utils/response.rs`) becomes `KojiGeometryCollection → match return_type → format`.

## 7. Mode enum + data migration

Current `Type` variants (MySQL ENUM, `string_value` shown) and the proposed collapse:

| New `Mode` | ← old `Type` values |
|-----------|---------------------|
| `Pokemon` | `circle_pokemon`, `circle_smart_pokemon`, `pokemon_iv`, `auto_pokemon`, `auto_tth` |
| `Fort` | `circle_raid`, `circle_smart_raid`, `circle_station` |
| `Quest` | `auto_quest`, `circle_quest` |
| `Unset` | `unset`, `leveling` |

**Flagged calls (verify at review):**
- `leveling` → `Unset` (it is account-leveling, not a data-scan purpose). Alternative: `Pokemon`.
- `circle_station` → `Fort` (a fixed POI). Alternative: its own handling later.

**DB migration:** both `geofence.mode` and `route.mode` are the same ENUM. Approach: widen the column to a string, `UPDATE` rows through the mapping above, then narrow the ENUM to the four new values. Order matters (cannot store a value absent from the ENUM).

### 7.1 Geometry-shape inference — delete
geo-types core makes geometry self-describing, so the type→shape (and shape→type) logic becomes dead code. Sites to delete:

- `crates/koji-core/src/geometry/geometry.rs:158` — `Geometry::to_feature` matches `FenceType` to pick Point/MultiPoint/MultiPolygon → **delete**, match the `geo::Geometry` variant.
- `crates/koji-core/src/geometry/multi_vec.rs:8` — `get_geojson_value(enum_type)` same → **delete**.
- `crates/koji-core/src/enum_map.rs:29` — `get_enum_by_geometry` infers `Type` from geometry shape (the reverse hack) → **delete outright**.
- `crates/koji-db/src/db/geofence.rs` `geo_type: String` column — **kept**. It is a denormalized fast-filter index (`geofence.rs:461` filters pagination by `Column::GeoType.eq(args.geotype)`), not dead weight. The refactor makes it *reliable* instead of dropping it: populate it deterministically from the `geo::Geometry` variant on every write so it can never drift from the actual geometry. Distinction — we delete *mode→shape inference* (using the semantic enum to guess geometry shape), **not** the geometry's own variant name cached for indexing.
- `crates/koji-db/src/db/route.rs:89` — `to_feature` hardcodes `CirclePokemon` and ignores the row's `mode` (a live bug) → fixed by reading the row's `mode`.
- `crates/koji-service/src/public/v2/calc.rs:235` — `fence_type_for(category)` (gym/pokestop/station → mode) is legitimate semantic tagging; **retarget** to `Mode` (gym→Fort, station→Fort, pokestop→Quest, else→Pokemon).

## 8. Fence/route unification

`geo::Geometry` covers `MultiPoint`, so a route (ordered scan points) and a fence (Polygon/MultiPolygon) are both `geo::Geometry` variants. One `KojiGeometry` carries both; `meta.mode` tags purpose.

- **Storage stays separate.** `route` has `geofence_id` / `points` / `description`; `geofence` has `parent` / `dragonite_area_id`. Genuinely different rows. Only the in-memory/transport type unifies.
- **Role is contextual** (which table/endpoint), never a type field, never inferred from shape. If a mixed payload ever needs to self-describe, add an explicit `kind` enum then — not now (YAGNI).
- **Falls out for free:** `route.points` becomes derivable from the geometry (kept as a stored denormalization for now; may drop later). (`geofence.geo_type` is *kept* — a fast-filter denormalization, §7.1 — but its value is now derived deterministically from the `geo::Geometry` variant on write.)

## 9. Recursive hierarchy

The adjacency list already exists — `geofence.parent: Option<u32>`, a self-referential FK — but is never walked (today: strictly one level via `WHERE parent = ?`).

- **Storage:** keep the self-FK. Add recursive descent via MySQL `WITH RECURSIVE` (raw SQL through sea-orm's `Statement`; sea-orm has no native recursive CTE). MySQL 8+/9.x supports it.
- **API:** `GET /api/v2/geofences/{id}?depth=N` returns the node plus descendants down to N levels:
  - `depth=0` → just the node
  - `depth=1` → node + direct children
  - `depth=k` → node + descendants to k levels
  - `depth` omitted / `depth=all` → the entire subtree
  - A sane maximum-depth guard caps runaway queries.
- **Result shape:** a flat `FeatureCollection`. Each feature carries `parent_id` and `ancestors:[names]` (ordered root→parent), so clients can render breadcrumbs and build the tree themselves.
- **Dropped args** (RDM name-munging): `parentstart`, `parentend`, `parentreplace`, `group`, `ignoremanualparent`, `excludeparents`. The structured `ancestors[]` field replaces every display-name hack.

## 10. What dies, demotes, lives

| Dies outright | Demoted to output-only adapter (1→M) | Lives |
|---|---|---|
| `GeoFormats` enum | Poracle | `geo::Geometry` (the new core) |
| `FeatureCtx` (→ `KojiMeta`, carried in the type) | `SingleVec`/`MultiVec` arrays | geojson (edge; built-in `From`) |
| 21 `To*` traits' N×M cross-product | `SingleStruct`/`MultiStruct` | `PointArray`/`PointStruct` (coord aliases inside adapters) |
| `wrapper_conversions!` macro | Text / AltText | s2 bridge (centralized in koji-core) |
| `enum_map.rs` (shape↔type guessing) | SQL | |
| type→shape match arms (`geometry.rs:158`, `multi_vec.rs:8`) | | |
| `FenceType` + `Type` (→ `Mode`) | | |
| `FeatureVec` / `GeometryVec` bare-array encodings | | |
| name-munge query args (§9) | | |

## 11. Boundary rewiring

- **API inbound:** JSON → `TryFrom` → `KojiGeometryCollection`. `Args.area`, `resolve_area`, `resolve_data_points` collapse into one inbound adapter.
- **API outbound:** `KojiGeometryCollection` → `?rt=` dispatcher → format. `response::send` takes the collection.
- **DB read:** `Model` → `KojiGeometry` (`geometry: Json` → geojson → `geo::Geometry`; `mode` → `Mode`; `parent`/properties → `meta`). **DB write:** the reverse (`geo::Geometry` → geojson JSON; `Mode` → enum). `upsert_from_geometry` takes the collection.
- **Algorithms:** borrow `&geo::Geometry` directly (they already do), return `Vec<KojiGeometry>` / a collection. `bootstrap::main(FeatureCollection) → Vec<Feature>` becomes the `KojiGeometry` forms.
- **Events / Dragonite:** `geofence_updated` / `route_updated` carry a `KojiGeometry`, serialized to a geojson `Feature` at the Dragonite edge (Dragonite expects geojson).

## 12. Implementation phases

Each phase compiles and passes tests on its own (green checkpoint).

1. **Foundation.** `koji-core` gains `Mode`, `KojiGeometry` / `KojiMeta` / `KojiGeometryCollection`, the conversion hub (inbound `TryFrom`, outbound `From`), and the centralized s2 bridge. The `Type → Mode` data migration (geofence + route). All lands *beside* the existing machinery so nothing breaks yet. Tests: round-trip + parity.
2. **Rewire + delete.** DB models, API in/out, algorithms, and events switch to `KojiGeometry`. Delete the inference sites, `GeoFormats`, `FeatureCtx`, `enum_map`, the `To*` cross-product + macro, and the `FeatureVec` / `GeometryVec` bare-array encodings (replaced by `FeatureCollection` / `GeometryCollection`). (`geo_type` is kept — §7.1.)
3. **Hierarchy.** Recursive CTE, `?depth=N`, the `ancestors[]` field, and removal of the name-munge args.

Each phase gets its own implementation plan (writing-plans) when reached.

## 13. Testing & parity strategy

- **Golden snapshots:** before refactoring, capture current outputs for representative geometries across every retained `?rt=` format. Assert the new `KojiGeometry → format` path reproduces them.
- **Round-trip property tests:** `geojson → KojiGeometry → geojson` is idempotent; `Model → KojiGeometry → Model` preserves fields.
- **Migration test:** every old `Type` value maps to the intended `Mode`; no row is left on an unmapped value.
- **Hierarchy:** a seeded country→state→county→city fixture; assert `?depth=N` returns exactly the expected level set and `ancestors[]` is correct.
- v1 byte-compatibility is moot ("clean v2"), but retained output formats stay stable for external consumers (e.g. Poracle).

## 14. Assumptions (verify at review)

1. `leveling → Unset`, `circle_station → Fort` mode mapping (§7).
2. `GeometryVec` bare-array dropped in favor of geojson `GeometryCollection`, promoted to a first-class output format (property-less, as `GeometryVec` was; already accepted on input). `FeatureVec` likewise dropped in favor of `FeatureCollection`. *(Confirmed at review 2026-06-13.)*
3. **GET shape:** single-resource `GET /{id}` returns a single `Feature` (standard REST); list endpoints return `FeatureCollection`. (Earlier "force the collection" was about dropping `FeatureVec`, not about wrapping single-resource reads.)
4. Typed `KojiMeta` field set = `id`, `name`, `mode`, `parent_id`, `ancestors`; everything else → `extra`. Final set pinned against `geofence.rs:126`.
5. DB `geometry` column stays JSON (no WKB migration).
6. `route.points` retained as a stored count for now (derivable from geometry; may drop later).
7. s2 bridge lives in `koji-core` (already a dependency).

## 15. Out of scope / deferred

- WKB / native-MySQL-geometry columns.
- An explicit fence/route `kind` discriminator (add only if a mixed payload needs it).
- A nested-tree hierarchy endpoint (flat + `ancestors[]` ships first; nested can be added later).
- `?ancestors=N` upward-walk parameter (the always-present `ancestors[]` field covers breadcrumbs).
- Client/admin-panel rework beyond keeping it compiling.
