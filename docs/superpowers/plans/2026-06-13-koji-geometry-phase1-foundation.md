# KojiGeometry Phase 1 — type foundation + Mode enum

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Introduce the `KojiGeometry` element type, the `KojiGeometryCollection` interface type, the `KojiMeta` sidecar, and the collapsed `Mode` enum — plus the geojson/geo/DB-Model conversions — purely additively, beside the existing machinery.

**Architecture:** `KojiGeometry` wraps `geo::Geometry<f64>` (canonical core) + a `KojiMeta` hybrid sidecar (typed fields + a flattened `extra` bag). `KojiGeometryCollection` is the universal interface newtype (`Vec<KojiGeometry>` + optional bbox) so the 1→M output adapters and a custom geojson serialization have a home. `Mode {Unset,Pokemon,Fort,Quest}` is a koji-core domain enum mirrored by a koji-db `DeriveActiveEnum`, bridged by the existing `enum_bridge!` macro. All conversions normalize through the new types (N→1 inbound, 1→M outbound).

**Tech Stack:** Rust (edition 2024), `geo`/`geo-types` 0.31/0.7, `geojson` 0.24 (geo-types feature on), `serde`/`serde_json`, sea-orm 1.1 (`DeriveActiveEnum`), the in-repo `macros` crate (`StrEnum`) and `enum_bridge!` macro.

---

## Scope & phasing notes

- **Additive only.** Nothing in this plan deletes or rewires existing code. `FenceType`, `Type`, `FenceMode`, `RouteMode`, `FeatureCtx`, `GeoFormats`, the `To*` traits, and `Model::to_feature` all stay untouched. The new types live alongside them. Deletion + boundary rewiring is Phase 2.
- **No DB migration here.** The `Type→Mode` column migration is breaking (existing Models deserialize the old enum) and lands atomically with the model switch in Phase 2. This plan only defines the Rust `Mode` enums + the legacy mapping function (unit-tested).
- **Carved out to a sibling plan (Phase 1B):** the s2↔geo bridge centralization and the remaining outbound format adapters (Poracle, Text, SQL, `SingleArray`/`MultiArray`, `SingleStruct`/`MultiStruct`). They are additive adapters best specified against their existing format code; this plan delivers the type spine + the geojson/geo/DB-Model conversions everything else builds on.
- **Reference spec:** `docs/superpowers/specs/2026-06-13-koji-geometry-universal-type-design.md`.

## File structure

| File | Responsibility | Action |
|------|----------------|--------|
| `crates/koji-core/src/mode.rs` | `Mode` domain enum + `from_legacy` mapping | Create |
| `crates/koji-core/src/geometry/koji_meta.rs` | `KojiMeta` hybrid sidecar | Create |
| `crates/koji-core/src/geometry/koji_geometry.rs` | `KojiGeometry` element + `KojiGeometryCollection` interface | Create |
| `crates/koji-core/src/geometry/koji_geojson.rs` | geojson ↔ KojiGeometry/Collection conversions | Create |
| `crates/koji-core/src/lib.rs` | module + re-export wiring | Modify |
| `crates/koji-core/src/geometry/mod.rs` | submodule declarations + re-exports | Modify |
| `crates/koji-db/src/db/sea_orm_active_enums.rs` | `Mode` `DeriveActiveEnum` + `enum_bridge!` | Modify |
| `crates/koji-db/src/db/geofence.rs` | `Model::to_koji_geometry` (additive method) | Modify |

`geo::Geometry` already covers Point/LineString/Polygon/MultiPolygon/MultiPoint/GeometryCollection, so one `geometry` field handles every shape — including routes (ordered `MultiPoint`/`LineString`).

---

### Task 1: `Mode` domain enum + legacy mapping (koji-core)

**Files:**
- Create: `crates/koji-core/src/mode.rs`
- Modify: `crates/koji-core/src/lib.rs` (add `mod mode;` + `pub use mode::Mode;`)
- Test: inline `#[cfg(test)]` in `crates/koji-core/src/mode.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_uses_lowercase_strings() {
        assert_eq!(serde_json::to_string(&Mode::Pokemon).unwrap(), "\"pokemon\"");
        assert_eq!(serde_json::to_string(&Mode::Unset).unwrap(), "\"unset\"");
        let m: Mode = serde_json::from_str("\"quest\"").unwrap();
        assert_eq!(m, Mode::Quest);
    }

    #[test]
    fn default_is_unset() {
        assert_eq!(Mode::default(), Mode::Unset);
    }

    #[test]
    fn from_legacy_maps_all_twelve() {
        for s in ["circle_pokemon", "circle_smart_pokemon", "pokemon_iv", "auto_pokemon", "auto_tth"] {
            assert_eq!(Mode::from_legacy(s), Mode::Pokemon, "{s}");
        }
        for s in ["circle_raid", "circle_smart_raid", "circle_station"] {
            assert_eq!(Mode::from_legacy(s), Mode::Fort, "{s}");
        }
        for s in ["auto_quest", "circle_quest"] {
            assert_eq!(Mode::from_legacy(s), Mode::Quest, "{s}");
        }
        for s in ["unset", "leveling", "anything_unknown"] {
            assert_eq!(Mode::from_legacy(s), Mode::Unset, "{s}");
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core mode::tests`
Expected: FAIL — `Mode` not found / does not compile.

- [ ] **Step 3: Write the implementation**

```rust
//! `Mode` — the scan-purpose tag shared by geofences and routes. Replaces the
//! RDM-derived `Type` / `FenceType` / `FenceMode` / `RouteMode` enums. It is a
//! pure semantic tag and NEVER decides geometry shape (the geometry is
//! self-describing). koji-db mirrors this as a `DeriveActiveEnum` and bridges
//! via `enum_bridge!`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Unset,
    Pokemon,
    Fort,
    Quest,
}

impl Mode {
    /// Map a legacy RDM mode string (any of the 12 historical values, or an
    /// unknown) to the collapsed four-variant `Mode`. Unknown → `Unset`.
    pub fn from_legacy(s: &str) -> Self {
        match s {
            "circle_pokemon" | "circle_smart_pokemon" | "pokemon_iv" | "auto_pokemon"
            | "auto_tth" => Mode::Pokemon,
            "circle_raid" | "circle_smart_raid" | "circle_station" => Mode::Fort,
            "auto_quest" | "circle_quest" => Mode::Quest,
            "unset" | "leveling" => Mode::Unset,
            _ => Mode::Unset,
        }
    }

    /// The lowercase wire/DB string for this mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Unset => "unset",
            Mode::Pokemon => "pokemon",
            Mode::Fort => "fort",
            Mode::Quest => "quest",
        }
    }
}
```

Then in `crates/koji-core/src/lib.rs`, add `mod mode;` with the other `mod` lines and `pub use mode::Mode;` with the other `pub use` lines.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core mode::tests`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/mode.rs crates/koji-core/src/lib.rs
git commit -m "feat(geometry): add collapsed Mode enum + legacy mapping"
```

---

### Task 2: `Mode` DB enum + bridge (koji-db)

**Files:**
- Modify: `crates/koji-db/src/db/sea_orm_active_enums.rs` (append `Mode` + bridge)
- Test: inline `#[cfg(test)]` in the same file

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod mode_bridge_tests {
    use super::Mode as DbMode;
    use koji_core::Mode as CoreMode;

    #[test]
    fn bridges_both_directions() {
        assert_eq!(CoreMode::from(DbMode::Fort), CoreMode::Fort);
        assert_eq!(DbMode::from(CoreMode::Quest), DbMode::Quest);
    }

    #[test]
    fn db_string_values_are_lowercase() {
        use sea_orm::ActiveEnum;
        assert_eq!(DbMode::Pokemon.to_value(), "pokemon");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-db mode_bridge_tests`
Expected: FAIL — `Mode` not found in `sea_orm_active_enums`.

- [ ] **Step 3: Write the implementation**

Append to `crates/koji-db/src/db/sea_orm_active_enums.rs`:

```rust
/// Storage mirror of `koji_core::Mode`. The geofence/route `mode` columns use
/// this; the four variants line up 1:1 with the domain enum and are bridged
/// below via `enum_bridge!`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "mode")]
pub enum Mode {
    #[sea_orm(string_value = "unset")]
    Unset,
    #[sea_orm(string_value = "pokemon")]
    Pokemon,
    #[sea_orm(string_value = "fort")]
    Fort,
    #[sea_orm(string_value = "quest")]
    Quest,
}

koji_core::enum_bridge!(Mode, koji_core::Mode, [Unset, Pokemon, Fort, Quest]);
```

(Confirm `koji_core` and `sea_orm::EnumIter`/`DeriveActiveEnum` are already imported at the top of the file; the existing `Type`/`FenceMode` enums use `use sea_orm::entity::prelude::*;`, which provides them. `enum_bridge!` is re-exported from `koji_core`.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-db mode_bridge_tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-db/src/db/sea_orm_active_enums.rs
git commit -m "feat(geometry): add koji-db Mode active-enum + core bridge"
```

---

### Task 3: `KojiMeta` sidecar (koji-core)

**Files:**
- Create: `crates/koji-core/src/geometry/koji_meta.rs`
- Modify: `crates/koji-core/src/geometry/mod.rs` (add `mod koji_meta;` + `pub use koji_meta::KojiMeta;`)
- Test: inline `#[cfg(test)]`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn typed_fields_and_extra_roundtrip_losslessly() {
        let json = serde_json::json!({
            "id": 7,
            "name": "Denver",
            "mode": "fort",
            "parent_id": 3,
            "color": "#ff0000",          // unknown -> extra
            "custom_flag": true          // unknown -> extra
        });
        let meta: KojiMeta = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(meta.id, Some(7));
        assert_eq!(meta.mode, Mode::Fort);
        assert_eq!(meta.parent_id, Some(3));
        assert_eq!(meta.extra.get("color").unwrap(), "#ff0000");
        assert!(meta.ancestors.is_empty());
        // round-trips back to the same object
        assert_eq!(serde_json::to_value(&meta).unwrap(), json);
    }

    #[test]
    fn empty_meta_omits_optional_keys() {
        let v = serde_json::to_value(KojiMeta::default()).unwrap();
        assert_eq!(v, serde_json::json!({ "mode": "unset" }));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_meta`
Expected: FAIL — `KojiMeta` not found.

- [ ] **Step 3: Write the implementation**

```rust
//! `KojiMeta` — the hybrid metadata sidecar for `KojiGeometry`. Typed fields for
//! what Koji branches on; a flattened `extra` bag carries arbitrary geojson
//! properties losslessly. Replaces `FeatureCtx` and the geojson `properties`
//! object.

use serde::{Deserialize, Serialize};

use crate::Mode;

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct KojiMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ancestors: Vec<String>,
    /// Lossless passthrough for any property not promoted to a typed field.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}
```

Then add to `crates/koji-core/src/geometry/mod.rs`: `mod koji_meta;` and `pub use koji_meta::KojiMeta;`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_meta`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_meta.rs crates/koji-core/src/geometry/mod.rs
git commit -m "feat(geometry): add KojiMeta hybrid sidecar"
```

---

### Task 4: `KojiGeometry` + `KojiGeometryCollection` (koji-core)

**Files:**
- Create: `crates/koji-core/src/geometry/koji_geometry.rs`
- Modify: `crates/koji-core/src/geometry/mod.rs` (add `mod koji_geometry;` + re-exports)
- Test: inline `#[cfg(test)]`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use geo::{Geometry, Point, Rect, coord};

    #[test]
    fn element_carries_geometry_and_default_meta() {
        let g = KojiGeometry::new(Point::new(1.0, 2.0));
        assert!(matches!(g.geometry, Geometry::Point(_)));
        assert_eq!(g.meta, crate::KojiMeta::default());
    }

    #[test]
    fn element_bbox_is_geometry_bounds() {
        let g = KojiGeometry::new(Point::new(3.0, 4.0));
        assert_eq!(g.bbox(), Some(Rect::new(coord! {x:3.0,y:4.0}, coord! {x:3.0,y:4.0})));
    }

    #[test]
    fn collection_bbox_unions_items() {
        let c = KojiGeometryCollection::new(vec![
            KojiGeometry::new(Point::new(0.0, 0.0)),
            KojiGeometry::new(Point::new(10.0, 5.0)),
        ]);
        assert_eq!(c.items.len(), 2);
        assert_eq!(c.bbox, Some(Rect::new(coord! {x:0.0,y:0.0}, coord! {x:10.0,y:5.0})));
    }

    #[test]
    fn empty_collection_has_no_bbox() {
        assert_eq!(KojiGeometryCollection::default().bbox, None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_geometry`
Expected: FAIL — types not found.

- [ ] **Step 3: Write the implementation**

```rust
//! `KojiGeometry` (one geometry + metadata) and `KojiGeometryCollection` (the
//! universal interface newtype). geo-types is the canonical core; everything
//! else converts at the edge.

use geo::{BoundingRect, Geometry, Rect};

use super::KojiMeta;

#[derive(Debug, Clone, PartialEq)]
pub struct KojiGeometry {
    pub geometry: Geometry<f64>,
    pub meta: KojiMeta,
}

impl KojiGeometry {
    pub fn new(geometry: impl Into<Geometry<f64>>) -> Self {
        Self { geometry: geometry.into(), meta: KojiMeta::default() }
    }

    pub fn with_meta(mut self, meta: KojiMeta) -> Self {
        self.meta = meta;
        self
    }

    pub fn bbox(&self) -> Option<Rect<f64>> {
        self.geometry.bounding_rect()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KojiGeometryCollection {
    pub items: Vec<KojiGeometry>,
    pub bbox: Option<Rect<f64>>,
}

impl KojiGeometryCollection {
    pub fn new(items: Vec<KojiGeometry>) -> Self {
        let bbox = items.iter().filter_map(KojiGeometry::bbox).reduce(union_rect);
        Self { items, bbox }
    }
}

impl FromIterator<KojiGeometry> for KojiGeometryCollection {
    fn from_iter<I: IntoIterator<Item = KojiGeometry>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

/// Smallest rect covering both inputs.
fn union_rect(a: Rect<f64>, b: Rect<f64>) -> Rect<f64> {
    use geo::coord;
    Rect::new(
        coord! { x: a.min().x.min(b.min().x), y: a.min().y.min(b.min().y) },
        coord! { x: a.max().x.max(b.max().x), y: a.max().y.max(b.max().y) },
    )
}
```

Then add to `crates/koji-core/src/geometry/mod.rs`: `mod koji_geometry;` and `pub use koji_geometry::{KojiGeometry, KojiGeometryCollection};`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_geometry`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_geometry.rs crates/koji-core/src/geometry/mod.rs
git commit -m "feat(geometry): add KojiGeometry element + KojiGeometryCollection interface"
```

---

### Task 5: geojson → KojiGeometry / Collection (inbound)

**Files:**
- Create: `crates/koji-core/src/geometry/koji_geojson.rs`
- Modify: `crates/koji-core/src/geometry/mod.rs` (add `mod koji_geojson;`)
- Test: inline `#[cfg(test)]`

Note: `geojson` 0.24 ships `TryFrom<&geojson::Geometry> for geo_types::Geometry<f64>` (geo-types feature, already on — existing code uses geo↔geojson). The Feature's `properties` (a `serde_json::Map`) deserializes straight into `KojiMeta` via `serde_json::from_value`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod inbound_tests {
    use super::*;
    use crate::Mode;

    #[test]
    fn feature_becomes_kojigeometry_with_meta() {
        let f: geojson::Feature = serde_json::from_value(serde_json::json!({
            "type": "Feature",
            "geometry": { "type": "Point", "coordinates": [1.0, 2.0] },
            "properties": { "id": 5, "name": "x", "mode": "quest", "extra_key": 9 }
        })).unwrap();

        let g = KojiGeometry::try_from(f).unwrap();
        assert!(matches!(g.geometry, geo::Geometry::Point(_)));
        assert_eq!(g.meta.id, Some(5));
        assert_eq!(g.meta.mode, Mode::Quest);
        assert_eq!(g.meta.extra.get("extra_key").unwrap(), 9);
    }

    #[test]
    fn featurecollection_becomes_collection() {
        let fc: geojson::FeatureCollection = serde_json::from_value(serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                { "type": "Feature", "geometry": {"type":"Point","coordinates":[0.0,0.0]}, "properties": null },
                { "type": "Feature", "geometry": {"type":"Point","coordinates":[1.0,1.0]}, "properties": null }
            ]
        })).unwrap();

        let c = KojiGeometryCollection::try_from(fc).unwrap();
        assert_eq!(c.items.len(), 2);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_geojson::inbound_tests`
Expected: FAIL — `TryFrom` impls missing.

- [ ] **Step 3: Write the implementation**

```rust
//! Edge conversions between geojson and the KojiGeometry types. geo-types stays
//! canonical; geojson is an edge format only.

use geo::Geometry;

use super::{KojiGeometry, KojiGeometryCollection, KojiMeta};

/// Error when a geojson Feature has no geometry or a non-convertible one.
#[derive(Debug)]
pub enum KojiGeojsonError {
    MissingGeometry,
    Convert(geojson::Error),
}

impl std::fmt::Display for KojiGeojsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KojiGeojsonError::MissingGeometry => write!(f, "feature has no geometry"),
            KojiGeojsonError::Convert(e) => write!(f, "geojson->geo conversion failed: {e}"),
        }
    }
}
impl std::error::Error for KojiGeojsonError {}

impl TryFrom<geojson::Feature> for KojiGeometry {
    type Error = KojiGeojsonError;

    fn try_from(f: geojson::Feature) -> Result<Self, Self::Error> {
        let gj = f.geometry.ok_or(KojiGeojsonError::MissingGeometry)?;
        let geometry =
            Geometry::<f64>::try_from(&gj).map_err(KojiGeojsonError::Convert)?;

        // Properties (a serde_json object) deserialize directly into KojiMeta;
        // typed keys populate fields, the rest land in `extra`.
        let meta: KojiMeta = match f.properties {
            Some(props) => serde_json::from_value(serde_json::Value::Object(props))
                .unwrap_or_default(),
            None => KojiMeta::default(),
        };
        Ok(KojiGeometry { geometry, meta })
    }
}

impl TryFrom<geojson::FeatureCollection> for KojiGeometryCollection {
    type Error = KojiGeojsonError;

    fn try_from(fc: geojson::FeatureCollection) -> Result<Self, Self::Error> {
        let items = fc
            .features
            .into_iter()
            .map(KojiGeometry::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(KojiGeometryCollection::new(items))
    }
}
```

Add `mod koji_geojson;` to `crates/koji-core/src/geometry/mod.rs` (and `pub use koji_geojson::KojiGeojsonError;`).

If `Geometry::<f64>::try_from(&gj)` does not resolve, confirm the exact geojson 0.24 entry point with `cargo doc -p geojson --open` (candidates: `geo_types::Geometry::try_from(&geojson::Geometry)` or `(&gj.value).try_into()`); the geo-types feature is already enabled workspace-wide.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_geojson::inbound_tests`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_geojson.rs crates/koji-core/src/geometry/mod.rs
git commit -m "feat(geometry): geojson -> KojiGeometry/Collection inbound conversions"
```

---

### Task 6: KojiGeometry / Collection → geojson (outbound: Feature/FC/GeometryCollection)

**Files:**
- Modify: `crates/koji-core/src/geometry/koji_geojson.rs` (append outbound impls + tests)

Note: `geojson::Value` has `From<&geo_types::Geometry<f64>>` (geo-types feature). `KojiMeta` serializes to a `serde_json` object via `serde_json::to_value`, which becomes the Feature `properties`.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod outbound_tests {
    use super::*;
    use crate::{KojiMeta, Mode};
    use geo::Point;

    fn sample() -> KojiGeometry {
        KojiGeometry::new(Point::new(1.0, 2.0)).with_meta(KojiMeta {
            id: Some(5),
            mode: Mode::Quest,
            ..Default::default()
        })
    }

    #[test]
    fn element_to_feature_carries_props() {
        let f = geojson::Feature::from(&sample());
        assert!(f.geometry.is_some());
        let props = f.properties.unwrap();
        assert_eq!(props.get("id").unwrap(), 5);
        assert_eq!(props.get("mode").unwrap(), "quest");
    }

    #[test]
    fn collection_to_featurecollection() {
        let c = KojiGeometryCollection::new(vec![sample(), sample()]);
        let fc = geojson::FeatureCollection::from(&c);
        assert_eq!(fc.features.len(), 2);
    }

    #[test]
    fn collection_to_geometrycollection_is_property_less() {
        let c = KojiGeometryCollection::new(vec![sample(), sample()]);
        let g = geojson::Geometry::from(&c);
        assert!(matches!(g.value, geojson::Value::GeometryCollection(ref v) if v.len() == 2));
    }

    #[test]
    fn feature_roundtrip_is_idempotent() {
        let f0 = geojson::Feature::from(&sample());
        let g = KojiGeometry::try_from(f0.clone()).unwrap();
        let f1 = geojson::Feature::from(&g);
        assert_eq!(f0.geometry, f1.geometry);
        assert_eq!(f0.properties, f1.properties);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-core koji_geojson::outbound_tests`
Expected: FAIL — `From` impls missing.

- [ ] **Step 3: Write the implementation**

Append to `crates/koji-core/src/geometry/koji_geojson.rs`:

```rust
impl From<&KojiGeometry> for geojson::Feature {
    fn from(g: &KojiGeometry) -> Self {
        let value = geojson::Value::from(&g.geometry);
        let properties = match serde_json::to_value(&g.meta) {
            Ok(serde_json::Value::Object(map)) if !map.is_empty() => Some(map),
            _ => None,
        };
        geojson::Feature {
            bbox: None,
            geometry: Some(geojson::Geometry::new(value)),
            id: g.meta.id.map(|i| geojson::feature::Id::Number(i.into())),
            properties,
            foreign_members: None,
        }
    }
}

impl From<&KojiGeometryCollection> for geojson::FeatureCollection {
    fn from(c: &KojiGeometryCollection) -> Self {
        geojson::FeatureCollection {
            bbox: None,
            features: c.items.iter().map(geojson::Feature::from).collect(),
            foreign_members: None,
        }
    }
}

impl From<&KojiGeometryCollection> for geojson::Geometry {
    /// Property-less `GeometryCollection` output (replaces the old bare
    /// `[Geometry]` array). Metadata is intentionally dropped — this is the
    /// property-less multi-geometry format.
    fn from(c: &KojiGeometryCollection) -> Self {
        let geometries = c
            .items
            .iter()
            .map(|g| geojson::Geometry::new(geojson::Value::from(&g.geometry)))
            .collect();
        geojson::Geometry::new(geojson::Value::GeometryCollection(geometries))
    }
}
```

Confirm the `geojson::feature::Id` and `geojson::Feature` field names against `cargo doc -p geojson`; if `Id::Number` takes a `serde_json::Number`, use `geojson::feature::Id::Number(serde_json::Number::from(i))`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-core koji_geojson::outbound_tests`
Expected: PASS (4 tests).

- [ ] **Step 5: Commit**

```bash
git add crates/koji-core/src/geometry/koji_geojson.rs
git commit -m "feat(geometry): KojiGeometry/Collection -> geojson Feature/FC/GeometryCollection"
```

---

### Task 7: DB `geofence::Model` → `KojiGeometry` (inbound, additive)

**Files:**
- Modify: `crates/koji-db/src/db/geofence.rs` (add inherent method `to_koji_geometry`)
- Test: inline `#[cfg(test)]` in `crates/koji-db/src/db/geofence.rs` (pure mapping; no DB connection)

Note: the geofence `Model` already holds `geometry: Json`, `mode: <enum>`, `id: u32`, `name: String`, `parent: Option<u32>`, `geo_type: String`. The existing code parses geometry JSON via `geojson::Geometry::from_json_value`. This method is **additive** — `Model::to_feature` is untouched.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod to_koji_tests {
    use super::*;
    use koji_core::Mode;

    #[test]
    fn model_maps_to_kojigeometry() {
        let model = Model {
            id: 42,
            name: "Boulder".to_string(),
            parent: Some(7),
            geometry: serde_json::json!({ "type": "Point", "coordinates": [1.0, 2.0] }),
            geo_type: "Point".to_string(),
            mode: Mode::Fort.into(), // db Mode via bridge
            // ...remaining fields filled with their Default / sentinel values...
            ..test_model_defaults()
        };

        let kg = model.to_koji_geometry().unwrap();
        assert!(matches!(kg.geometry, geo::Geometry::Point(_)));
        assert_eq!(kg.meta.id, Some(42));
        assert_eq!(kg.meta.name.as_deref(), Some("Boulder"));
        assert_eq!(kg.meta.parent_id, Some(7));
        assert_eq!(kg.meta.mode, Mode::Fort);
    }
}
```

(If constructing a full `Model` literal is unwieldy, add a small `#[cfg(test)] fn test_model_defaults() -> Model` helper in this module that fills timestamps/`dragonite_area_id` with defaults; do NOT add test helpers to non-test code.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p koji-db to_koji_tests`
Expected: FAIL — `to_koji_geometry` not found.

- [ ] **Step 3: Write the implementation**

Add an inherent method on `geofence::Model` (near `to_feature`):

```rust
impl Model {
    /// Additive: map this row to a `KojiGeometry` (the Phase 1 target type).
    /// Does not replace `to_feature`. The geometry JSON is parsed to geo-types;
    /// the DB `mode` enum bridges to `koji_core::Mode`.
    pub fn to_koji_geometry(&self) -> Result<koji_core::KojiGeometry, ModelError> {
        let gj = geojson::Geometry::from_json_value(self.geometry.clone())
            .map_err(|e| ModelError::Custom(format!("[GEOMETRY]: {e}")))?;
        let geometry = geo::Geometry::<f64>::try_from(&gj)
            .map_err(|e| ModelError::Custom(format!("[GEOMETRY]: {e}")))?;

        let meta = koji_core::KojiMeta {
            id: Some(self.id),
            name: Some(self.name.clone()),
            mode: koji_core::Mode::from(self.mode),
            parent_id: self.parent,
            ancestors: Vec::new(),
            extra: serde_json::Map::new(),
        };
        Ok(koji_core::KojiGeometry { geometry, meta })
    }
}
```

Confirm the exact `ModelError` variant/constructor used elsewhere in this file (the plugin_config work used `ModelError::Custom`); match it. Confirm `geojson::Geometry::from_json_value` is the symbol already in use in `to_feature`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p koji-db to_koji_tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/koji-db/src/db/geofence.rs
git commit -m "feat(geometry): additive geofence Model -> KojiGeometry conversion"
```

---

### Task 8: Phase 1 verification gate

**Files:** none (verification only)

- [ ] **Step 1: Full workspace build + clippy + fmt + tests (parallel)**

Run (single batch):
- `cargo build --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all -- --check`
- `cargo test -p koji-core && cargo test -p koji-db`

Expected: all green. New types compile, all new unit tests pass, no clippy regressions, fmt clean. The existing machinery (`FenceType`, `GeoFormats`, `Model::to_feature`, etc.) is untouched and still compiles.

- [ ] **Step 2: Confirm additivity**

Run: `git grep -n "FenceType\|GeoFormats\|FeatureCtx" crates/koji-core/src/lib.rs`
Expected: still present (this phase deletes nothing). If any are gone, a task overreached — revert the deletion; deletion is Phase 2.

- [ ] **Step 3: Commit (if any fmt/clippy fixups were needed)**

```bash
git add -A && git commit -m "chore(geometry): Phase 1 verification fixups" || echo "nothing to commit"
```

---

## Self-review

**Spec coverage (Phase 1 slice of `2026-06-13-koji-geometry-universal-type-design.md`):**
- §5 `KojiGeometry` / `KojiMeta` / `KojiGeometryCollection` → Tasks 3, 4. ✓
- §4.5 `Mode {Unset,Pokemon,Fort,Quest}` + legacy mapping → Tasks 1, 2. ✓
- §6 inbound geojson→Koji, outbound Koji→geojson + `GeometryCollection` → Tasks 5, 6. ✓
- §11 DB read `Model → KojiGeometry` → Task 7. ✓
- §13 round-trip property test (geojson→Koji→geojson idempotent) → Task 6 Step 1. ✓
- **Deferred by design (flagged):** the `Type→Mode` column migration (→ Phase 2, breaking); s2 bridge + Poracle/Text/SQL/array/struct outbound adapters (→ Phase 1B); DB write `KojiGeometry → Model` (→ Phase 2 rewire). The `route::Model → KojiGeometry` inbound mirror is intentionally not duplicated here (same shape as Task 7; lands with the route rewire/1B to avoid an unused method).

**Placeholder scan:** no TBD/TODO; every code step shows complete code or a precise API-confirmation pointer with the exact `cargo doc`/`git grep` command.

**Type consistency:** `Mode` (core) ↔ `Mode` (db) bridged in Task 2 and consumed in Task 7. `KojiMeta` fields (`id: Option<u32>`, `mode: Mode`, `parent_id: Option<u32>`, `ancestors: Vec<String>`, `extra`) are identical across Tasks 3, 5, 6, 7. `KojiGeometry { geometry, meta }` / `KojiGeometryCollection { items, bbox }` consistent across Tasks 4–7. `to_koji_geometry` named identically in Task 7 test + impl.
