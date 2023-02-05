use sea_orm_migration::prelude::*;

use crate::{
    m20221207_120629_create_geofence::Geofence, m20221207_122452_create_project::Project,
    m20230117_010422_routes_table::Route,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(
                        ColumnDef::new(Geofence::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .modify_column(
                        ColumnDef::new(Geofence::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra(
                                "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Project::Table)
                    .modify_column(
                        ColumnDef::new(Project::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .modify_column(
                        ColumnDef::new(Project::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra(
                                "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Route::Table)
                    .modify_column(
                        ColumnDef::new(Route::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .modify_column(
                        ColumnDef::new(Route::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra(
                                "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(ColumnDef::new(Geofence::CreatedAt).timestamp().not_null())
                    .modify_column(ColumnDef::new(Geofence::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Project::Table)
                    .modify_column(ColumnDef::new(Project::CreatedAt).timestamp().not_null())
                    .modify_column(ColumnDef::new(Project::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .alter_table(
                Table::alter()
                    .table(Route::Table)
                    .modify_column(ColumnDef::new(Route::CreatedAt).timestamp().not_null())
                    .modify_column(ColumnDef::new(Route::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
