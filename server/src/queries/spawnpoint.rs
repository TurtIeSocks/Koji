use super::*;

use crate::{
    entities::spawnpoint,
    models::{
        api::BoundsArg,
        scanner::{GenericData, Spawnpoint},
    },
    utils::{self, convert::normalize},
};

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .into_model::<Spawnpoint>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .filter(spawnpoint::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(spawnpoint::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<Spawnpoint<f64>>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: FeatureCollection,
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
        .into_model::<Spawnpoint>()
        .all(conn)
        .await?;
    Ok(normalize::spawnpoint(items))
}
