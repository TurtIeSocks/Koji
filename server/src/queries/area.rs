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
    if let Some(item) = item {
        if let Some(geofence) = item.geofence {
            if !geofence.is_empty() {
                Ok(collection::from_feature(parse_text(
                    geofence.as_str(),
                    Some(item.name),
                    Some(&Type::AutoQuest),
                )))
            } else {
                Err(DbErr::Custom("Geofence is empty".to_string()))
            }
        } else {
            Err(DbErr::Custom("No geofence found".to_string()))
        }
    } else {
        Err(DbErr::Custom("Area not found".to_string()))
    }
}
