//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1

use crate::api::{EnsureProperties, ToCollection};

use super::{sea_orm_active_enums::Type, *};

use geojson::GeoJson;
use sea_orm::{entity::prelude::*, InsertResult};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "geofence")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u32,
    pub name: String,
    pub area: Json,
    pub mode: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::project::Entity")]
    Project,
}

impl Related<project::Entity> for Entity {
    fn to() -> RelationDef {
        geofence_project::Relation::Project.def()
    }
    fn via() -> Option<RelationDef> {
        Some(geofence_project::Relation::Geofence.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    pub async fn paginate(
        db: &DatabaseConnection,
        page: usize,
        posts_per_page: usize,
        sort_by: Column,
        order_by: Order,
    ) -> Result<PaginateResults<Model>, DbErr> {
        let paginator = Entity::find()
            .order_by(sort_by, order_by)
            .paginate(db, posts_per_page);
        let total = paginator.num_items_and_pages().await?;

        let results = if let Ok(paginated_results) = paginator.fetch_page(page).await.map(|p| p) {
            paginated_results
        } else {
            vec![]
        };
        let results = future::try_join_all(
            results
                .into_iter()
                .map(|result| Query::get_related_projects(db, result)),
        )
        .await
        .unwrap();

        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: total.number_of_pages == page + 1,
            has_next: page + 1 < total.number_of_pages,
        })
    }

    pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    pub async fn get_all_no_fences(db: &DatabaseConnection) -> Result<Vec<NoFence>, DbErr> {
        Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .column(Column::Mode)
            .column(Column::CreatedAt)
            .column(Column::UpdatedAt)
            .order_by(Column::Name, Order::Asc)
            .into_model::<NoFence>()
            .all(db)
            .await
    }

    pub async fn get_related_projects(
        db: &DatabaseConnection,
        model: Model,
    ) -> Result<(Model, Vec<NameId>), DbErr> {
        let related = model
            .find_related(project::Entity)
            .select_only()
            .column(project::Column::Id)
            .column(project::Column::Name)
            .into_model::<NameId>()
            .all(db)
            .await?;
        Ok((model, related))
    }

    pub async fn create(db: &DatabaseConnection, new_project: Model) -> Result<Model, DbErr> {
        ActiveModel {
            name: Set(new_project.name.to_owned()),
            area: Set(new_project.area),
            mode: Set(new_project.mode),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(db)
        .await
    }

    pub async fn get_one(db: &DatabaseConnection, id: u32) -> Result<(Model, Vec<NameId>), DbErr> {
        let record = Entity::find_by_id(id).one(db).await?;
        let record = record.unwrap();
        Query::get_related_projects(db, record).await
    }

    pub async fn update(
        db: &DatabaseConnection,
        id: u32,
        new_model: Model,
    ) -> Result<Model, DbErr> {
        let old_model: Option<Model> = Entity::find_by_id(id).one(db).await?;
        let mut old_model: ActiveModel = old_model.unwrap().into();
        old_model.name = Set(new_model.name.to_owned());
        old_model.area = Set(new_model.area);
        old_model.mode = Set(new_model.mode);
        old_model.updated_at = Set(Utc::now());
        old_model.update(db).await
    }

    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        let record = Entity::delete_by_id(id).exec(db).await?;
        Ok(record)
    }

    pub async fn as_collection(conn: &DatabaseConnection) -> Result<FeatureCollection, DbErr> {
        let items = Entity::find()
            .order_by(Column::Name, Order::Asc)
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

        Ok(items.to_collection(None, None))
    }

    pub async fn route(
        conn: &DatabaseConnection,
        instance_name: &String,
    ) -> Result<Feature, DbErr> {
        let items = Entity::find()
            .filter(Column::Name.contains(instance_name))
            .one(conn)
            .await?;
        if let Some(items) = items {
            let feature = Feature::from_json_value(items.area);
            return match feature {
                Ok(feat) => {
                    Ok(feat
                        .ensure_properties(Some(instance_name.to_string()), Some(&Type::AutoQuest)))
                }
                Err(err) => Err(DbErr::Custom(err.to_string())),
            };
        } else {
            Err(DbErr::Custom("Instance not found".to_string()))
        }
    }

    async fn insert_related_projects(
        conn: &DatabaseConnection,
        projects: Vec<u64>,
        id: u32,
    ) -> Result<InsertResult<geofence_project::ActiveModel>, DbErr> {
        let projects: Vec<geofence_project::ActiveModel> = projects
            .into_iter()
            .map(|project| geofence_project::ActiveModel {
                project_id: Set(project as u32),
                geofence_id: Set(id),
                ..Default::default()
            })
            .collect();
        geofence_project::Entity::insert_many(projects)
            .exec(conn)
            .await
    }

    pub async fn save(
        conn: &DatabaseConnection,
        area: FeatureCollection,
    ) -> Result<(usize, usize), DbErr> {
        let existing = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .into_model::<NameId>()
            .all(conn)
            .await?;

        let mut inserts = 0;
        let mut update_len = 0;

        for feat in area.into_iter() {
            if let Some(name) = feat.property("name") {
                if let Some(name) = name.as_str() {
                    let mut feat = feat.clone();
                    feat.id = None;
                    let mode = if let Some(r#type) = feat.property("type") {
                        if let Some(r#type) = r#type.as_str() {
                            Some(r#type.to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    let projects: Option<Vec<u64>> =
                        if let Some(projects) = feat.property("projects") {
                            if let Some(projects) = projects.as_array() {
                                Some(
                                    projects
                                        .iter()
                                        .filter_map(|project| {
                                            if let Some(project) = project.as_u64() {
                                                Some(project)
                                            } else {
                                                None
                                            }
                                        })
                                        .collect(),
                                )
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                    feat.remove_property("name");
                    feat.remove_property("type");
                    feat.remove_property("projects");
                    let area = GeoJson::Feature(feat).to_json_value();
                    if let Some(area) = area.as_object() {
                        let area = sea_orm::JsonValue::Object(area.to_owned());
                        let name = name.to_string();
                        let is_update = existing.iter().find(|entry| entry.name == name);

                        if let Some(entry) = is_update {
                            let old_model: Option<Model> =
                                Entity::find_by_id(entry.id).one(conn).await?;
                            let mut old_model: ActiveModel = old_model.unwrap().into();
                            old_model.area = Set(area);
                            old_model.updated_at = Set(Utc::now());
                            let model = old_model.update(conn).await?;

                            if let Some(projects) = projects {
                                Query::insert_related_projects(conn, projects, model.id).await?;
                            };
                            update_len += 1;
                        } else {
                            let model = ActiveModel {
                                name: Set(name.to_string()),
                                area: Set(area),
                                mode: Set(mode),
                                created_at: Set(Utc::now()),
                                updated_at: Set(Utc::now()),
                                ..Default::default()
                            }
                            .insert(conn)
                            .await?;

                            if let Some(projects) = projects {
                                Query::insert_related_projects(conn, projects, model.id).await?;
                            };
                            inserts += 1;
                        }
                    }
                }
            }
        }
        Ok((inserts, update_len))
    }

    pub async fn by_project(
        conn: &DatabaseConnection,
        project_id: String,
    ) -> Result<Vec<Feature>, DbErr> {
        let items = Entity::find()
            .order_by(Column::Name, Order::Asc)
            .left_join(project::Entity)
            .filter(project::Column::Name.eq(project_id))
            .all(conn)
            .await?;

        let items: Vec<Feature> = items
            .into_iter()
            .map(|item| {
                let feature = Feature::from_json_value(item.area);
                let feature = if feature.is_ok() {
                    feature.unwrap()
                } else {
                    Feature::default()
                };
                feature
            })
            .collect();
        Ok(items)
    }
}
