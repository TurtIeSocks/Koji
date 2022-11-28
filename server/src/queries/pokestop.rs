use super::*;

use crate::{
    entities::pokestop,
    models::{api::BoundsArg, scanner::GenericData, PointStruct},
    utils::{self, convert::normalize},
};

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .filter(pokestop::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(pokestop::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: FeatureCollection,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM pokestop {}", utils::sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}
