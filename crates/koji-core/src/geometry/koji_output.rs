//! Outbound format adapters for `KojiGeometryCollection`.
//!
//! These are **matrix-independent**: each adapter reads coordinates from the
//! item's `geo::Geometry` and metadata from its `KojiMeta`, never delegating to
//! the geojson `To*` matrix (whose cells die in Phase 2). Correctness is pinned
//! by parity tests against the oracle `geojson::FeatureCollection::from(&c).to_X()`,
//! which is exact today.
//!
//! Coordinate convention: the matrix emits `PointArray = [lat, lon]` (y, x).
//! `geo::Geometry` stores `[x, y]` (lon, lat), so every extraction flips to
//! `[coord.y, coord.x]`.

use geo::{Geometry, LineString, MultiPolygon, Polygon};

use super::{MultiStruct, MultiVec, PointStruct, SingleStruct, SingleVec};
use crate::geometry::KojiGeometryCollection;

/// Flatten one closed/open ring (`LineString`) into the matrix's `[lat, lon]`
/// point order, matching `Geometry::to_single_vec`'s `[point[1], point[0]]`.
fn ring_points(line: &LineString<f64>) -> Vec<[f64; 2]> {
    line.coords().map(|c| [c.y, c.x]).collect()
}

/// One polygon → a single flat `[lat, lon]` vec across exterior + interior
/// rings, matching the matrix `Polygon` branch (which iterates every ring of
/// the geojson polygon and flattens them into one inner vec).
fn polygon_points(poly: &Polygon<f64>) -> Vec<[f64; 2]> {
    let mut out = ring_points(poly.exterior());
    for interior in poly.interiors() {
        out.extend(ring_points(interior));
    }
    out
}

/// Per-item grouping that mirrors `Feature::to_multi_vec`:
/// - `MultiPolygon` → one group per polygon
/// - everything else (Point/MultiPoint/Line/MultiLineString/Polygon) → one group
/// - `GeometryCollection` → one group per non-empty sub-geometry
///
/// Returns the list of `[lat, lon]` groups contributed by a single item.
fn item_groups(geom: &Geometry<f64>) -> Vec<Vec<[f64; 2]>> {
    match geom {
        Geometry::MultiPolygon(mp) => mp.iter().map(polygon_points).collect(),
        Geometry::GeometryCollection(gc) => gc
            .iter()
            .filter_map(|g| {
                let group = single_group(g);
                if group.is_empty() { None } else { Some(group) }
            })
            .collect(),
        other => vec![single_group(other)],
    }
}

/// The flat `[lat, lon]` coords of a geometry treated as a single group, matching
/// the matrix `_ => geometry.to_single_vec()` path (Polygon flattens rings;
/// MultiPolygon flattens every ring of every polygon; points/lines map directly).
fn single_group(geom: &Geometry<f64>) -> Vec<[f64; 2]> {
    match geom {
        Geometry::Point(p) => vec![[p.y(), p.x()]],
        Geometry::MultiPoint(mp) => mp.iter().map(|p| [p.y(), p.x()]).collect(),
        Geometry::Line(l) => vec![[l.start.y, l.start.x], [l.end.y, l.end.x]],
        Geometry::LineString(ls) => ring_points(ls),
        Geometry::MultiLineString(mls) => mls.iter().flat_map(ring_points).collect(),
        Geometry::Polygon(poly) => polygon_points(poly),
        Geometry::MultiPolygon(mp) => mp.iter().flat_map(|p| polygon_points(p)).collect(),
        Geometry::Rect(r) => polygon_points(&r.to_polygon()),
        Geometry::Triangle(t) => polygon_points(&t.to_polygon()),
        Geometry::GeometryCollection(gc) => gc.iter().flat_map(single_group).collect(),
    }
}

/// Append the first point if it doesn't already close the ring, matching the
/// matrix `SingleVec::ensure_first_last`. The matrix applies this when going
/// `SingleVec`/`MultiVec` → struct forms (`to_single_struct` on a `SingleVec`),
/// so the struct adapters must reproduce it.
fn ensure_first_last(mut points: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
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
fn to_point_struct(p: [f64; 2]) -> PointStruct {
    PointStruct {
        lat: p[0],
        lon: p[1],
    }
}

impl KojiGeometryCollection {
    /// All items' coordinates flattened into one `[lat, lon]` vec.
    ///
    /// Parity: `geojson::FeatureCollection::from(&self).to_single_vec()`.
    pub fn to_single_vec(&self) -> SingleVec {
        self.items
            .iter()
            .flat_map(|item| item_groups(&item.geometry).into_iter().flatten())
            .collect()
    }

    /// Coordinates grouped one inner vec per ring/geometry.
    ///
    /// Parity: `geojson::FeatureCollection::from(&self).to_multi_vec()`.
    pub fn to_multi_vec(&self) -> MultiVec {
        self.items
            .iter()
            .flat_map(|item| item_groups(&item.geometry))
            .collect()
    }

    /// All coordinates as one flat `PointStruct` vec.
    ///
    /// Parity: `geojson::FeatureCollection::from(&self).to_single_struct()`.
    /// The matrix flattens to a single vec, then `ensure_first_last`s it before
    /// the struct map — so this closes the *flattened* ring, not each group.
    pub fn to_single_struct(&self) -> SingleStruct {
        ensure_first_last(self.to_single_vec())
            .into_iter()
            .map(to_point_struct)
            .collect()
    }

    /// Coordinates as `PointStruct` groups, one per ring/geometry.
    ///
    /// Parity: `geojson::FeatureCollection::from(&self).to_multi_struct()`.
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
    /// Parity: `geojson::FeatureCollection::from(&self).to_text(sep_1, sep_2, poly_sep)`,
    /// i.e. `MultiVec::to_text` over `to_multi_vec()`. Coordinates use the default
    /// `f64` `Display` (no fixed precision), `lat` then `sep_1` then `lon`.
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
                    group_to_text(&group, sep_1, sep_2, poly_sep),
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
    /// Parity: `geojson::FeatureCollection::from(&self).to_sql()`.
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
            let bbox = bbox_of(&single_group(&item.geometry));
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
}

/// `[min_lon, min_lat, max_lon, max_lat]` of a `[lat, lon]` coord set, trimmed to
/// 6 decimals — byte-for-byte the matrix `SingleVec::get_bbox`. (bbox\[0,2\] track
/// `point[1]` = lon; bbox\[1,3\] track `point[0]` = lat.)
fn bbox_of(points: &[[f64; 2]]) -> [f64; 4] {
    let mut bbox = if points.is_empty() {
        [0.0, 0.0, 0.0, 0.0]
    } else {
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ]
    };
    for point in points {
        if point[1] < bbox[0] {
            bbox[0] = point[1];
        }
        if point[1] > bbox[2] {
            bbox[2] = point[1];
        }
        if point[0] < bbox[1] {
            bbox[1] = point[0];
        }
        if point[0] > bbox[3] {
            bbox[3] = point[0];
        }
    }
    [
        trim6(bbox[0]),
        trim6(bbox[1]),
        trim6(bbox[2]),
        trim6(bbox[3]),
    ]
}

/// Round to 6 decimals, matching `TrimPrecision for f64`.
fn trim6(v: f64) -> f64 {
    if !v.is_finite() {
        return v;
    }
    let factor = 1_000_000.0_f64;
    (v * factor).round() / factor
}

/// The item's geojson `Geometry`, ring-closed, rendered to its JSON string — the
/// exact value the oracle embeds in the SQL (`geojson::Value::from(&geo)` is the
/// edge geo→geojson conversion, then ring-closure inlined from the matrix
/// `EnsurePoints for Geometry`). Not a `To*` matrix call.
fn geojson_geometry_closed(geom: &Geometry<f64>) -> String {
    let mut value = geojson::Value::from(geom);
    // Inline `EnsurePoints::ensure_first_last`: close each ring whose last point
    // differs from its first on *both* axes (matrix uses `&&`).
    let close_ring = |ring: &mut Vec<Vec<f64>>| {
        if let Some(last) = ring.last() {
            if last[0] != ring[0][0] && last[1] != ring[0][1] {
                let first = ring[0].clone();
                ring.push(first);
            }
        }
    };
    match &mut value {
        geojson::Value::Polygon(rings) => rings.iter_mut().for_each(close_ring),
        geojson::Value::MultiPolygon(polys) => polys
            .iter_mut()
            .flat_map(|p| p.iter_mut())
            .for_each(close_ring),
        _ => {}
    }
    geojson::Geometry::new(value).to_string()
}

/// One group → text, matching `SingleVec::to_text`: the inter-point separator is
/// suppressed on the final point of the group.
fn group_to_text(group: &[[f64; 2]], sep_1: &str, sep_2: &str, poly_sep: bool) -> String {
    let last = if group.is_empty() { 0 } else { group.len() - 1 };
    group
        .iter()
        .enumerate()
        .map(|(i, pt)| point_to_text(pt, sep_1, if i == last { "" } else { sep_2 }, poly_sep))
        .collect()
}

/// One `[lat, lon]` point → text, matching `PointArray::to_text`:
/// `format!("{}{}{}{}", lat, sep_1, lon, sep_2)` with default `f64` `Display`.
fn point_to_text(pt: &[f64; 2], sep_1: &str, sep_2: &str, _poly_sep: bool) -> String {
    format!("{}{}{}{}", pt[0], sep_1, pt[1], sep_2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{ToMultiVec, ToSingleVec}; // the existing oracle traits
    use crate::{KojiGeometry, KojiGeometryCollection, KojiMeta, Mode};
    use geo::{LineString, MultiPoint, Point, Polygon, coord};

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

    #[test]
    fn single_struct_matches_oracle() {
        use crate::geometry::ToSingleStruct;
        let c = sample();
        // `PointStruct` has no `PartialEq`; compare its serialized form (it derives
        // `Serialize`) — full-fidelity parity against the oracle.
        let mine = serde_json::to_value(c.to_single_struct()).unwrap();
        let oracle = serde_json::to_value(fc(&c).to_single_struct()).unwrap();
        assert_eq!(mine, oracle);
    }

    #[test]
    fn multi_struct_matches_oracle() {
        use crate::geometry::ToMultiStruct;
        let c = sample();
        let mine = serde_json::to_value(c.to_multi_struct()).unwrap();
        let oracle = serde_json::to_value(fc(&c).to_multi_struct()).unwrap();
        assert_eq!(mine, oracle);
    }

    #[test]
    fn text_matches_oracle_both_param_sets() {
        use crate::geometry::ToText;
        let c = sample();
        assert_eq!(c.to_text(",", "\n", true), fc(&c).to_text(",", "\n", true));
        assert_eq!(c.to_text(" ", ",", false), fc(&c).to_text(" ", ",", false));
    }

    #[test]
    fn sql_matches_oracle() {
        use crate::geometry::ToSql;
        let c = sample();
        assert_eq!(c.to_sql(), fc(&c).to_sql());
    }
}
