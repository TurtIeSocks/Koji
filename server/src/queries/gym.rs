use super::*;
use crate::db::schema::gym::dsl::*;
use crate::models::scanner::Gym;

pub fn query_all_gyms(conn: &MysqlConnection) -> Result<Vec<Gym>, DbError> {
    let items = gym.select((id, lat, lon)).load::<Gym>(conn)?;
    Ok(items)
}
