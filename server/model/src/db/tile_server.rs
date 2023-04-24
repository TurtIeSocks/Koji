use super::*;

use sea_orm::entity::prelude::*;
use serde_json::json;
use std::str::FromStr;

use crate::{
    api::args::AdminReqParsed,
    error::ModelError,
    utils::{json::JsonToModel, parse_order},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "tile_server")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub name: String,
    pub url: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u32>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(id)).one(db).await?,
        };
        if let Some(record) = record {
            Ok(record)
        } else {
            Err(ModelError::Geofence("Does not exist".to_string()))
        }
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => Ok(json!(record)),
            Err(err) => Err(err),
        }
    }

    pub async fn paginate(
        db: &DatabaseConnection,
        args: AdminReqParsed,
    ) -> Result<PaginateResults<Vec<Json>>, DbErr> {
        let paginator = Entity::find()
            .order_by(
                Column::from_str(&args.sort_by).unwrap_or(Column::Name),
                parse_order(&args.order),
            )
            .filter(Column::Name.like(format!("%{}%", args.q).as_str()))
            .paginate(db, args.per_page);
        let total = paginator.num_items_and_pages().await?;

        let results: Vec<Model> = match paginator.fetch_page(args.page).await {
            Ok(results) => results,
            Err(err) => {
                log::error!("[project] Error paginating, {:?}", err);
                vec![]
            }
        };

        let results: Vec<Json> = results.into_iter().map(|model| json!(model)).collect();

        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: total.number_of_pages == args.page + 1,
            has_next: args.page + 1 < total.number_of_pages,
        })
    }

    pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    pub async fn get_json_cache(db: &DatabaseConnection) -> Result<Vec<sea_orm::JsonValue>, DbErr> {
        let results = Entity::find()
            .order_by(Column::Name, Order::Asc)
            .all(db)
            .await?;

        Ok(results.into_iter().map(|model| json!(model)).collect())
    }

    pub async fn upsert(
        db: &DatabaseConnection,
        id: u32,
        new_model: Json,
    ) -> Result<Model, ModelError> {
        let old_model: Option<Model> = Entity::find_by_id(id).one(db).await?;
        let mut new_model = new_model.to_tileserver()?;

        let model = if let Some(old_model) = old_model {
            new_model.id = Set(old_model.id);
            new_model.update(db).await?
        } else {
            new_model.insert(db).await?
        };
        Ok(model)
    }

    pub async fn upsert_json_return(
        db: &DatabaseConnection,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
        let result = Query::upsert(db, id, json).await?;
        Ok(json!(result))
    }

    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        let record = Entity::delete_by_id(id).exec(db).await?;
        Ok(record)
    }

    pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
        Ok(Entity::find()
            .filter(Column::Name.like(format!("%{}%", search).as_str()))
            .into_json()
            .all(db)
            .await?)
    }
}
