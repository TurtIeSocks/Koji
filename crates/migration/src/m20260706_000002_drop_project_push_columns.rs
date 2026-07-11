use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::ConnectionTrait;

#[derive(DeriveMigrationName)]
pub struct Migration;

use crate::helpers::has_column;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Projects-v2 (spec 2026-07-06): push config now lives in
    /// `webhook_subscription` (migrated by m20260706_000001); the v1 columns are
    /// dead. `golbat` was vestigial in v2 (read-only golbat crate, zero callers).
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        for col in ["api_endpoint", "api_key", "golbat"] {
            if has_column(manager, "project", col).await? {
                log::info!("[MIGRATION] dropping project.{col}");
                manager
                    .get_connection()
                    .execute_unprepared(&format!("ALTER TABLE `project` DROP COLUMN `{col}`"))
                    .await?;
            }
        }
        Ok(())
    }

    /// Restores the columns with their original types (see
    /// `m20230121_184556_add_project_api.rs`: `api_endpoint`/`api_key` are
    /// nullable `string()` i.e. `VARCHAR(255)`, `golbat` is
    /// `boolean().not_null().default(false)` i.e. `TINYINT(1) NOT NULL DEFAULT 0`
    /// — the latter type confirmed literally by the scanner→golbat rename
    /// migration's `CHANGE COLUMN ... TINYINT(1) NOT NULL DEFAULT 0`). Push
    /// config itself is not un-migrated — it lives in `webhook_subscription`.
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !has_column(manager, "project", "api_endpoint").await? {
            manager
                .get_connection()
                .execute_unprepared(
                    "ALTER TABLE `project` \
                       ADD COLUMN `api_endpoint` VARCHAR(255) NULL, \
                       ADD COLUMN `api_key` VARCHAR(255) NULL, \
                       ADD COLUMN `golbat` TINYINT(1) NOT NULL DEFAULT 0",
                )
                .await?;
        }
        Ok(())
    }
}
