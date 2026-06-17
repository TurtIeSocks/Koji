use crate::TrimPrecision;
use geo::{MultiPolygon, Polygon, Simplify};
use geojson::{Geometry, Value};

use super::*;

impl EnsurePoints for Geometry {
    fn ensure_first_last(self) -> Self {
        let mut return_value = self;
        match &mut return_value.value {
            Value::MultiPolygon(polygons) => {
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
            Value::Polygon(poly) => {
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

impl TrimPrecision for Vec<Vec<Vec<Precision>>> {
    fn trim_precision(self, precision: u32) -> Self {
        let mut formatted_data = Vec::new();

        for outer_vec in self {
            let mut formatted_outer_vec = Vec::new();
            for inner_vec in outer_vec {
                let mut formatted_inner_vec = Vec::new();
                for num in inner_vec {
                    formatted_inner_vec.push(num.trim_precision(precision));
                }
                formatted_outer_vec.push(formatted_inner_vec);
            }
            formatted_data.push(formatted_outer_vec);
        }

        formatted_data
    }
}

impl GeometryHelpers for Geometry {
    fn simplify(self) -> Self {
        let mut geometry = match self.value {
            Value::Polygon(_) => {
                Geometry::from(&Polygon::<Precision>::try_from(self).unwrap().simplify(0.0001))
            }
            Value::MultiPolygon(_) => Geometry::from(
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
            Value::Polygon(value) => {
                Geometry::from(geojson::Value::Polygon(value.trim_precision(precision)))
            }
            Value::MultiPolygon(value) => {
                let mut formatted_data: Vec<Vec<Vec<Vec<Precision>>>> = Vec::new();
                for outer_vec in value {
                    formatted_data.push(outer_vec.trim_precision(precision))
                }
                Geometry::from(geojson::Value::MultiPolygon(formatted_data))
            }
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
    use geojson::{Geometry, Value};

    fn make_polygon_gj(closed: bool) -> Geometry {
        let ring = if closed {
            vec![
                vec![0.0, 0.0],
                vec![1.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 0.0],
            ]
        } else {
            vec![vec![0.0, 0.0], vec![1.0, 0.0], vec![1.0, 1.0]]
        };
        Geometry::new(Value::Polygon(vec![ring]))
    }

    // ── EnsurePoints for Geometry ────────────────────────────────────────────

    #[test]
    fn ensure_first_last_closes_open_polygon_ring() {
        let g = make_polygon_gj(false);
        let closed = g.ensure_first_last();
        if let Value::Polygon(rings) = &closed.value {
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
        let ring_len_before = if let Value::Polygon(rings) = &g.value {
            rings[0].len()
        } else {
            panic!("expected Polygon")
        };
        let out = g.ensure_first_last();
        if let Value::Polygon(rings) = &out.value {
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
        let g = Geometry::new(Value::Point(vec![1.0, 2.0]));
        let out = g.clone().ensure_first_last();
        assert_eq!(out.value, g.value);
    }

    #[test]
    fn ensure_first_last_multipolygon_closes_rings() {
        let ring = vec![vec![0.0, 0.0], vec![2.0, 0.0], vec![2.0, 2.0]];
        let mp = Geometry::new(Value::MultiPolygon(vec![vec![ring]]));
        let out = mp.ensure_first_last();
        if let Value::MultiPolygon(polys) = &out.value {
            let ring = &polys[0][0];
            assert_eq!(ring[0], ring[ring.len() - 1]);
        } else {
            panic!("expected MultiPolygon");
        }
    }

    // ── TrimPrecision for Vec<Vec<Vec<Precision>>> ────────────────────────────────

    #[test]
    fn trim_precision_vec3_rounds_all_coords() {
        let v: Vec<Vec<Vec<Precision>>> = vec![vec![vec![1.23456789, 9.87654321]]];
        let trimmed = v.trim_precision(4);
        assert_eq!(trimmed[0][0][0], 1.2346);
        assert_eq!(trimmed[0][0][1], 9.8765);
    }

    // ── TrimPrecision for Geometry ───────────────────────────────────────────

    #[test]
    fn trim_precision_geometry_polygon() {
        let ring = vec![vec![1.23456789f64, 2.34567890]];
        let g = Geometry::new(Value::Polygon(vec![ring]));
        let trimmed = g.trim_precision(4);
        if let Value::Polygon(rings) = &trimmed.value {
            assert_eq!(rings[0][0][0], 1.2346);
            assert_eq!(rings[0][0][1], 2.3457);
        } else {
            panic!("expected Polygon");
        }
    }

    #[test]
    fn trim_precision_geometry_multipolygon() {
        let ring = vec![vec![1.111111f64, 2.222222]];
        let g = Geometry::new(Value::MultiPolygon(vec![vec![ring]]));
        let trimmed = g.trim_precision(3);
        if let Value::MultiPolygon(polys) = &trimmed.value {
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
        let g = Geometry::new(Value::Point(vec![1.999999, 2.888888]));
        let out = g.clone().trim_precision(2);
        assert_eq!(out.value, g.value);
    }

    // ── GeometryHelpers::simplify ─────────────────────────────────────────────

    #[test]
    fn simplify_polygon_reduces_collinear_points() {
        // Ring with a nearly-collinear midpoint at (1, 0.00001).
        let ring = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.00001],
            vec![2.0, 0.0],
            vec![2.0, 2.0],
            vec![0.0, 2.0],
            vec![0.0, 0.0],
        ];
        let g = Geometry::new(Value::Polygon(vec![ring]));
        let simplified = g.simplify();
        if let Value::Polygon(rings) = &simplified.value {
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
        let g = Geometry::new(Value::Point(vec![0.0, 0.0]));
        let out = g.clone().simplify();
        assert_eq!(out.value, g.value);
    }
}
