//! `KojiGeometry` (one geometry + metadata) and `KojiGeometryCollection` (the
//! universal interface newtype). geo-types is the canonical core; everything
//! else converts at the edge.

use geo::{BoundingRect, Geometry, MultiPoint, Point, Rect, Simplify};

use super::KojiMeta;

#[derive(Debug, Clone, PartialEq)]
pub struct KojiGeometry {
    pub geometry: Geometry<f64>,
    pub meta: KojiMeta,
}

impl KojiGeometry {
    pub fn new(geometry: impl Into<Geometry<f64>>) -> Self {
        Self {
            geometry: geometry.into(),
            meta: KojiMeta::default(),
        }
    }

    pub fn with_meta(mut self, meta: KojiMeta) -> Self {
        self.meta = meta;
        self
    }

    pub fn bbox(&self) -> Option<Rect<f64>> {
        self.geometry.bounding_rect()
    }

    /// Douglas–Peucker simplification of the inner geometry. Only ring/line
    /// geometries (`LineString`/`Polygon`/`MultiLineString`/`MultiPolygon`) are
    /// affected; point geometries pass through unchanged. Metadata is preserved.
    /// Re-homed from the matrix `GeometryHelpers::simplify` (which used the same
    /// `geo` Douglas–Peucker, `epsilon = 0.0001`).
    pub fn simplify(mut self, epsilon: f64) -> Self {
        self.geometry = match self.geometry {
            Geometry::LineString(g) => Geometry::LineString(g.simplify(epsilon)),
            Geometry::Polygon(g) => Geometry::Polygon(g.simplify(epsilon)),
            Geometry::MultiLineString(g) => Geometry::MultiLineString(g.simplify(epsilon)),
            Geometry::MultiPolygon(g) => Geometry::MultiPolygon(g.simplify(epsilon)),
            other => other,
        };
        self
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KojiGeometryCollection {
    pub items: Vec<KojiGeometry>,
    pub bbox: Option<Rect<f64>>,
}

impl KojiGeometryCollection {
    pub fn new(items: Vec<KojiGeometry>) -> Self {
        let bbox = items
            .iter()
            .filter_map(KojiGeometry::bbox)
            .reduce(union_rect);
        Self { items, bbox }
    }

    /// Douglas–Peucker simplification of every item (see
    /// [`KojiGeometry::simplify`]). Rebuilds the collection bbox.
    pub fn simplify(self, epsilon: f64) -> Self {
        Self::new(
            self.items
                .into_iter()
                .map(|item| item.simplify(epsilon))
                .collect(),
        )
    }

    /// Collapse every `Point` item into a single `MultiPoint` item, dropping all
    /// other geometries (re-homed from the v1/v2 `/merge-points` endpoints, which
    /// pulled `Value::Point` features into one `MultiPoint`). Point order is
    /// preserved; the merged item carries default metadata. An empty input (or no
    /// points) yields a single empty-`MultiPoint` item, matching the old path
    /// which always built one `MultiPoint` feature.
    pub fn merge_points(self) -> Self {
        let points: Vec<Point<f64>> = self
            .items
            .into_iter()
            .filter_map(|item| match item.geometry {
                Geometry::Point(p) => Some(p),
                _ => None,
            })
            .collect();
        let merged = KojiGeometry::new(MultiPoint::new(points));
        Self::new(vec![merged])
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
        assert_eq!(
            g.bbox(),
            Some(Rect::new(coord! {x:3.0,y:4.0}, coord! {x:3.0,y:4.0}))
        );
    }

    #[test]
    fn collection_bbox_unions_items() {
        let c = KojiGeometryCollection::new(vec![
            KojiGeometry::new(Point::new(0.0, 0.0)),
            KojiGeometry::new(Point::new(10.0, 5.0)),
        ]);
        assert_eq!(c.items.len(), 2);
        assert_eq!(
            c.bbox,
            Some(Rect::new(coord! {x:0.0,y:0.0}, coord! {x:10.0,y:5.0}))
        );
    }

    /// `merge_points` parity vs the old `/merge-points` matrix path: it collected
    /// `Value::Point` coords (`[lon, lat]`) into one `Value::MultiPoint`. The
    /// Koji-native path must emit the identical `MultiPoint` geojson.
    #[test]
    fn merge_points_matches_matrix_multipoint() {
        use crate::geometry::ValueHelpers;

        // geojson Point coords are [lon, lat].
        let raw: Vec<Vec<f64>> = vec![vec![2.0, 1.0], vec![4.0, 3.0], vec![6.0, 5.0]];

        // OLD matrix path: a MultiVec wrapping the points, projected as MultiPoint.
        let old_value = vec![raw.iter().map(|p| [p[1], p[0]]).collect::<Vec<[f64; 2]>>()]
            .multi_point();

        // NEW path: Point items -> merge_points -> geojson Geometry.
        let coll = KojiGeometryCollection::new(
            raw.iter()
                .map(|p| KojiGeometry::new(Point::new(p[0], p[1])))
                .collect(),
        )
        .merge_points();
        assert_eq!(coll.items.len(), 1);
        let new_value = geojson::Value::from(&coll.items[0].geometry);

        assert_eq!(new_value, old_value, "merge_points must match the matrix MultiPoint");
    }

    /// Non-point geometries are dropped by `merge_points` (the old path only read
    /// `Value::Point` features).
    #[test]
    fn merge_points_drops_non_points() {
        use geo::{LineString, coord};
        let coll = KojiGeometryCollection::new(vec![
            KojiGeometry::new(Point::new(1.0, 1.0)),
            KojiGeometry::new(LineString::new(vec![coord! {x:0.,y:0.}, coord! {x:1.,y:1.}])),
            KojiGeometry::new(Point::new(2.0, 2.0)),
        ])
        .merge_points();
        let geojson::Value::MultiPoint(pts) = geojson::Value::from(&coll.items[0].geometry) else {
            panic!("expected MultiPoint");
        };
        assert_eq!(pts, vec![vec![1.0, 1.0], vec![2.0, 2.0]]);
    }

    /// `simplify` parity vs the matrix `GeometryHelpers::simplify` on a polygon
    /// (both Douglas–Peucker, epsilon 0.0001).
    #[test]
    fn simplify_matches_matrix_on_polygon() {
        use crate::geometry::GeometryHelpers;
        use geojson::{Geometry as GjGeometry, Value as GjValue};

        // A polygon with a nearly-collinear vertex that DP should drop.
        let ring = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.00001],
            vec![2.0, 0.0],
            vec![2.0, 2.0],
            vec![0.0, 2.0],
            vec![0.0, 0.0],
        ];
        let gj = GjGeometry::new(GjValue::Polygon(vec![ring]));

        // OLD matrix path on the geojson geometry.
        let old_value = gj.clone().simplify().value;

        // NEW path: geojson -> geo -> KojiGeometry::simplify -> geojson.
        let geo_geom: geo::Geometry<f64> = geo::Geometry::try_from(&gj).unwrap();
        let simplified = KojiGeometry::new(geo_geom).simplify(0.0001);
        let new_value = geojson::Value::from(&simplified.geometry);

        assert_eq!(new_value, old_value, "simplify must match the matrix Douglas-Peucker");
    }

    #[test]
    fn empty_collection_has_no_bbox() {
        assert_eq!(KojiGeometryCollection::default().bbox, None);
    }
}
