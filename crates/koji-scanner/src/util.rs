use std::fmt::Write;

use geojson::{Feature, FeatureCollection, Value};

use koji_core::{KojiBbox, KojiGeometry, KojiGeometryCollection, SingleVec};

/// The `[lat, lon]` coordinate list of one geojson `Feature`, via the Koji-native
/// path (`KojiGeometry::try_from` → the inherent `to_single_vec`). A feature with
/// no/unconvertible geometry yields an empty vec. Bbox is only consumed for
/// `Polygon`/`MultiPolygon` in `sql_raw_bbox`, where `single_group` reproduces the
/// flattening exactly.
fn feature_single_vec(feature: &Feature) -> SingleVec {
    match KojiGeometry::try_from(feature.clone()) {
        Ok(kg) => KojiGeometryCollection::new(vec![kg]).to_single_vec(),
        Err(_) => vec![],
    }
}

pub fn sql_raw_bbox(area: &FeatureCollection) -> String {
    let mut string = String::new();
    for (i, feature) in area.into_iter().enumerate() {
        let bbox = if let Some(bbox) = feature.bbox.as_ref() {
            bbox.clone()
        } else if let Some(bbox) = KojiBbox::from_points(&feature_single_vec(feature))
            .map(|b| b.trim(6).to_geojson_bbox_vec())
        {
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
