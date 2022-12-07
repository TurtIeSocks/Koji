use super::*;

use geojson::Value;

use crate::models::{MultiVec, Poracle};

pub fn from_collection(fc: FeatureCollection) -> Vec<Poracle> {
    let mut return_vec: Vec<Poracle> = vec![];

    for (i, feature) in fc.into_iter().enumerate() {
        let mut poracle_feat = Poracle {
            name: None,
            color: None,
            description: None,
            display_in_matches: None,
            group: None,
            id: None,
            user_selectable: None,
            path: None,
            multipath: None,
        };
        if feature.contains_property("name") {
            poracle_feat.name = Some(
                feature
                    .property("name")
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            );
        }
        if feature.contains_property("id") {
            poracle_feat.id = Some(feature.property("id").unwrap().as_u64().unwrap_or(i as u64));
        } else {
            poracle_feat.id = Some(i as u64);
        }
        if feature.contains_property("color") {
            poracle_feat.color = Some(
                feature
                    .property("color")
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            );
        }
        if feature.contains_property("description") {
            poracle_feat.description = Some(
                feature
                    .property("description")
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            );
        }
        if feature.contains_property("group") {
            poracle_feat.group = Some(
                feature
                    .property("group")
                    .unwrap()
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            );
        }
        if feature.contains_property("display_in_matches") {
            poracle_feat.display_in_matches = Some(
                feature
                    .property("display_in_matches")
                    .unwrap()
                    .as_bool()
                    .unwrap_or(true),
            );
        } else {
            poracle_feat.display_in_matches = Some(true);
        }
        if feature.contains_property("user_selectable") {
            poracle_feat.user_selectable = Some(
                feature
                    .property("user_selectable")
                    .unwrap()
                    .as_bool()
                    .unwrap_or(true),
            );
        } else {
            poracle_feat.user_selectable = Some(true);
        }
        if let Some(geometry) = feature.geometry {
            let mut multipath: MultiVec = vec![];
            match geometry.value {
                Value::MultiPolygon(_) => {
                    feature::split_multi(geometry).into_iter().for_each(|f| {
                        multipath.push(vector::from_geometry(f.geometry.unwrap()));
                    })
                }
                Value::GeometryCollection(geometries) => geometries.into_iter().for_each(|g| {
                    if g.value.type_name() == "Polygon" {
                        let value = vector::from_geometry(g);
                        if !value.is_empty() {
                            multipath.push(value)
                        }
                    }
                }),
                Value::Polygon(_) => poracle_feat.path = Some(vector::from_geometry(geometry)),
                _ => {
                    println!(
                        "Poracle format does not support: {:?}",
                        geometry.value.type_name()
                    );
                }
            }
            if !multipath.is_empty() {
                poracle_feat.multipath = Some(multipath);
            }
        }
        return_vec.push(poracle_feat);
    }
    return_vec
}
