use geojson::FeatureCollection;

use super::*;
use crate::entities::area;
use crate::entities::sea_orm_active_enums::Type;
use crate::utils::{
    convert::{collection, normalize},
    parse_text,
};

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<Feature>, DbErr> {
    let items = area::Entity::find().all(conn).await?;
    Ok(normalize::area(items))
}

pub async fn route(
    conn: &DatabaseConnection,
    area_name: &String,
) -> Result<FeatureCollection, DbErr> {
    let item = area::Entity::find()
        .filter(area::Column::Name.contains(area_name))
        .one(conn)
        .await?;
    if item.is_some() {
        let item = item.unwrap();
        if item.geofence.is_some() && !item.geofence.clone().unwrap().is_empty() {
            Ok(collection::from_feature(parse_text(
                item.geofence.unwrap().as_str(),
                Some(item.name),
                Some(Type::AutoQuest),
            )))
        } else {
            Err(DbErr::Custom("No geofence found".to_string()))
        }
    } else {
        Err(DbErr::Custom("Area not found".to_string()))
    }
}
