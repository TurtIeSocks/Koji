use crate::models::{Body, Gym, Instance, Pokestop, Spawnpoint};
use diesel::prelude::*;

type DbError = Box<dyn std::error::Error + Send + Sync>;

pub fn find_spawnpoints(
    conn: &MysqlConnection,
    payload: &Body,
) -> Result<Vec<Spawnpoint>, DbError> {
    use crate::schema::spawnpoint::dsl::*;

    let items = spawnpoint
        .filter(lat.lt(payload.max_lat))
        .filter(lat.gt(payload.min_lat))
        .filter(lon.lt(payload.max_lon))
        .filter(lon.gt(payload.min_lon))
        .load::<Spawnpoint>(conn)?;
    Ok(items)
}

pub fn find_all_spawnpoints(conn: &MysqlConnection) -> Result<Vec<Spawnpoint>, DbError> {
    use crate::schema::spawnpoint::dsl::*;

    let items = spawnpoint.load::<Spawnpoint>(conn)?;
    Ok(items)
}

pub fn find_all_pokestops(conn: &MysqlConnection) -> Result<Vec<Pokestop>, DbError> {
    use crate::schema::pokestop::dsl::*;

    let items = pokestop.select((id, lat, lon)).load::<Pokestop>(conn)?;
    Ok(items)
}

pub fn find_all_gyms(conn: &MysqlConnection) -> Result<Vec<Gym>, DbError> {
    use crate::schema::gym::dsl::*;

    let items = gym.select((id, lat, lon)).load::<Gym>(conn)?;
    Ok(items)
}

pub fn find_all_instances(conn: &MysqlConnection) -> Result<Vec<Instance>, DbError> {
    use crate::schema::instance::dsl::*;

    let items = instance.load::<Instance>(conn)?;
    Ok(items)
}
