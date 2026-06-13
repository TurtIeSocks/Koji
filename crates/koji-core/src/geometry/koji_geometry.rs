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

    #[test]
    fn empty_collection_has_no_bbox() {
        assert_eq!(KojiGeometryCollection::default().bbox, None);
    }
}
