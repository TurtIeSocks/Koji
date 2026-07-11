use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

use crate::helpers::has_column;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// The scanner→golbat rename (4d60e30) renamed the Rust entity field + the
    /// migration enum variant but never shipped a column rename, so SeaORM now
    /// queries a `project.golbat` column that legacy DBs don't have (they still
    /// have `scanner`) → "Unknown column 'project.golbat'". Fresh DBs created the
    /// column as `golbat` already. Rename only when the legacy column is present
    /// and the new one is absent, so this is a no-op on fresh DBs and idempotent.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if has_column(manager, "project", "scanner").await?
            && !has_column(manager, "project", "golbat").await?
        {
            log::info!("[MIGRATION] project: renaming legacy column `scanner` -> `golbat`");
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE `project` CHANGE COLUMN `scanner` `golbat` TINYINT(1) NOT NULL DEFAULT 0",
                )
                .await?;
        } else {
            log::info!(
                "[MIGRATION] project.golbat already present (or no `scanner` column); skipping rename"
            );
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if has_column(manager, "project", "golbat").await?
            && !has_column(manager, "project", "scanner").await?
        {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE `project` CHANGE COLUMN `golbat` `scanner` TINYINT(1) NOT NULL DEFAULT 0",
                )
                .await?;
        }
        Ok(())
    }
}
