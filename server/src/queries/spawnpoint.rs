use super::*;
use crate::db::schema::spawnpoint::dsl::*;
use crate::models::{api::MapBounds, scanner::Spawnpoint};

pub fn query_all_spawnpoints(conn: &MysqlConnection) -> Result<Vec<Spawnpoint>, DbError> {
    let items = spawnpoint.load::<Spawnpoint>(conn)?;
    Ok(items)
}

pub fn query_bound_spawnpoints(
    conn: &MysqlConnection,
    payload: &MapBounds,
) -> Result<Vec<Spawnpoint>, DbError> {
    let items = spawnpoint
        .filter(lat.lt(payload.max_lat))
        .filter(lat.gt(payload.min_lat))
        .filter(lon.lt(payload.max_lon))
        .filter(lon.gt(payload.min_lon))
        .load::<Spawnpoint>(conn)?;
    Ok(items)
}
