use super::*;
use crate::entities::pokestop;
use crate::models::{
    api::MapBounds,
    scanner::{GenericData, LatLon},
};
use crate::utils::sql_raw::sql_raw;

pub fn return_generic(items: Vec<LatLon>) -> Vec<GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| GenericData::new(format!("p{}", i), item.lat, item.lon))
        .collect()
}

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &MapBounds,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .select_only()
        .column(pokestop::Column::Lat)
        .column(pokestop::Column::Lon)
        .filter(pokestop::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(pokestop::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &Vec<[f64; 2]>,
) -> Result<Vec<GenericData>, DbErr> {
    let items = pokestop::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM pokestop {}", sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}
