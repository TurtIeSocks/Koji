use super::*;
use crate::db::schema::spawnpoint::dsl::*;
use crate::models::{
    api::MapBounds,
    scanner::{GenericData, Spawnpoint},
};
use crate::utils::sql_raw::sql_raw;

pub fn return_generic(items: Vec<Spawnpoint>) -> Vec<GenericData> {
    items
        .into_iter()
        .map(|item| {
            GenericData::new(
                item.id.to_string(),
                item.lat,
                item.lon,
                Some(item.despawn_sec.is_some()),
            )
        })
        .collect()
}

pub fn all(conn: &MysqlConnection) -> Result<Vec<GenericData>, DbError> {
    let items = spawnpoint.load::<Spawnpoint>(conn)?;
    Ok(return_generic(items))
}

pub fn bound(conn: &MysqlConnection, payload: &MapBounds) -> Result<Vec<GenericData>, DbError> {
    let items = spawnpoint
        .filter(lat.lt(payload.max_lat))
        .filter(lat.gt(payload.min_lat))
        .filter(lon.lt(payload.max_lon))
        .filter(lon.gt(payload.min_lon))
        .load::<Spawnpoint>(conn)?;
    Ok(return_generic(items))
}

pub fn area(conn: &MysqlConnection, area: &Vec<[f64; 2]>) -> Result<Vec<GenericData>, DbError> {
    let items = sql_query(sql_raw(area, "spawnpoint"))
        .load::<Spawnpoint>(conn)
        .expect("Error loading spawnpoints");
    Ok(return_generic(items))
}
