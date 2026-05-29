use sea_orm_migration::prelude::*;

use crate::m20221207_120629_create_geofence::Geofence;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_05] adding mode column to Geofence table");
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .add_column(ColumnDef::new(Geofence::Mode).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_05] removing mode column from Geofence table");
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .drop_column(Geofence::Mode)
                    .to_owned(),
            )
            .await
    }
}
