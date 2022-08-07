use super::*;
use crate::db::schema::instance::dsl::*;
use crate::models::scanner::Instance;

pub fn query_all_instances(conn: &MysqlConnection) -> Result<Vec<Instance>, DbError> {
    let items = instance.load::<Instance>(conn)?;
    Ok(items)
}

pub fn query_instance_route(
    conn: &MysqlConnection,
    instance_name: &String,
) -> Result<Instance, DbError> {
    let items = instance
        .filter(name.eq(instance_name))
        .first::<Instance>(conn)?;

    Ok(items)
}
