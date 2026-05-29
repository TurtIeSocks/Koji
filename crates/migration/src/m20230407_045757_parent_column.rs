use sea_orm_migration::prelude::*;

use crate::m20221207_120629_create_geofence::Geofence;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_18] adding parent column to geofence table");
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .add_column(ColumnDef::new(Geofence::Parent).unsigned())
                    .drop_column(Geofence::Area)
                    .add_foreign_key(
                        &TableForeignKey::new()
                            .name("FK_parent_id")
                            .from_tbl(Geofence::Table)
                            .from_col(Geofence::Parent)
                            .to_tbl(Geofence::Table)
                            .to_col(Geofence::Id)
                            .to_owned(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_18] adding parent column to geofence table");
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .drop_column(Geofence::Parent)
                    .add_column(ColumnDef::new(Geofence::Area).json())
                    .drop_foreign_key(Alias::new("FK_parent_id"))
                    .to_owned(),
            )
            .await
    }
}
