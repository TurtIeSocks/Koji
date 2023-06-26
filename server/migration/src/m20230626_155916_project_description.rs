use sea_orm_migration::prelude::*;

use crate::m20221207_122452_create_project::Project;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_20] Creating Project Description Column");
        let table = Table::alter()
            .table(Project::Table)
            .add_column(ColumnDef::new(Project::Description).string())
            .to_owned();

        manager.alter_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_20] Dropping Project Description Column");
        let table: TableAlterStatement = Table::alter()
            .table(Project::Table)
            .drop_column(Project::Description)
            .to_owned();

        manager.alter_table(table).await
    }
}
