use sea_orm_migration::prelude::*;

use crate::m20230117_010422_routes_table::Route;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_08] Creating Route Description Column");
        let table = Table::alter()
            .table(Route::Table)
            .add_column(ColumnDef::new(Route::Description).string())
            .to_owned();

        manager.alter_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_08] Dropping Route Description Column");
        let table = Table::alter()
            .table(Route::Table)
            .drop_column(Route::Description)
            .to_owned();

        manager.alter_table(table).await
    }
}
