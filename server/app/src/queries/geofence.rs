use super::*;

use chrono::Utc;
use geojson::GeoJson;
use migration::{Expr, Order};
use sea_orm::{FromQueryResult, QueryOrder, Set};

use crate::entity::geofence;

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<Feature>, DbErr> {
    let items = geofence::Entity::find()
        // .find_with_related(project::Entity)
        .order_by(geofence::Column::Name, Order::Asc)
        .all(conn)
        .await?;
    let items: Vec<Feature> = items
        .into_iter()
        .map(|item| {
            let feature = Feature::from_json_value(item.area);
            let mut feature = if feature.is_ok() {
                feature.unwrap()
            } else {
                Feature::default()
            };
            feature.set_property("name", item.name);
            feature.set_property("id", item.id);
            feature
        })
        .collect();
    Ok(items)
}

#[derive(FromQueryResult)]
struct Geofence {
    id: i32,
    name: String,
}

pub async fn save(
    conn: &DatabaseConnection,
    area: FeatureCollection,
) -> Result<(usize, usize), DbErr> {
    let existing = geofence::Entity::find()
        .select_only()
        .column(geofence::Column::Id)
        .column(geofence::Column::Name)
        .into_model::<Geofence>()
        .all(conn)
        .await?;

    let mut inserts: Vec<geofence::ActiveModel> = vec![];
    let mut update_len = 0;

    for feat in area.into_iter() {
        if let Some(name) = feat.property("name") {
            if let Some(name) = name.as_str() {
                let area = GeoJson::Feature(feat.clone()).to_json_value();
                if let Some(area) = area.as_object() {
                    let area = sea_orm::JsonValue::Object(area.to_owned());
                    let name = name.to_string();
                    let is_update = existing.iter().find(|entry| entry.name == name);

                    if let Some(entry) = is_update {
                        geofence::Entity::update_many()
                            .col_expr(geofence::Column::Area, Expr::value(area))
                            .filter(geofence::Column::Id.eq(entry.id))
                            .exec(conn)
                            .await?;
                        update_len += 1;
                    } else {
                        inserts.push(geofence::ActiveModel {
                            name: Set(name.to_string()),
                            area: Set(area),
                            created_at: Set(Utc::now()),
                            updated_at: Set(Utc::now()),
                            ..Default::default()
                        })
                    }
                }
            }
        }
    }
    let insert_len = inserts.len();
    if !inserts.is_empty() {
        geofence::Entity::insert_many(inserts).exec(conn).await?;
    }
    Ok((insert_len, update_len))
}
