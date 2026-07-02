use crate::TrimPrecision;
use geo::{MultiPolygon, Polygon, Simplify};
use geojson::{Geometry, GeometryValue};

use super::*;

impl EnsurePoints for Geometry {
    fn ensure_first_last(self) -> Self {
        let mut return_value = self;
        match &mut return_value.value {
            GeometryValue::MultiPolygon { coordinates: polygons } => {
                for polygon in polygons.iter_mut() {
                    for line_string in polygon.iter_mut() {
                        let last = match line_string.last() {
                            Some(last) => last,
                            None => continue,
                        };
                        if last[0] != line_string[0][0] && last[1] != line_string[0][1] {
                            line_string.push(line_string[0].clone())
                        }
                    }
                }
                return_value
            }
            GeometryValue::Polygon { coordinates: poly } => {
                for line_string in poly {
                    let last = match line_string.last() {
                        Some(last) => last,
                        None => continue,
                    };
                    if last[0] != line_string[0][0] && last[1] != line_string[0][1] {
                        line_string.push(line_string[0].clone())
                    }
                }
                return_value
            }
            _ => return_value,
        }
    }
}

// geojson 1.0's `Position` is a tinyvec-backed struct (not `Vec<f64>`), so the
// old hand-rolled `Vec<Vec<Vec<f64>>>` impl no longer matches coordinate types.
// Recurse instead: trim a single `Position`, then a blanket `Vec<T>` lifts it to
// LineString/Polygon/MultiPolygon nesting for free.
impl TrimPrecision for geojson::Position {
    fn trim_precision(self, precision: u32) -> Self {
        self.as_slice()
            .iter()
            .map(|c| c.trim_precision(precision))
            .collect::<Vec<Precision>>()
            .into()
    }
}

impl<T: TrimPrecision> TrimPrecision for Vec<T> {
    fn trim_precision(self, precision: u32) -> Self {
        self.into_iter()
            .map(|x| x.trim_precision(precision))
            .collect()
    }
}

impl GeometryHelpers for Geometry {
    fn simplify(self) -> Self {
        let mut geometry = match self.value {
            GeometryValue::Polygon { .. } => {
                Geometry::from(&Polygon::<Precision>::try_from(self).unwrap().simplify(0.0001))
            }
            GeometryValue::MultiPolygon { .. } => Geometry::from(
                &MultiPolygon::<Precision>::try_from(self)
                    .unwrap()
                    .simplify(0.0001),
            ),
            _ => self,
        };
        geometry.bbox = geometry_geojson_bbox(&geometry);
        geometry
    }
}

impl TrimPrecision for Geometry {
    fn trim_precision(self, precision: u32) -> Self {
        let mut geometry = match self.value {
            GeometryValue::Polygon { coordinates: value } => {
                Geometry::from(geojson::GeometryValue::Polygon { coordinates: value.trim_precision(precision) })
            }
            GeometryValue::MultiPolygon { coordinates: value } => Geometry::from(
                geojson::GeometryValue::MultiPolygon { coordinates: value.trim_precision(precision) },
            ),
            _ => self,
        };
        geometry.bbox = geometry_geojson_bbox(&geometry);
        geometry
    }
}

/// The geojson `bbox` member `[min_lon, min_lat, max_lon, max_lat]` (trimmed to 6)
/// for a geojson `Geometry`, via the Koji-native path.
fn geometry_geojson_bbox(g: &Geometry) -> Option<geojson::Bbox> {
    let feature = geojson::Feature {
        geometry: Some(g.clone()),
        ..Default::default()
    };
    KojiGeometry::try_from(feature)
        .ok()
        .and_then(|kg| KojiGeometryCollection::new(vec![kg]).geojson_bbox())
}

#[cfg(test)]
mod tests {
    use super::*;
    use geojson::{Geometry, GeometryValue};

    fn make_polygon_gj(closed: bool) -> Geometry {
        let ring = if closed {
            vec![
                geojson::Position::from([0.0, 0.0]),
                geojson::Position::from([1.0, 0.0]),
                geojson::Position::from([1.0, 1.0]),
                geojson::Position::from([0.0, 0.0]),
            ]
        } else {
            vec![geojson::Position::from([0.0, 0.0]), geojson::Position::from([1.0, 0.0]), geojson::Position::from([1.0, 1.0])]
        };
        Geometry::new(GeometryValue::Polygon { coordinates: vec![ring] })
    }

    // ── EnsurePoints for Geometry ────────────────────────────────────────────

    #[test]
    fn ensure_first_last_closes_open_polygon_ring() {
        let g = make_polygon_gj(false);
        let closed = g.ensure_first_last();
        if let GeometryValue::Polygon { coordinates: rings } = &closed.value {
            let ring = &rings[0];
            assert_eq!(
                ring[0],
                ring[ring.len() - 1],
                "first and last must match after closing"
            );
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn ensure_first_last_already_closed_polygon_unchanged_len() {
        let g = make_polygon_gj(true);
        let ring_len_before = if let GeometryValue::Polygon { coordinates: rings } = &g.value {
            rings[0].len()
        } else {
            panic!("expected Polygon")
        };
        let out = g.ensure_first_last();
        if let GeometryValue::Polygon { coordinates: rings } = &out.value {
            // Already closed: the condition checks BOTH axes differ, so a ring
            // whose last == first on both axes is left unchanged.
            assert_eq!(rings[0].len(), ring_len_before);
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn ensure_first_last_non_polygon_passthrough() {
        // Point geometry passes through unchanged.
        let g = Geometry::new(GeometryValue::Point { coordinates: geojson::Position::from([1.0, 2.0]) });
        let out = g.clone().ensure_first_last();
        assert_eq!(out.value, g.value);
    }

    #[test]
    fn ensure_first_last_multipolygon_closes_rings() {
        let ring = vec![geojson::Position::from([0.0, 0.0]), geojson::Position::from([2.0, 0.0]), geojson::Position::from([2.0, 2.0])];
        let mp = Geometry::new(GeometryValue::MultiPolygon { coordinates: vec![vec![ring]] });
        let out = mp.ensure_first_last();
        if let GeometryValue::MultiPolygon { coordinates: polys } = &out.value {
            let ring = &polys[0][0];
            assert_eq!(ring[0], ring[ring.len() - 1]);
        } else {
            panic!("expected MultiPolygon");
        }
    }

    // ── TrimPrecision for Vec<Vec<Vec<Precision>>> ────────────────────────────────

    #[test]
    fn trim_precision_vec3_rounds_all_coords() {
        let v: Vec<Vec<geojson::Position>> = vec![vec![geojson::Position::from([1.23456789, 9.87654321])]];
        let trimmed = v.trim_precision(4);
        assert_eq!(trimmed[0][0][0], 1.2346);
        assert_eq!(trimmed[0][0][1], 9.8765);
    }

    // ── TrimPrecision for Geometry ───────────────────────────────────────────

    #[test]
    fn trim_precision_geometry_polygon() {
        let ring = vec![geojson::Position::from([1.23456789f64, 2.34567890])];
        let g = Geometry::new(GeometryValue::Polygon { coordinates: vec![ring] });
        let trimmed = g.trim_precision(4);
        if let GeometryValue::Polygon { coordinates: rings } = &trimmed.value {
            assert_eq!(rings[0][0][0], 1.2346);
            assert_eq!(rings[0][0][1], 2.3457);
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn trim_precision_geometry_multipolygon() {
        let ring = vec![geojson::Position::from([1.111111f64, 2.222222])];
        let g = Geometry::new(GeometryValue::MultiPolygon { coordinates: vec![vec![ring]] });
        let trimmed = g.trim_precision(3);
        if let GeometryValue::MultiPolygon { coordinates: polys } = &trimmed.value {
            assert_eq!(polys[0][0][0][0], 1.111);
            assert_eq!(polys[0][0][0][1], 2.222);
        } else {
            panic!("expected MultiPolygon");
        }
    }

    #[test]
    fn trim_precision_geometry_non_polygon_passthrough() {
        // Point geometry: TrimPrecision for Geometry falls through the `_ => self` arm.
        // Uses 2-element coord (lon, lat) as required by geojson spec.
        let g = Geometry::new(GeometryValue::Point { coordinates: geojson::Position::from([1.999999, 2.888888]) });
        let out = g.clone().trim_precision(2);
        assert_eq!(out.value, g.value);
    }

    // ── GeometryHelpers::simplify ─────────────────────────────────────────────

    #[test]
    fn simplify_polygon_reduces_collinear_points() {
        // Ring with a nearly-collinear midpoint at (1, 0.00001).
        let ring = vec![
            geojson::Position::from([0.0, 0.0]),
            geojson::Position::from([1.0, 0.00001]),
            geojson::Position::from([2.0, 0.0]),
            geojson::Position::from([2.0, 2.0]),
            geojson::Position::from([0.0, 2.0]),
            geojson::Position::from([0.0, 0.0]),
        ];
        let g = Geometry::new(GeometryValue::Polygon { coordinates: vec![ring] });
        let simplified = g.simplify();
        if let GeometryValue::Polygon { coordinates: rings } = &simplified.value {
            // The collinear point should be dropped → fewer vertices.
            assert!(
                rings[0].len() < 6,
                "expected simplification to drop a point"
            );
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn simplify_non_polygon_passthrough() {
        let g = Geometry::new(GeometryValue::Point { coordinates: geojson::Position::from([0.0, 0.0]) });
        let out = g.clone().simplify();
        assert_eq!(out.value, g.value);
    }
}
