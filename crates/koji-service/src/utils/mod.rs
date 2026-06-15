use super::*;

use geojson::{Geometry, Value};
use koji_core::{EnsurePoints, KojiBbox, SingleVec, SpawnpointTth, UnknownId};
use koji_db::{KojiDb, ModelError, db::geofence};
use koji_scanner::{
    GenericData,
    entities::{gym, pokestop, spawnpoint, station},
};

pub(crate) mod api_response;
pub(crate) mod auth;
pub(crate) mod error;
pub(crate) mod format;
pub(crate) mod openapi;
pub(crate) mod pagination;
pub(crate) mod response;

pub(crate) fn is_docker() -> io::Result<bool> {
    let mut path = env::current_dir()?;
    path.push("dist");
    let metadata = fs::metadata(path)?;
    Ok(metadata.is_dir())
}

pub(crate) async fn load_collection(
    instance: &String,
    conn: &KojiDb,
) -> Result<FeatureCollection, ModelError> {
    match load_feature(instance, conn).await {
        // Wrap the single loaded feature into a one-element `FeatureCollection`
        // (ring-closed + bbox, as the old matrix `Feature::to_collection` did),
        // without the `To*` matrix.
        Ok(feature) => {
            let bbox = koji_core::KojiGeometry::try_from(feature.clone())
                .ok()
                .and_then(|kg| koji_core::KojiGeometryCollection::new(vec![kg]).geojson_bbox());
            Ok(FeatureCollection {
                bbox: bbox.clone(),
                features: vec![Feature { bbox, ..feature }.ensure_first_last()],
                foreign_members: None,
            })
        }
        Err(err) => Err(err),
    }
}

pub(crate) async fn load_feature(instance: &String, conn: &KojiDb) -> Result<Feature, ModelError> {
    // Fetch the area geofence as a `KojiGeometry` and convert to a geojson
    // `Feature` at the edge (Phase 1 outbound `From<&KojiGeometry>`); the
    // clustering pipeline downstream only reads the feature's geometry (for the
    // bbox + point-in-polygon area filter), so the Koji-native read is faithful.
    // `load_collection` re-applies the bbox + ring-close wrapping exactly as
    // before.
    let kg = geofence::Query::get_one_koji(&conn.koji, instance.to_string()).await?;
    Ok(geojson::Feature::from(&kg))
}

pub(crate) async fn create_or_find_collection(
    instance: &String,
    conn: &KojiDb,
    area: FeatureCollection,
    parent: &Option<UnknownId>,
    data_points: &SingleVec,
) -> Result<FeatureCollection, ModelError> {
    if !data_points.is_empty() {
        // data_points is [lat, lon]; KojiBbox emits the correct geojson order.
        let kb = KojiBbox::from_points(data_points)
            .expect("data_points is non-empty in this branch")
            .trim(6);
        Ok(FeatureCollection {
            bbox: Some(kb.to_geojson_bbox_vec()),
            features: vec![Feature {
                bbox: Some(kb.to_geojson_bbox_vec()),
                geometry: Some(Geometry {
                    value: Value::Polygon(vec![vec![
                        vec![kb.min_lon, kb.min_lat],
                        vec![kb.min_lon, kb.max_lat],
                        vec![kb.max_lon, kb.max_lat],
                        vec![kb.max_lon, kb.min_lat],
                        vec![kb.min_lon, kb.min_lat],
                    ]]),
                    bbox: None,
                    foreign_members: None,
                }),
                ..Feature::default()
            }],
            ..FeatureCollection::default()
        })
    } else if !area.features.is_empty() {
        Ok(area)
    } else if let Some(parent) = parent {
        // A parent's direct children as the area to cluster within. Fetch them
        // Koji-native via `by_parent_koji` (same rows + parent-not-found error as
        // its matrix predecessor) and convert to a geojson `FeatureCollection` at
        // the edge (Phase 1 outbound). The clustering pipeline reads only the
        // features' geometry for the bbox + point-in-polygon filter.
        let coll = geofence::Query::by_parent_koji(&conn.koji, parent).await?;
        Ok(geojson::FeatureCollection::from(&coll))
    } else if !instance.is_empty() {
        load_collection(instance, conn).await
    } else {
        Ok(FeatureCollection::default())
    }
}

pub(crate) async fn points_from_area(
    area: &FeatureCollection,
    category: &str,
    conn: &KojiDb,
    last_seen: u32,
    tth: SpawnpointTth,
) -> Result<Vec<GenericData>, DbErr> {
    if !area.features.is_empty() {
        match category {
            "gym" => gym::Query::area(&conn.scanner, area, last_seen).await,
            "pokestop" => pokestop::Query::area(&conn.scanner, area, last_seen).await,
            "station" => station::Query::area(&conn.scanner, area, last_seen).await,
            "spawnpoint" => spawnpoint::Query::area(&conn.scanner, area, last_seen, tth).await,
            "fort" => {
                // "fort" aggregates gym + pokestop + station results
                let gyms = gym::Query::area(&conn.scanner, area, last_seen).await?;
                let pokestops = pokestop::Query::area(&conn.scanner, area, last_seen).await?;
                let stations = station::Query::area(&conn.scanner, area, last_seen).await?;
                Ok(gyms.into_iter().chain(pokestops).chain(stations).collect())
            }
            _ => Err(DbErr::Custom("Invalid Category".to_string())),
        }
    } else {
        Ok(vec![])
    }
}
