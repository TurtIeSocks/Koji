use std::str::FromStr;

use num_traits::Float;

use super::*;
use crate::entities::area;
use crate::models::scanner::GenericInstance;
use crate::utils::convert::{arrays::parse_flat_text, normalize};

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericInstance>, DbErr> {
    let items = area::Entity::find().all(conn).await?;
    Ok(normalize::area(items))
}

pub async fn route<T>(
    conn: &DatabaseConnection,
    area_name: &String,
) -> Result<Vec<Vec<[T; 2]>>, DbErr>
where
    T: Float + FromStr,
{
    let item = area::Entity::find()
        .filter(area::Column::Name.contains(area_name))
        .one(conn)
        .await?;
    if item.is_some() {
        let item = item.unwrap();
        if item.geofence.is_some() && !item.geofence.clone().unwrap().is_empty() {
            let data = parse_flat_text(item.geofence.clone().unwrap().as_str());
            Ok(data)
        } else {
            Err(DbErr::Custom("No geofence found".to_string()))
        }
    } else {
        Err(DbErr::Custom("Area not found".to_string()))
    }
}
