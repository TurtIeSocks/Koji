use super::*;

use geo::Coord;
use geojson::{Geometry, Value};
use model::{
    api::{collection::Default, single_vec::SingleVec, BBox, ToCollection},
    db::{area, geofence, gym, instance, pokestop, spawnpoint, GenericData},
    error::ModelError,
    KojiDb,
};

pub mod auth;
pub mod error;
pub mod request;
pub mod response;

pub fn is_docker() -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push("dist");
    let metadata = fs::metadata(path)?;
    Ok(metadata.is_dir())
}

pub async fn load_collection(
    instance: &String,
    scanner_type: &String,
    conn: &KojiDb,
) -> Result<FeatureCollection, ModelError> {
    match load_feature(instance, scanner_type, conn).await {
        Ok(feature) => Ok(feature.to_collection(None, None)),
        Err(err) => Err(err),
    }
}

pub async fn load_feature(
    instance: &String,
    scanner_type: &String,
    conn: &KojiDb,
) -> Result<Feature, ModelError> {
    match geofence::Query::get_one_feature(&conn.koji_db, instance.to_string(), true).await {
        Ok(area) => Ok(area),
        Err(_) => {
            if scanner_type.eq("rdm") {
                instance::Query::feature_from_name(&conn.data_db, &instance).await
            } else {
                area::Query::feature_from_name(
                    &conn.unown_db.as_ref().unwrap(),
                    &instance,
                    "auto_quest".to_string(),
                )
                .await
            }
        }
    }
}

pub async fn create_or_find_collection(
    instance: &String,
    scanner_type: &String,
    conn: &KojiDb,
    area: FeatureCollection,
    data_points: &SingleVec,
) -> Result<FeatureCollection, ModelError> {
    if !data_points.is_empty() {
        let bbox = BBox::new(
            &data_points
                .iter()
                .map(|p| Coord { x: p[1], y: p[0] })
                .collect(),
        );
        Ok(FeatureCollection {
            bbox: bbox.get_geojson_bbox(),
            features: vec![Feature {
                bbox: bbox.get_geojson_bbox(),
                geometry: Some(Geometry {
                    value: Value::Polygon(bbox.get_poly()),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Feature::default()
            }],
            ..FeatureCollection::default()
        })
    } else if !area.features.is_empty() {
        Ok(area)
    } else if !instance.is_empty() {
        load_collection(instance, scanner_type, conn).await
    } else {
        Ok(FeatureCollection::default())
    }
}

pub async fn points_from_area(
    area: &FeatureCollection,
    category: &String,
    conn: &KojiDb,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    if !area.features.is_empty() {
        match category.as_str() {
            "gym" => gym::Query::area(&conn.data_db, &area, last_seen).await,
            "pokestop" => pokestop::Query::area(&conn.data_db, &area, last_seen).await,
            "spawnpoint" => spawnpoint::Query::area(&conn.data_db, &area, last_seen).await,
            _ => Err(DbErr::Custom("Invalid Category".to_string())),
        }
    } else {
        Ok(vec![])
    }
}
