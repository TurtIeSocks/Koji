use super::*;
use crate::db::{schema::instance::dsl::*, sql_types::InstanceType};
use crate::models::scanner::Instance;

pub fn query_all_instances(
    conn: &MysqlConnection,
    incoming: String,
) -> Result<Vec<Instance>, DbError> {
    let instance_type: std::option::Option<InstanceType> = if incoming == "auto_quest" {
        Some(InstanceType::auto_quest)
    } else if incoming == "circle_pokemon" {
        Some(InstanceType::circle_pokemon)
    } else if incoming == "circle_raid" {
        Some(InstanceType::circle_raid)
    } else {
        None
    };
    let items = if instance_type.is_some() {
        instance
            .filter(type_.eq(instance_type.unwrap()))
            .load::<Instance>(conn)
            .expect("Error loading instances")
    } else {
        instance
            .load::<Instance>(conn)
            .expect("Error loading instances")
    };
    Ok(items)
}

pub fn query_instance_route(
    conn: &MysqlConnection,
    instance_name: &String,
) -> Result<Instance, DbError> {
    let items = instance
        .filter(name.eq(instance_name))
        .first::<Instance>(conn)
        .expect("No instance found");
    Ok(items)
}
