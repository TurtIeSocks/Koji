use super::*;

use geo::Coord;
use geojson::{Geometry, Value};
use model::{
    api::{collection::Default, single_vec::SingleVec, BBox, ToCollection},
    db::{area, geofence, gym, instance, pokestop, spawnpoint, GenericData},
    KojiDb,
};

pub mod auth;
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
) -> Result<FeatureCollection, DbErr> {
    match load_feature(instance, scanner_type, conn).await {
        Ok(feature) => Ok(feature.to_collection(None, None)),
        Err(err) => Err(err),
    }
}

pub async fn load_feature(
    instance: &String,
    scanner_type: &String,
    conn: &KojiDb,
) -> Result<Feature, DbErr> {
    match geofence::Query::route(&conn.koji_db, &instance).await {
        Ok(area) => Ok(area),
        Err(_) => {
            if scanner_type.eq("rdm") {
                instance::Query::route(&conn.data_db, &instance).await
            } else {
                area::Query::route(&conn.unown_db.as_ref().unwrap(), &instance).await
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
) -> Result<FeatureCollection, DbErr> {
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
            foreign_members: None,
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
