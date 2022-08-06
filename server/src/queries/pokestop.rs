use super::*;
use crate::db::schema::pokestop::dsl::*;
use crate::models::scanner::Pokestop;

pub fn query_all_pokestops(conn: &MysqlConnection) -> Result<Vec<Pokestop>, DbError> {
    let items = pokestop
        .select((id, lat, lon, name))
        .load::<Pokestop>(conn)?;
    Ok(items)
}

pub fn query_area_pokestops(
    conn: &MysqlConnection,
    area: String,
) -> Result<Vec<Pokestop>, DbError> {
    let formatted = format!("SELECT * FROM pokestop WHERE ST_CONTAINS(ST_GeomFromText(\"POLYGON(({:}))\"), POINT(lat, lon))", area);
    let items = sql_query(formatted)
        .load::<Pokestop>(conn)
        .expect("Error loading pokestops");
    Ok(items)
}
