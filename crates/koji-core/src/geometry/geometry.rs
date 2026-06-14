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

impl TrimPrecision for Vec<Vec<Vec<f64>>> {
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
                Geometry::from(&Polygon::<f64>::try_from(self).unwrap().simplify(0.0001))
            }
            Value::MultiPolygon(_) => Geometry::from(
                &MultiPolygon::<f64>::try_from(self)
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
                let mut formatted_data: Vec<Vec<Vec<Vec<f64>>>> = Vec::new();
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
/// for a geojson `Geometry`, via the Koji-native path. Replaces `GetBbox for Geometry`.
fn geometry_geojson_bbox(g: &Geometry) -> Option<geojson::Bbox> {
    let feature = geojson::Feature {
        geometry: Some(g.clone()),
        ..Default::default()
    };
    KojiGeometry::try_from(feature)
        .ok()
        .and_then(|kg| KojiGeometryCollection::new(vec![kg]).geojson_bbox())
}

impl GetBbox for Geometry {
    fn get_bbox(&self) -> Option<geojson::Bbox> {
        // Koji-native: wrap the geometry in a minimal `Feature`, convert to a
        // `KojiGeometry`, and read its `[lat, lon]` list via the Phase 1B inherent
        // `to_single_vec` (no `To*` matrix), then its bbox. An unconvertible
        // geometry yields no bbox.
        let feature = geojson::Feature {
            geometry: Some(self.clone()),
            ..Default::default()
        };
        match KojiGeometry::try_from(feature) {
            Ok(kg) => KojiGeometryCollection::new(vec![kg])
                .to_single_vec()
                .get_bbox(),
            Err(_) => None,
        }
    }
}
