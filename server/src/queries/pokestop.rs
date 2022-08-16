use super::*;
use crate::db::schema::pokestop::dsl::*;
use crate::models::scanner::LatLon;
use crate::models::{
    api::MapBounds,
    scanner::{GenericData, Pokestop},
};
use crate::utils::sql_raw::sql_raw;

pub fn return_generic(items: Vec<Pokestop>) -> Vec<GenericData> {
    items
        .into_iter()
        .map(|item| GenericData::new(item.id, item.lat, item.lon, None))
        .collect()
}

pub fn all(conn: &MysqlConnection) -> Result<Vec<GenericData>, DbError> {
    let items = pokestop.select((id, lat, lon)).load::<Pokestop>(conn)?;
    Ok(return_generic(items))
}

pub fn bound(conn: &MysqlConnection, payload: &MapBounds) -> Result<Vec<GenericData>, DbError> {
    let items = pokestop
        .filter(lat.lt(payload.max_lat))
        .filter(lat.gt(payload.min_lat))
        .filter(lon.lt(payload.max_lon))
        .filter(lon.gt(payload.min_lon))
        .load::<Pokestop>(conn)?;
    Ok(return_generic(items))
}

pub fn area(conn: &MysqlConnection, area: &Vec<LatLon>) -> Result<Vec<GenericData>, DbError> {
    let items = sql_query(sql_raw(area, "pokestop"))
        .load::<Pokestop>(conn)
        .expect("Error loading pokestops");
    Ok(return_generic(items))
}
