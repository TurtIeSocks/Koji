use super::*;

use chrono::Utc;
use entity::sea_orm_active_enums::Type;
use geojson::GeoJson;
use migration::Expr;
use models::ToCollection;
use sea_orm::{ActiveModelTrait, DeleteResult, Order, PaginatorTrait, QueryOrder, Set};
use serde::Serialize;

use crate::{entity::geofence, models::scanner::IdName};

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

pub async fn route(
    conn: &DatabaseConnection,
    instance_name: &String,
) -> Result<FeatureCollection, DbErr> {
    let items = geofence::Entity::find()
        .filter(geofence::Column::Name.contains(instance_name))
        .one(conn)
        .await?;
    if let Some(items) = items {
        let feature = Feature::from_json_value(items.area);
        return match feature {
            Ok(feat) => Ok(feat.to_collection(None, Some(&Type::AutoQuest))),
            Err(err) => Err(DbErr::Custom(err.to_string())),
        };
    } else {
        Err(DbErr::Custom("Instance not found".to_string()))
    }
}

pub async fn save(
    conn: &DatabaseConnection,
    area: FeatureCollection,
) -> Result<(usize, usize), DbErr> {
    let existing = geofence::Entity::find()
        .select_only()
        .column(geofence::Column::Id)
        .column(geofence::Column::Name)
        .into_model::<IdName>()
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

#[derive(Debug, Serialize)]
pub struct PaginateResults<T> {
    results: Vec<T>,
    total: usize,
    has_next: bool,
    has_prev: bool,
}

pub struct Query;

impl Query {
    pub async fn paginate(
        db: &DatabaseConnection,
        page: usize,
        posts_per_page: usize,
        sort_by: geofence::Column,
        order_by: Order,
    ) -> Result<PaginateResults<geofence::Model>, DbErr> {
        let paginator = geofence::Entity::find()
            .order_by(sort_by, order_by)
            .paginate(db, posts_per_page);
        let total = paginator.num_items_and_pages().await?;

        let results = if let Ok(stuff) = paginator.fetch_page(page).await.map(|p| p) {
            stuff
        } else {
            vec![]
        };
        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: total.number_of_pages == page + 1,
            has_next: page + 1 < total.number_of_pages,
        })
    }
    pub async fn get_one(
        db: &DatabaseConnection,
        id: u32,
    ) -> Result<Option<geofence::Model>, DbErr> {
        let record = geofence::Entity::find_by_id(id).one(db).await?;
        Ok(record)
    }

    pub async fn update(
        db: &DatabaseConnection,
        id: u32,
        updated_geofence: geofence::Model,
    ) -> Result<geofence::Model, DbErr> {
        let old_fence: Option<geofence::Model> = geofence::Entity::find_by_id(id).one(db).await?;
        let mut old_fence: geofence::ActiveModel = old_fence.unwrap().into();
        old_fence.name = Set(updated_geofence.name.to_owned());
        old_fence.area = Set(updated_geofence.area.to_owned());
        old_fence.update(db).await
    }
    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        let record = geofence::Entity::delete_by_id(id).exec(db).await?;
        Ok(record)
    }
}
