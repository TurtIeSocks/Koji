use super::*;
use crate::entities::spawnpoint;
use crate::models::{
    api::MapBounds,
    scanner::{GenericData, TrimmedSpawn},
};
use crate::utils;

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .into_model::<TrimmedSpawn>()
        .all(conn)
        .await?;
    Ok(utils::convert::normalize::spawnpoint(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &MapBounds,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .filter(spawnpoint::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(spawnpoint::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<TrimmedSpawn<f64>>()
        .all(conn)
        .await?;
    Ok(utils::convert::normalize::spawnpoint(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: Vec<Vec<[f64; 2]>>,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!(
                "SELECT lat, lon, despawn_sec FROM spawnpoint {}",
                utils::sql_raw(area)
            )
            .as_str(),
            vec![],
        ))
        .into_model::<TrimmedSpawn>()
        .all(conn)
        .await?;
    Ok(utils::convert::normalize::spawnpoint(items))
}
