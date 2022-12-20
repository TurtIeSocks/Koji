use super::*;

use crate::{
    entity::gym,
    models::{api::BoundsArg, point_struct::PointStruct, scanner::GenericData},
    utils::{self, normalize},
};

pub async fn all(conn: &DatabaseConnection, last_seen: u32) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .filter(gym::Column::Updated.gt(last_seen))
        .filter(gym::Column::Deleted.eq(false))
        .filter(gym::Column::Enabled.eq(true))
        .limit(2_000_000)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .filter(gym::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(gym::Column::Lon.between(payload.min_lon, payload.max_lon))
        .filter(gym::Column::Updated.gt(last_seen))
        .filter(gym::Column::Deleted.eq(false))
        .filter(gym::Column::Enabled.eq(true))
        .limit(2_000_000)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &FeatureCollection,
    last_seen: u32,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM gym WHERE enabled = 1 AND deleted = 0 AND updated >= {} AND ({}) LIMIT 2000000", last_seen, utils::sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}
