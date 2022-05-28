use crate::models::{Gym, Instance, MapBounds, Pokestop, Spawnpoint};
use diesel::prelude::*;
use diesel::{sql_query};

type DbError = Box<dyn std::error::Error + Send + Sync>;

pub fn find_spawnpoints(
    conn: &MysqlConnection,
    payload: &MapBounds,
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

pub fn get_instance_route(
    conn: &MysqlConnection,
    instance_name: String,
) -> Result<Instance, DbError> {
    use crate::schema::instance::dsl::*;

    let items = instance
        .filter(name.eq(instance_name))
        .first::<Instance>(conn)?;
    Ok(items)
}

pub fn get_pokestops_in_area(conn: &MysqlConnection, area: String) -> Result<Vec<Pokestop>, DbError> {
    let formatted = format!("SELECT * FROM pokestop WHERE ST_CONTAINS(ST_GeomFromText(\"POLYGON(({:}))\"), POINT(lat, lon))", area);
    println!("{}", formatted);
    let items = sql_query(formatted)
        .load::<Pokestop>(conn)
        .expect("Error loading pokestops");
    Ok(items)
}
