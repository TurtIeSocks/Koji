use super::*;

use sea_orm::IntoActiveModel;
use sea_orm::entity::prelude::*;

use crate::error::ModelError;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "plugin_config")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    #[sea_orm(column_type = "Json", nullable)]
    pub args_default: Option<Json>,
    pub description: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    /// All overlay rows (feeds the registry build).
    pub async fn all(db: &DatabaseConnection) -> Result<Vec<Model>, ModelError> {
        Ok(Entity::find().all(db).await?)
    }

    /// One overlay row by `(kind, name)`, if present.
    pub async fn get_one(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
    ) -> Result<Option<Model>, ModelError> {
        Ok(Entity::find()
            .filter(Column::Kind.eq(kind))
            .filter(Column::Name.eq(name))
            .one(db)
            .await?)
    }

    /// Insert or update the overlay for `(kind, name)`. `None` fields leave the
    /// stored value unchanged on update / fall to column defaults on insert.
    pub async fn upsert(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
        enabled: Option<bool>,
        args_default: Option<Json>,
        description: Option<String>,
    ) -> Result<Model, ModelError> {
        let existing = Query::get_one(db, kind, name).await?;
        let mut active = match existing {
            Some(model) => model.into_active_model(),
            None => ActiveModel {
                kind: Set(kind.to_string()),
                name: Set(name.to_string()),
                ..Default::default()
            },
        };
        if let Some(enabled) = enabled {
            active.enabled = Set(enabled);
        }
        if let Some(args) = args_default {
            active.args_default = Set(Some(args));
        }
        if let Some(desc) = description {
            active.description = Set(Some(desc));
        }
        active.save(db).await?;
        // Read back the full row (save() returns the active model).
        Query::get_one(db, kind, name)
            .await?
            .ok_or_else(|| ModelError::Custom("plugin_config upsert read-back failed".to_string()))
    }

    pub async fn delete(
        db: &DatabaseConnection,
        kind: &str,
        name: &str,
    ) -> Result<sea_orm::DeleteResult, ModelError> {
        Ok(Entity::delete_many()
            .filter(Column::Kind.eq(kind))
            .filter(Column::Name.eq(name))
            .exec(db)
            .await?)
    }
}
