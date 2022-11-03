use super::*;
use crate::entities::gym;

use crate::models::scanner::LatLon;
use crate::models::{api::MapBounds, scanner::GenericData};
use crate::utils::sql_raw::sql_raw;

pub fn return_generic(items: Vec<LatLon>) -> Vec<GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| GenericData::new(format!("g{}", i), item.lat, item.lon))
        .collect()
}

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &MapBounds,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .select_only()
        .column(gym::Column::Lat)
        .column(gym::Column::Lon)
        .filter(gym::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(gym::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &Vec<[f64; 2]>,
) -> Result<Vec<GenericData>, DbErr> {
    let items = gym::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            format!("SELECT lat, lon FROM gym {}", sql_raw(area)).as_str(),
            vec![],
        ))
        .into_model::<LatLon>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}
