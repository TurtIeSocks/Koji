use super::{m20221207_120629_create_geofence::Geofence, m20221207_122452_create_project::Project};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(GeofenceProject::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GeofenceProject::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GeofenceProject::GeofenceId)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeofenceProject::ProjectId)
                            .unsigned()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_geofence_id")
                            .from(GeofenceProject::Table, GeofenceProject::GeofenceId)
                            .to(Geofence::Table, Geofence::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_project_id")
                            .from(GeofenceProject::Table, GeofenceProject::ProjectId)
                            .to(Project::Table, Project::Id),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(GeofenceProject::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum GeofenceProject {
    Table,
    Id,
    GeofenceId,
    ProjectId,
}
