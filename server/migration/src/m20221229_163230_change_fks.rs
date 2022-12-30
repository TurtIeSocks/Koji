use super::{m20221207_120629_create_geofence::Geofence, m20221207_122452_create_project::Project};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Table::alter()
            .table(GeofenceProject::Table)
            .drop_foreign_key(Alias::new("FK_geofence_id"))
            .drop_foreign_key(Alias::new("FK_project_id"))
            .to_owned();

        manager.alter_table(table).await?;

        let foreign_key_char = TableForeignKey::new()
            .name("FK_geofence_id")
            .from_tbl(GeofenceProject::Table)
            .from_col(GeofenceProject::GeofenceId)
            .to_tbl(Geofence::Table)
            .to_col(Geofence::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::Cascade)
            .to_owned();

        let foreign_key_font = TableForeignKey::new()
            .name("FK_project_id")
            .from_tbl(GeofenceProject::Table)
            .from_col(GeofenceProject::ProjectId)
            .to_tbl(Project::Table)
            .to_col(Project::Id)
            .on_delete(ForeignKeyAction::Cascade)
            .on_update(ForeignKeyAction::Cascade)
            .to_owned();

        let table = Table::alter()
            .table(GeofenceProject::Table)
            .add_foreign_key(&foreign_key_char)
            .add_foreign_key(&foreign_key_font)
            .to_owned();

        manager.alter_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = Table::alter()
            .table(GeofenceProject::Table)
            .drop_foreign_key(Alias::new("FK_geofence_id"))
            .drop_foreign_key(Alias::new("FK_project_id"))
            .to_owned();

        manager.alter_table(table).await?;

        let foreign_key_char = TableForeignKey::new()
            .name("FK_geofence_id")
            .from_tbl(GeofenceProject::Table)
            .from_col(GeofenceProject::GeofenceId)
            .to_tbl(Geofence::Table)
            .to_col(Geofence::Id)
            .to_owned();

        let foreign_key_font = TableForeignKey::new()
            .name("FK_project_id")
            .from_tbl(GeofenceProject::Table)
            .from_col(GeofenceProject::ProjectId)
            .to_tbl(Project::Table)
            .to_col(Project::Id)
            .to_owned();

        let table = Table::alter()
            .table(GeofenceProject::Table)
            .add_foreign_key(&foreign_key_char)
            .add_foreign_key(&foreign_key_font)
            .to_owned();

        manager.alter_table(table).await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum GeofenceProject {
    Table,
    // Id,
    GeofenceId,
    ProjectId,
}
