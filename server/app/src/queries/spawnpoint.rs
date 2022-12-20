use super::*;

use crate::{
    entity::spawnpoint,
    models::{
        api::BoundsArg,
        scanner::{GenericData, Spawnpoint},
    },
    utils::{self, normalize},
};

pub async fn all(conn: &DatabaseConnection, last_seen: u32) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .limit(2_000_000)
        .filter(spawnpoint::Column::LastSeen.gt(last_seen))
        .into_model::<Spawnpoint>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .filter(spawnpoint::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(spawnpoint::Column::Lon.between(payload.min_lon, payload.max_lon))
        .filter(spawnpoint::Column::LastSeen.gt(last_seen))
        .limit(2_000_000)
        .into_model::<Spawnpoint<f64>>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &FeatureCollection,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!(
                "SELECT lat, lon, despawn_sec FROM spawnpoint WHERE last_seen >= {} AND ({}) LIMIT 2000000",
                last_seen,
                utils::sql_raw(area)
            )
            .as_str(),
            vec![],
        ))
        .into_model::<Spawnpoint>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}
