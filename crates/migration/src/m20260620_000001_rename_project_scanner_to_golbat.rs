use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// True if the `project` table has a column named exactly `col`.
async fn has_column(manager: &SchemaManager<'_>, col: &str) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'project' AND COLUMN_NAME = ? LIMIT 1",
        [col.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// The scanner→golbat rename (4d60e30) renamed the Rust entity field + the
    /// migration enum variant but never shipped a column rename, so SeaORM now
    /// queries a `project.golbat` column that legacy DBs don't have (they still
    /// have `scanner`) → "Unknown column 'project.golbat'". Fresh DBs created the
    /// column as `golbat` already. Rename only when the legacy column is present
    /// and the new one is absent, so this is a no-op on fresh DBs and idempotent.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if has_column(manager, "scanner").await? && !has_column(manager, "golbat").await? {
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
        if has_column(manager, "golbat").await? && !has_column(manager, "scanner").await? {
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
