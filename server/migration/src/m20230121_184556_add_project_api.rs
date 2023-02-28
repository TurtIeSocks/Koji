use sea_orm_migration::prelude::*;

use crate::m20221207_122452_create_project::Project;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_07] Adding Project API Columns");
        let table = Table::alter()
            .table(Project::Table)
            .add_column(ColumnDef::new(Project::ApiEndpoint).string())
            .add_column(ColumnDef::new(Project::ApiKey).string())
            .add_column(
                ColumnDef::new(Project::Scanner)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .to_owned();

        manager.alter_table(table).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_07] Removing Project API Columns");
        let table = Table::alter()
            .table(Project::Table)
            .drop_column(Project::ApiEndpoint)
            .drop_column(Project::ApiKey)
            .drop_column(Project::Scanner)
            .to_owned();

        manager.alter_table(table).await
    }
}
