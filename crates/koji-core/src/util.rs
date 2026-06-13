use std::fmt::Write;

use geojson::{FeatureCollection, Value};

use crate::geometry::{EnsurePoints, GetBbox, ToSingleVec};

pub trait TrimPrecision {
    fn trim_precision(self, precision: u32) -> Self;
}

impl TrimPrecision for f64 {
    fn trim_precision(self, precision: u32) -> f64 {
        if !self.is_finite() {
            return self;
        }
        let precision_factor = 10u32.pow(precision) as f64;
        (self * precision_factor).round() / precision_factor
    }
}

pub fn sql_raw(area: &FeatureCollection) -> String {
    let mut string = "".to_string();
    for (i, feature) in area.into_iter().enumerate() {
        let bbox = if let Some(bbox) = feature.bbox.clone() {
            bbox
        } else {
            feature.clone().to_single_vec().get_bbox().unwrap()
        };
        if let Some(geometry) = feature.geometry.clone() {
            let geo = geometry.ensure_first_last();
            match geo.value {
                Value::Polygon(_) | Value::MultiPolygon(_) => {
                    string = format!(
                        "{}{} (\n\tlon BETWEEN {} AND {}\n\tAND lat BETWEEN {} AND {}\n\tAND ST_CONTAINS(\n\t\tST_GeomFromGeoJSON('{}', 2, 0),\n\t\tPOINT(lon, lat)\n\t)\n)",
                        string,
                        if i == 0 { "" } else { "\nOR" },
                        bbox[0],
                        bbox[2],
                        bbox[1],
                        bbox[3],
                        geo
                    );
                }
                _ => {}
            }
        }
    }
    string
}

pub fn sql_raw_bbox(area: &FeatureCollection) -> String {
    let mut string = String::new();
    for (i, feature) in area.into_iter().enumerate() {
        let bbox = if let Some(bbox) = feature.bbox.as_ref() {
            bbox.clone()
        } else if let Some(bbox) = feature.clone().to_single_vec().get_bbox() {
            bbox
        } else {
            continue;
        };
        if let Some(geometry) = &feature.geometry {
            match geometry.value {
                Value::Polygon(_) | Value::MultiPolygon(_) => {
                    let _ = write!(
                        string,
                        "{} (lon BETWEEN {} AND {} AND lat BETWEEN {} AND {})",
                        if i == 0 { "" } else { "\nOR" },
                        bbox[0],
                        bbox[2],
                        bbox[1],
                        bbox[3]
                    );
                }
                _ => {}
            }
        }
    }
    string
}
