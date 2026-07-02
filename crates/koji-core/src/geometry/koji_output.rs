//! Outbound format adapters for `KojiGeometryCollection`.
//!
//! These are **matrix-independent**: each adapter reads coordinates from the
//! item's `geo::Geometry` and metadata from its `KojiMeta`, never delegating to
//! the geojson `To*` matrix (deleted in Phase 2). Correctness is pinned by tests
//! against goldens frozen from that matrix while it still existed.
//!
//! Coordinate convention: the matrix emitted `PointArray = [lat, lon]` (y, x).
//! `geo::Geometry` stores `[x, y]` (lon, lat), so every extraction flips to
//! `[coord.y, coord.x]`.

use geo::{Geometry, LineString, Polygon};
use crate::Precision;

use super::{MultiStruct, MultiVec, PointStruct, Poracle, SingleStruct, SingleVec};
use crate::UnknownId;
use crate::geometry::{KojiGeometry, KojiGeometryCollection};

/// Flatten one closed/open ring (`LineString`) into the matrix's `[lat, lon]`
/// point order, matching `Geometry::to_single_vec`'s `[point[1], point[0]]`.
fn ring_points(line: &LineString<Precision>) -> Vec<[Precision; 2]> {
    line.coords().map(|c| [c.y, c.x]).collect()
}

/// One polygon → a single flat `[lat, lon]` vec across exterior + interior
/// rings, matching the matrix `Polygon` branch (which iterates every ring of
/// the geojson polygon and flattens them into one inner vec).
fn polygon_points(poly: &Polygon<Precision>) -> Vec<[Precision; 2]> {
    let mut out = ring_points(poly.exterior());
    for interior in poly.interiors() {
        out.extend(ring_points(interior));
    }
    out
}

/// The flat `[lat, lon]` coords of a geometry treated as a single group, matching
/// the matrix's single-geometry coordinate path (Polygon flattens rings;
/// MultiPolygon flattens every ring of every polygon; points/lines map directly).
fn single_group(geom: &Geometry<Precision>) -> Vec<[Precision; 2]> {
    match geom {
        Geometry::Point(p) => vec![[p.y(), p.x()]],
        Geometry::MultiPoint(mp) => mp.iter().map(|p| [p.y(), p.x()]).collect(),
        Geometry::Line(l) => vec![[l.start.y, l.start.x], [l.end.y, l.end.x]],
        Geometry::LineString(ls) => ring_points(ls),
        Geometry::MultiLineString(mls) => mls.iter().flat_map(ring_points).collect(),
        Geometry::Polygon(poly) => polygon_points(poly),
        Geometry::MultiPolygon(mp) => mp.iter().flat_map(polygon_points).collect(),
        Geometry::Rect(r) => polygon_points(&r.to_polygon()),
        Geometry::Triangle(t) => polygon_points(&t.to_polygon()),
        Geometry::GeometryCollection(gc) => gc.iter().flat_map(single_group).collect(),
    }
}

/// Append the first point if it doesn't already close the ring, matching the
/// matrix `SingleVec::ensure_first_last`. The matrix applies this when going
/// `SingleVec`/`MultiVec` → struct forms (`to_single_struct` on a `SingleVec`),
/// so the struct adapters must reproduce it.
fn ensure_first_last(mut points: Vec<[Precision; 2]>) -> Vec<[Precision; 2]> {
    if points.is_empty() {
        return points;
    }
    if points[0] != points[points.len() - 1] {
        points.push(points[0]);
    }
    points
}

/// `[lat, lon]` → `PointStruct`, matching `From<PointArray> for PointStruct`
/// (`lat = p[0]`, `lon = p[1]`).
fn to_point_struct(p: [Precision; 2]) -> PointStruct {
    PointStruct {
        lat: p[0],
        lon: p[1],
    }
}

/// Project a `SingleVec` (`[lat, lon]` points) to a `geo::MultiPoint`, exactly
/// reproducing the matrix `SingleVec::to_feature(CirclePokemon)` geometry: the
/// ring is closed (`ensure_first_last`), each `[lat, lon]` becomes
/// `Point(x = lon, y = lat)`, then *consecutive* duplicates are collapsed
/// (`remove_repeated_points`). The matrix's `multi_point()` ran the dedupe; the
/// `ensure_first_last` came from `to_single_vec()` inside `to_multi_vec()`.
///
/// This is the canonical home for what the v2 calc core duplicated as
/// `centers_to_multipoint`; both the calc cores and the bootstrap edge use it so
/// the MultiPoint cluster-center output is single-sourced and matrix-free.
pub fn single_vec_to_multipoint(centers: &SingleVec) -> geo::MultiPoint<Precision> {
    use geo::RemoveRepeatedPoints;

    let mut points = centers.clone();
    if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
        && first != last
    {
        points.push(first);
    }
    let mp: geo::MultiPoint<Precision> = points
        .into_iter()
        .map(|[lat, lon]| geo::Point::new(lon, lat))
        .collect();
    mp.remove_repeated_points()
}

/// Build the labeled-less `geojson::Feature` a bootstrap edge emits for a routed
/// `SingleVec`: the [`single_vec_to_multipoint`] geometry wrapped via the Phase 1
/// outbound `From<&KojiGeometry>`. The caller attaches its `__name`/`__mode`/…
/// properties. Replaces the matrix `SingleVec::to_feature(CirclePokemon)`; the
/// (discarded-downstream) bbox fields the matrix set are intentionally omitted.
pub fn single_vec_to_multipoint_feature(centers: &SingleVec) -> geojson::Feature {
    let kg = KojiGeometry::new(single_vec_to_multipoint(centers));
    geojson::Feature::from(&kg)
}

/// Project a `SingleVec` (`[lat, lon]` points) to a single-ring `geo::Polygon`,
/// reproducing the (now-deleted) matrix single-feature polygon geometry (no fence
/// type → the `polygon()` branch): the ring is closed (`ensure_first_last`) and
/// each `[lat, lon]` becomes `(x = lon, y = lat)`.
pub fn single_vec_to_polygon(centers: &SingleVec) -> geo::Polygon<Precision> {
    let mut points = centers.clone();
    if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
        && first != last
    {
        points.push(first);
    }
    let ring: geo::LineString<Precision> = points
        .into_iter()
        .map(|[lat, lon]| geo::coord! { x: lon, y: lat })
        .collect();
    geo::Polygon::new(ring, vec![])
}

/// The unlabeled `geojson::Feature` a bootstrap *plugin* edge emits for a routed
/// `SingleVec`: the [`single_vec_to_polygon`] geometry via the Phase 1 outbound
/// `From<&KojiGeometry>`. Replaces the (now-deleted) matrix single-feature polygon
/// projection.
pub fn single_vec_to_polygon_feature(centers: &SingleVec) -> geojson::Feature {
    let kg = KojiGeometry::new(single_vec_to_polygon(centers));
    geojson::Feature::from(&kg)
}

impl KojiGeometryCollection {
    /// `[min_lon, min_lat, max_lon, max_lat]` (trimmed to 6 decimals) over every
    /// item's points — the geojson `bbox` member projection.
    pub fn geojson_bbox(&self) -> Option<Vec<Precision>> {
        super::KojiBbox::from_points(&self.to_single_vec()).map(|b| b.trim(6).to_geojson_bbox_vec())
    }

    /// All items' coordinates flattened into one `[lat, lon]` vec.
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `single_vec` path
    /// (`to_multi_vec().flatten()` — grouping is erased, so this is every item's
    /// `single_group` concatenated).
    pub fn to_single_vec(&self) -> SingleVec {
        self.items
            .iter()
            .flat_map(|item| single_group(&item.geometry))
            .collect()
    }

    /// Coordinates grouped **one inner vec per item** (NOT per ring/sub-geometry).
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `multi_vec` path,
    /// which is `features.map(|feat| feat.to_single_vec())` — exactly one flattened
    /// group per feature. A MultiPolygon item flattens all polygons into one group;
    /// a GeometryCollection item flattens all sub-geometries into one group.
    pub fn to_multi_vec(&self) -> MultiVec {
        self.items
            .iter()
            .map(|item| single_group(&item.geometry))
            .collect()
    }

    /// All coordinates as one flat `PointStruct` vec.
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `single_struct` path.
    /// The matrix flattens to a single vec, then `ensure_first_last`s it before
    /// the struct map — so this closes the *flattened* ring, not each group.
    pub fn to_single_struct(&self) -> SingleStruct {
        let flat: SingleVec = self
            .items
            .iter()
            .flat_map(|item| single_group(&item.geometry))
            .collect();
        ensure_first_last(flat)
            .into_iter()
            .map(to_point_struct)
            .collect()
    }

    /// Coordinates as `PointStruct` groups, one per item (see `to_multi_vec`).
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `multi_struct` path.
    /// The matrix `ensure_first_last`s each group before the struct map.
    pub fn to_multi_struct(&self) -> MultiStruct {
        self.to_multi_vec()
            .into_iter()
            .map(|group| {
                ensure_first_last(group)
                    .into_iter()
                    .map(to_point_struct)
                    .collect()
            })
            .collect()
    }

    /// Plain-text coordinate dump. Covers both `response.rs` parameterizations:
    /// Text (`",", "\n", true`) and AltText (`" ", ",", false`).
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `text` path with the
    /// same `(sep_1, sep_2, poly_sep)`, i.e. `MultiVec` text over the groups.
    /// Coordinates use the default
    /// `Precision` `Display` (no fixed precision), `lat` then `sep_1` then `lon`.
    pub fn to_text(&self, sep_1: &str, sep_2: &str, poly_sep: bool) -> String {
        let groups = self.to_multi_vec();
        if groups.is_empty() {
            // The matrix `MultiVec::to_text` underflows on empty input; an empty
            // collection has no coordinates, so emit nothing.
            return String::new();
        }
        let more_than_1 = groups.len() > 1;
        let last = groups.len() - 1;
        groups
            .into_iter()
            .enumerate()
            .map(|(i, group)| {
                format!(
                    "{}{}{}{}",
                    if i != 0 && poly_sep { "\n" } else { "" },
                    if more_than_1 && poly_sep {
                        format!("[Geofence {}]\n", i + 1)
                    } else {
                        String::new()
                    },
                    group_to_text(&group, sep_1, sep_2),
                    if i == last { "" } else { sep_2 }
                )
            })
            .collect()
    }

    /// `SELECT … WHERE …` over each polygonal item: bbox range + `ST_CONTAINS`
    /// against the embedded geojson geometry. Non-polygon items contribute no
    /// clause but still advance the item index (matching the matrix, where the
    /// `\nOR` joiner keys off the FeatureCollection index, not the emit count).
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `sql` path.
    pub fn to_sql(&self) -> String {
        let mut clauses = String::new();
        for (i, item) in self.items.iter().enumerate() {
            let is_poly = matches!(
                item.geometry,
                Geometry::Polygon(_) | Geometry::MultiPolygon(_)
            );
            if !is_poly {
                continue;
            }
            let bbox = super::KojiBbox::from_points(&single_group(&item.geometry))
                .map(|b| b.trim(6).to_geojson_bbox())
                .unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let geo = geojson_geometry_closed(&item.geometry);
            clauses = format!(
                "{}{} (\n\tlon BETWEEN {} AND {}\n\tAND lat BETWEEN {} AND {}\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)",
                clauses,
                if i == 0 { "" } else { "\nOR" },
                bbox[0],
                bbox[2],
                bbox[1],
                bbox[3],
                geo
            );
        }
        format!("SELECT * FROM {{database.table}} WHERE{}", clauses)
    }

    /// One `Poracle` per item. Metadata is read from each item's `KojiMeta`
    /// (serialized to the same property map the oracle reads back), and
    /// `path`/`multipath` come from the item's `geo::Geometry`.
    ///
    /// Parity oracle: the geojson `FeatureCollection` matrix `poracle_vec` path.
    /// The oracle reads geojson `properties` that Phase 1 wrote from `KojiMeta`,
    /// so serializing the meta and applying the oracle's key reads is exact.
    pub fn to_poracle_vec(&self) -> Vec<Poracle> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| item_poracle(i, item))
            .collect()
    }
}

/// Build a single `Poracle` from one item at index `i`, matching the oracle's
/// per-feature read logic (`FeatureCollection::to_poracle_vec`) field-for-field.
fn item_poracle(i: usize, item: &KojiGeometry) -> Poracle {
    // The oracle reads from geojson `properties`, which Phase 1 produced via
    // `serde_json::to_value(&KojiMeta)`. Reproduce that exact map, then read the
    // same keys — so typed fields (`id`, `name`) and `extra` keys
    // (`color`, `group`, `parent`, `description`, `displayInMatches`,
    // `userSelectable`) map identically.
    let props: serde_json::Map<String, serde_json::Value> = match serde_json::to_value(&item.meta) {
        Ok(serde_json::Value::Object(map)) => map,
        _ => serde_json::Map::new(),
    };
    let str_prop = |key: &str| -> Option<String> {
        props.get(key).map(|v| v.as_str().unwrap_or("").to_string())
    };

    let mut poracle = Poracle::default();

    if props.contains_key("name") {
        poracle.name = str_prop("name");
    }
    poracle.id = Some(UnknownId::Number(if let Some(v) = props.get("id") {
        v.as_f64().unwrap_or((i + 1) as Precision) as u32
    } else {
        (i + 1) as u32
    }));
    if props.contains_key("color") {
        poracle.color = str_prop("color");
    }
    if props.contains_key("description") {
        poracle.description = str_prop("description");
    }
    if props.contains_key("group") {
        poracle.group = str_prop("group");
    } else if props.contains_key("parent") {
        poracle.group = str_prop("parent");
    }
    poracle.display_in_matches = Some(
        props
            .get("displayInMatches")
            .map(|v| v.as_bool().unwrap_or(true))
            .unwrap_or(true),
    );
    poracle.user_selectable = Some(
        props
            .get("userSelectable")
            .map(|v| v.as_bool().unwrap_or(true))
            .unwrap_or(true),
    );

    // Geometry → path (single polygon) / multipath (multi). Mirrors the oracle's
    // match on the geojson geometry value. Point/MultiPoint/LineString leave the
    // `Poracle::default` `path` (`Some(vec![])`) and no multipath.
    match &item.geometry {
        Geometry::Polygon(poly) => poracle.path = Some(polygon_points(poly)),
        Geometry::MultiPolygon(mp) => {
            let multipath: MultiVec = mp.iter().map(polygon_points).collect();
            if !multipath.is_empty() {
                poracle.multipath = Some(multipath);
            }
        }
        Geometry::GeometryCollection(gc) => {
            let mut multipath: MultiVec = vec![];
            for g in gc.iter() {
                if let Geometry::Polygon(poly) = g {
                    let value = polygon_points(poly);
                    if !value.is_empty() {
                        multipath.push(value);
                    }
                }
            }
            if !multipath.is_empty() {
                poracle.multipath = Some(multipath);
            }
        }
        _ => {}
    }

    poracle
}

/// The item's geojson `Geometry`, ring-closed, rendered to its JSON string — the
/// exact value the oracle embeds in the SQL (`geojson::GeometryValue::from(&geo)` is the
/// edge geo→geojson conversion, then ring-closure inlined from the matrix
/// `EnsurePoints for Geometry`). Not a `To*` matrix call.
fn geojson_geometry_closed(geom: &Geometry<Precision>) -> String {
    let mut value = geojson::GeometryValue::from(geom);
    // Inline `EnsurePoints::ensure_first_last`: close each ring whose last point
    // differs from its first on *both* axes (matrix uses `&&`).
    let close_ring = |ring: &mut Vec<geojson::Position>| {
        let needs_close = match (ring.first(), ring.last()) {
            (Some(first), Some(last)) => last[0] != first[0] && last[1] != first[1],
            _ => false,
        };
        if needs_close {
            let first = ring[0].clone();
            ring.push(first);
        }
    };
    match &mut value {
        geojson::GeometryValue::Polygon { coordinates: rings } => rings.iter_mut().for_each(close_ring),
        geojson::GeometryValue::MultiPolygon { coordinates: polys } => polys
            .iter_mut()
            .flat_map(|p| p.iter_mut())
            .for_each(close_ring),
        _ => {}
    }
    geojson::Geometry::new(value).to_string()
}

/// One group → text, matching `SingleVec::to_text`: the inter-point separator is
/// suppressed on the final point of the group.
fn group_to_text(group: &[[Precision; 2]], sep_1: &str, sep_2: &str) -> String {
    let last = if group.is_empty() { 0 } else { group.len() - 1 };
    group
        .iter()
        .enumerate()
        .map(|(i, pt)| point_to_text(pt, sep_1, if i == last { "" } else { sep_2 }))
        .collect()
}

/// One `[lat, lon]` point → text, matching `PointArray::to_text`:
/// `format!("{}{}{}{}", lat, sep_1, lon, sep_2)` with default `Precision` `Display`.
fn point_to_text(pt: &[Precision; 2], sep_1: &str, sep_2: &str) -> String {
    format!("{}{}{}{}", pt[0], sep_1, pt[1], sep_2)
}

#[cfg(test)]
mod tests {
    use crate::{KojiGeometry, KojiGeometryCollection, KojiMeta, Mode, MultiVec, SingleVec};
    use geo::{
        Geometry, GeometryCollection, LineString, MultiPoint, MultiPolygon, Point, Polygon, coord,
    };

    /// A representative collection: one polygon + one multipoint, with metadata.
    fn sample() -> KojiGeometryCollection {
        let poly = Polygon::new(
            LineString::from(vec![
                coord! {x:0.0,y:0.0},
                coord! {x:2.0,y:0.0},
                coord! {x:2.0,y:2.0},
                coord! {x:0.0,y:0.0},
            ]),
            vec![],
        );
        let mp = MultiPoint::from(vec![Point::new(5.0, 6.0), Point::new(7.0, 8.0)]);
        KojiGeometryCollection::new(vec![
            KojiGeometry::new(poly).with_meta(KojiMeta {
                name: Some("a".into()),
                mode: Mode::Fort,
                ..Default::default()
            }),
            KojiGeometry::new(mp).with_meta(KojiMeta {
                name: Some("b".into()),
                ..Default::default()
            }),
        ])
    }

    /// An adversarial collection that exercises every adapter branch the
    /// axis-symmetric `sample()` leaves untouched:
    ///
    /// 1. Asymmetric polygon (x∈[0,10], y∈[0,4]) **with a hole** — the asymmetry
    ///    pins the lon/lat axis assignment in `to_sql`'s bbox (a swap would
    ///    surface), and the interior ring pins `polygon_points`'s exterior +
    ///    interior flattening. Its `KojiMeta` carries non-default poracle fields
    ///    (`color`/`description`/`group`/`displayInMatches:false`/
    ///    `userSelectable:false`) so the poracle field-reads are actually driven
    ///    off the wire, not left at defaults.
    /// 2. `MultiPolygon` of two distinct polygons — both polygons flattened into
    ///    one `to_multi_vec` group (`single_group`), split per-polygon in poracle
    ///    `multipath`, and a single `to_sql` clause over the union bbox.
    /// 3. `GeometryCollection` of a `Polygon` + a `MultiPoint` — the GC branch in
    ///    `single_group` (polygon + multipoint flattened into one group) / poracle
    ///    `multipath` (polygon-only filter) / `to_sql` (GC is non-polygon →
    ///    contributes no clause but advances the index).
    ///
    /// A bare `LineString` is intentionally **excluded**: the goldens below were
    /// frozen from the Phase-1-outbound matrix oracle, which dropped bare lines
    /// (`ToSingleVec for geojson::Geometry` only matched
    /// `Polygon`/`MultiPolygon`/`Point`/`MultiPoint`). The adapters' `single_group`
    /// *does* handle `LineString`, so a bare-line case is covered separately rather
    /// than against these matrix-derived goldens.
    fn sample_rich() -> KojiGeometryCollection {
        // 1. asymmetric polygon with a hole + rich poracle metadata.
        let poly_with_hole = Polygon::new(
            LineString::from(vec![
                coord! {x:0.0,y:0.0},
                coord! {x:10.0,y:0.0},
                coord! {x:10.0,y:4.0},
                coord! {x:0.0,y:4.0},
                coord! {x:0.0,y:0.0},
            ]),
            vec![LineString::from(vec![
                coord! {x:2.0,y:1.0},
                coord! {x:4.0,y:1.0},
                coord! {x:4.0,y:3.0},
                coord! {x:2.0,y:3.0},
                coord! {x:2.0,y:1.0},
            ])],
        );
        let extra = serde_json::json!({
            "color": "#00ff00",
            "description": "a rich fence",
            "group": "alpha",
            "displayInMatches": false,
            "userSelectable": false,
        })
        .as_object()
        .unwrap()
        .clone();
        let poly_meta = KojiMeta {
            id: Some(7),
            name: Some("poly".into()),
            mode: Mode::Fort,
            extra,
            ..Default::default()
        };

        // 2. MultiPolygon of two distinct (non-overlapping) polygons.
        let multi_poly = MultiPolygon::new(vec![
            Polygon::new(
                LineString::from(vec![
                    coord! {x:20.0,y:20.0},
                    coord! {x:23.0,y:20.0},
                    coord! {x:23.0,y:22.0},
                    coord! {x:20.0,y:20.0},
                ]),
                vec![],
            ),
            Polygon::new(
                LineString::from(vec![
                    coord! {x:30.0,y:30.0},
                    coord! {x:34.0,y:30.0},
                    coord! {x:34.0,y:33.0},
                    coord! {x:30.0,y:33.0},
                    coord! {x:30.0,y:30.0},
                ]),
                vec![],
            ),
        ]);

        // 3. GeometryCollection: a Polygon + a MultiPoint.
        let gc = GeometryCollection::new_from(vec![
            Polygon::new(
                LineString::from(vec![
                    coord! {x:40.0,y:40.0},
                    coord! {x:42.0,y:40.0},
                    coord! {x:42.0,y:41.0},
                    coord! {x:40.0,y:40.0},
                ]),
                vec![],
            )
            .into(),
            MultiPoint::from(vec![Point::new(50.0, 51.0), Point::new(52.0, 53.0)]).into(),
        ]);

        KojiGeometryCollection::new(vec![
            KojiGeometry::new(poly_with_hole).with_meta(poly_meta),
            KojiGeometry::new(multi_poly).with_meta(KojiMeta {
                name: Some("multi".into()),
                mode: Mode::Fort,
                ..Default::default()
            }),
            // `geo_types::Geometry` has no `From<GeometryCollection>` (unlike the
            // other variants), so build the enum variant explicitly.
            KojiGeometry::new(Geometry::GeometryCollection(gc)).with_meta(KojiMeta {
                name: Some("gc".into()),
                ..Default::default()
            }),
        ])
    }

    /// Parse a JSON golden string into a `serde_json::Value` for comparison.
    fn j(s: &str) -> serde_json::Value {
        serde_json::from_str(s).unwrap()
    }

    /// All seven inherent adapters on the adversarial `sample_rich()`, asserted
    /// against goldens frozen from the (now-deleted) matrix oracle — proving the
    /// matrix-independent extraction generalizes past the axis-symmetric `sample()`
    /// (holes, MultiPolygon, GeometryCollection, asymmetric bbox, non-default
    /// poracle fields). (Bare `LineString` excluded — see `sample_rich`.)
    #[test]
    fn all_adapters_match_golden_on_rich_sample() {
        let c = sample_rich();

        let single_vec: SingleVec = vec![
            [0.0, 0.0],
            [0.0, 10.0],
            [4.0, 10.0],
            [4.0, 0.0],
            [0.0, 0.0],
            [1.0, 2.0],
            [1.0, 4.0],
            [3.0, 4.0],
            [3.0, 2.0],
            [1.0, 2.0],
            [20.0, 20.0],
            [20.0, 23.0],
            [22.0, 23.0],
            [20.0, 20.0],
            [30.0, 30.0],
            [30.0, 34.0],
            [33.0, 34.0],
            [33.0, 30.0],
            [30.0, 30.0],
            [40.0, 40.0],
            [40.0, 42.0],
            [41.0, 42.0],
            [40.0, 40.0],
            [51.0, 50.0],
            [53.0, 52.0],
        ];
        assert_eq!(c.to_single_vec(), single_vec, "single_vec");

        let multi_vec: MultiVec = vec![
            vec![
                [0.0, 0.0],
                [0.0, 10.0],
                [4.0, 10.0],
                [4.0, 0.0],
                [0.0, 0.0],
                [1.0, 2.0],
                [1.0, 4.0],
                [3.0, 4.0],
                [3.0, 2.0],
                [1.0, 2.0],
            ],
            vec![
                [20.0, 20.0],
                [20.0, 23.0],
                [22.0, 23.0],
                [20.0, 20.0],
                [30.0, 30.0],
                [30.0, 34.0],
                [33.0, 34.0],
                [33.0, 30.0],
                [30.0, 30.0],
            ],
            vec![
                [40.0, 40.0],
                [40.0, 42.0],
                [41.0, 42.0],
                [40.0, 40.0],
                [51.0, 50.0],
                [53.0, 52.0],
            ],
        ];
        assert_eq!(c.to_multi_vec(), multi_vec, "multi_vec");

        assert_eq!(
            c.to_sql(),
            "SELECT * FROM {database.table} WHERE (\n\tlon BETWEEN 0 AND 10\n\tAND lat BETWEEN 0 AND 4\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{\"type\":\"Polygon\",\"coordinates\":[[[0.0,0.0],[10.0,0.0],[10.0,4.0],[0.0,4.0],[0.0,0.0]],[[2.0,1.0],[4.0,1.0],[4.0,3.0],[2.0,3.0],[2.0,1.0]]]}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)\nOR (\n\tlon BETWEEN 20 AND 34\n\tAND lat BETWEEN 20 AND 33\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{\"type\":\"MultiPolygon\",\"coordinates\":[[[[20.0,20.0],[23.0,20.0],[23.0,22.0],[20.0,20.0]]],[[[30.0,30.0],[34.0,30.0],[34.0,33.0],[30.0,33.0],[30.0,30.0]]]]}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)",
            "sql"
        );

        assert_eq!(
            c.to_text(",", "\n", true),
            "[Geofence 1]\n0,0\n0,10\n4,10\n4,0\n0,0\n1,2\n1,4\n3,4\n3,2\n1,2\n\n[Geofence 2]\n20,20\n20,23\n22,23\n20,20\n30,30\n30,34\n33,34\n33,30\n30,30\n\n[Geofence 3]\n40,40\n40,42\n41,42\n40,40\n51,50\n53,52",
            "text (Text params)"
        );
        assert_eq!(
            c.to_text(" ", ",", false),
            "0 0,0 10,4 10,4 0,0 0,1 2,1 4,3 4,3 2,1 2,20 20,20 23,22 23,20 20,30 30,30 34,33 34,33 30,30 30,40 40,40 42,41 42,40 40,51 50,53 52",
            "text (AltText params)"
        );

        // `PointStruct`/`Poracle`/`UnknownId` lack `PartialEq`; compare serialized
        // forms (all derive `Serialize`) against the goldens.
        assert_eq!(
            serde_json::to_value(c.to_single_struct()).unwrap(),
            j(
                r#"[{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":10.0},{"lat":4.0,"lon":10.0},{"lat":4.0,"lon":0.0},{"lat":0.0,"lon":0.0},{"lat":1.0,"lon":2.0},{"lat":1.0,"lon":4.0},{"lat":3.0,"lon":4.0},{"lat":3.0,"lon":2.0},{"lat":1.0,"lon":2.0},{"lat":20.0,"lon":20.0},{"lat":20.0,"lon":23.0},{"lat":22.0,"lon":23.0},{"lat":20.0,"lon":20.0},{"lat":30.0,"lon":30.0},{"lat":30.0,"lon":34.0},{"lat":33.0,"lon":34.0},{"lat":33.0,"lon":30.0},{"lat":30.0,"lon":30.0},{"lat":40.0,"lon":40.0},{"lat":40.0,"lon":42.0},{"lat":41.0,"lon":42.0},{"lat":40.0,"lon":40.0},{"lat":51.0,"lon":50.0},{"lat":53.0,"lon":52.0},{"lat":0.0,"lon":0.0}]"#
            ),
            "single_struct"
        );
        assert_eq!(
            serde_json::to_value(c.to_multi_struct()).unwrap(),
            j(
                r#"[[{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":10.0},{"lat":4.0,"lon":10.0},{"lat":4.0,"lon":0.0},{"lat":0.0,"lon":0.0},{"lat":1.0,"lon":2.0},{"lat":1.0,"lon":4.0},{"lat":3.0,"lon":4.0},{"lat":3.0,"lon":2.0},{"lat":1.0,"lon":2.0},{"lat":0.0,"lon":0.0}],[{"lat":20.0,"lon":20.0},{"lat":20.0,"lon":23.0},{"lat":22.0,"lon":23.0},{"lat":20.0,"lon":20.0},{"lat":30.0,"lon":30.0},{"lat":30.0,"lon":34.0},{"lat":33.0,"lon":34.0},{"lat":33.0,"lon":30.0},{"lat":30.0,"lon":30.0},{"lat":20.0,"lon":20.0}],[{"lat":40.0,"lon":40.0},{"lat":40.0,"lon":42.0},{"lat":41.0,"lon":42.0},{"lat":40.0,"lon":40.0},{"lat":51.0,"lon":50.0},{"lat":53.0,"lon":52.0},{"lat":40.0,"lon":40.0}]]"#
            ),
            "multi_struct"
        );
        assert_eq!(
            serde_json::to_value(c.to_poracle_vec()).unwrap(),
            j(
                r##"[{"id":7,"name":"poly","color":"#00ff00","group":"alpha","description":"a rich fence","userSelectable":false,"displayInMatches":false,"path":[[0.0,0.0],[0.0,10.0],[4.0,10.0],[4.0,0.0],[0.0,0.0],[1.0,2.0],[1.0,4.0],[3.0,4.0],[3.0,2.0],[1.0,2.0]]},{"id":2,"name":"multi","userSelectable":true,"displayInMatches":true,"path":[],"multipath":[[[20.0,20.0],[20.0,23.0],[22.0,23.0],[20.0,20.0]],[[30.0,30.0],[30.0,34.0],[33.0,34.0],[33.0,30.0],[30.0,30.0]]]},{"id":3,"name":"gc","userSelectable":true,"displayInMatches":true,"path":[],"multipath":[[[40.0,40.0],[40.0,42.0],[41.0,42.0],[40.0,40.0]]]}]"##
            ),
            "poracle_vec"
        );
    }

    #[test]
    fn single_vec_matches_golden() {
        let c = sample();
        let golden: SingleVec = vec![
            [0.0, 0.0],
            [0.0, 2.0],
            [2.0, 2.0],
            [0.0, 0.0],
            [6.0, 5.0],
            [8.0, 7.0],
        ];
        assert_eq!(c.to_single_vec(), golden);
    }

    #[test]
    fn multi_vec_matches_golden() {
        let c = sample();
        let golden: MultiVec = vec![
            vec![[0.0, 0.0], [0.0, 2.0], [2.0, 2.0], [0.0, 0.0]],
            vec![[6.0, 5.0], [8.0, 7.0]],
        ];
        assert_eq!(c.to_multi_vec(), golden);
    }

    #[test]
    fn single_struct_matches_golden() {
        let c = sample();
        // `PointStruct` has no `PartialEq`; compare its serialized form (it derives
        // `Serialize`) against the golden.
        assert_eq!(
            serde_json::to_value(c.to_single_struct()).unwrap(),
            j(
                r#"[{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":2.0},{"lat":2.0,"lon":2.0},{"lat":0.0,"lon":0.0},{"lat":6.0,"lon":5.0},{"lat":8.0,"lon":7.0},{"lat":0.0,"lon":0.0}]"#
            )
        );
    }

    #[test]
    fn multi_struct_matches_golden() {
        let c = sample();
        assert_eq!(
            serde_json::to_value(c.to_multi_struct()).unwrap(),
            j(
                r#"[[{"lat":0.0,"lon":0.0},{"lat":0.0,"lon":2.0},{"lat":2.0,"lon":2.0},{"lat":0.0,"lon":0.0}],[{"lat":6.0,"lon":5.0},{"lat":8.0,"lon":7.0},{"lat":6.0,"lon":5.0}]]"#
            )
        );
    }

    #[test]
    fn text_matches_golden_both_param_sets() {
        let c = sample();
        assert_eq!(
            c.to_text(",", "\n", true),
            "[Geofence 1]\n0,0\n0,2\n2,2\n0,0\n\n[Geofence 2]\n6,5\n8,7"
        );
        assert_eq!(c.to_text(" ", ",", false), "0 0,0 2,2 2,0 0,6 5,8 7");
    }

    #[test]
    fn sql_matches_golden() {
        let c = sample();
        assert_eq!(
            c.to_sql(),
            "SELECT * FROM {database.table} WHERE (\n\tlon BETWEEN 0 AND 2\n\tAND lat BETWEEN 0 AND 2\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{\"type\":\"Polygon\",\"coordinates\":[[[0.0,0.0],[2.0,0.0],[2.0,2.0],[0.0,0.0]]]}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)"
        );
    }

    #[test]
    fn poracle_vec_matches_golden() {
        let c = sample();
        // `Poracle`/`UnknownId` have no `PartialEq`; both derive `Serialize`, so
        // compare the serialized form against the golden.
        assert_eq!(
            serde_json::to_value(c.to_poracle_vec()).unwrap(),
            j(
                r#"[{"id":1,"name":"a","userSelectable":true,"displayInMatches":true,"path":[[0.0,0.0],[0.0,2.0],[2.0,2.0],[0.0,0.0]]},{"id":2,"name":"b","userSelectable":true,"displayInMatches":true,"path":[]}]"#
            )
        );
    }

    /// `single_vec_to_multipoint_feature` geometry, asserted against goldens frozen
    /// from the (now-deleted) matrix `SingleVec::to_feature(CirclePokemon)`. Covers
    /// open, pre-closed, and interior-duplicate routes — the closing-coord +
    /// adjacent-dedupe edges the matrix handled (all three dedupe to the same
    /// MultiPoint).
    #[test]
    fn single_vec_multipoint_feature_matches_golden() {
        use crate::geometry::single_vec_to_multipoint_feature;

        let golden =
            geojson::GeometryValue::MultiPoint { coordinates: vec![geojson::Position::from([2.0, 1.0]), geojson::Position::from([4.0, 3.0]), geojson::Position::from([6.0, 5.0])] };
        for centers in [
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]],             // open
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [1.0, 2.0]], // pre-closed
            vec![[1.0, 2.0], [3.0, 4.0], [3.0, 4.0], [5.0, 6.0]], // interior dup
        ] {
            let centers: SingleVec = centers;
            let new = single_vec_to_multipoint_feature(&centers);
            assert_eq!(
                new.geometry.unwrap().value,
                golden,
                "bootstrap MultiPoint must match the golden"
            );
        }
    }

    /// `single_vec_to_polygon_feature` geometry, asserted against goldens frozen
    /// from the (now-deleted) matrix `SingleVec::to_feature` (no fence type →
    /// polygon, ring-closed).
    #[test]
    fn single_vec_polygon_feature_matches_golden() {
        use crate::geometry::single_vec_to_polygon_feature;

        let golden = geojson::GeometryValue::Polygon { coordinates: vec![vec![
            geojson::Position::from([2.0, 1.0]),
            geojson::Position::from([4.0, 3.0]),
            geojson::Position::from([6.0, 5.0]),
            geojson::Position::from([2.0, 1.0]),
        ]] };
        for centers in [
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]], // open ring
            vec![[1.0, 2.0], [3.0, 4.0], [5.0, 6.0], [1.0, 2.0]], // pre-closed
        ] {
            let centers: SingleVec = centers;
            let new = single_vec_to_polygon_feature(&centers);
            assert_eq!(
                new.geometry.unwrap().value,
                golden,
                "bootstrap-plugin Polygon must match the golden"
            );
        }
    }
}
