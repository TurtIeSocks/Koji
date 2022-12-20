use super::*;

use crate::{
    entity::pokestop,
    models::{api::BoundsArg, point_struct::PointStruct, scanner::GenericData},
    utils::{self, normalize},
};

pub async fn all(conn: &DatabaseConnection, last_seen: u32) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .filter(pokestop::Column::Updated.gt(last_seen))
        .filter(pokestop::Column::Deleted.eq(false))
        .filter(pokestop::Column::Enabled.eq(true))
        .limit(2_000_000)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .filter(pokestop::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(pokestop::Column::Lon.between(payload.min_lon, payload.max_lon))
        .filter(pokestop::Column::Updated.gt(last_seen))
        .filter(pokestop::Column::Deleted.eq(false))
        .filter(pokestop::Column::Enabled.eq(true))
        .limit(2_000_000)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &FeatureCollection,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM pokestop WHERE enabled = 1 AND deleted = 0 AND updated >= {} AND ({}) LIMIT 2000000", last_seen, utils::sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "p"))
}
