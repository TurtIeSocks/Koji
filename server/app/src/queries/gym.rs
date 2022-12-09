use super::*;

use crate::{
    entity::gym,
    models::{api::BoundsArg, point_struct::PointStruct, scanner::GenericData},
    utils::{self, normalize},
};

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &BoundsArg,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .filter(gym::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(gym::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &FeatureCollection,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM gym {}", utils::sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<PointStruct>()
        .all(conn)
        .await?;
    Ok(normalize::fort(items, "g"))
}
