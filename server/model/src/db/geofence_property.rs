//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use crate::utils::json::JsonToModel;

use super::*;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "geofence_property")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub geofence_id: u32,
    pub property_id: u32,
    #[sea_orm(column_type = "Text", nullable)]
    pub value: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::geofence::Entity",
        from = "Column::GeofenceId",
        to = "super::geofence::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Geofence,
    #[sea_orm(
        belongs_to = "super::property::Entity",
        from = "Column::PropertyId",
        to = "super::property::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Property,
}

impl Related<super::geofence::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Geofence.def()
    }
}

impl Related<super::property::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Property.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    /// Creates a new Geofence model, only used from admin panel when creating a single geofence.

    pub async fn upsert(
        db: &DatabaseConnection,
        json: &Json,
        geofence_id: Option<u32>,
    ) -> Result<Model, ModelError> {
        let mut active_model = json.to_geofence_property(geofence_id)?;
        let existing = Entity::find()
            .filter(Column::GeofenceId.eq(active_model.geofence_id.as_ref().clone()))
            .filter(Column::PropertyId.eq(active_model.property_id.as_ref().clone()))
            .one(db)
            .await?;
        Ok(if let Some(existing) = existing {
            active_model.id = Set(existing.id);
            active_model.update(db).await?
        } else {
            active_model.insert(db).await?
        })
    }

    pub async fn upsert_many(
        db: &DatabaseConnection,
        incoming: &Vec<Json>,
        geofence_id: Option<u32>,
    ) -> Result<Vec<Model>, ModelError> {
        let models = future::try_join_all(
            incoming
                .into_iter()
                .map(|json| Query::upsert(db, json, geofence_id)),
        )
        .await?;
        Ok(models)
    }
}
