//! A geofence is keyed by `(name, mode)` instead of `name` alone, so one place
//! can hold a fence per mode (a border and a trimmed fence sharing a name).
//!
//! `up` drops whatever single-column unique index sits on `geofence.name` (MySQL
//! named it `name` when `m20221207_120629_create_geofence` made it, but it is
//! found by shape rather than trusted by name), adds `uq_geofence_name_mode`,
//! and drops the unused `area_fence` table from
//! `m20260529_000003_dragonite_linkage`.
//!
//! `down` restores both. It cannot restore the name-only key when two modes
//! already share a name: MySQL refuses it and the migration fails, leaving the
//! rows and the `(name, mode)` key alone for the operator to resolve.

use sea_orm_migration::{prelude::*, sea_orm::Statement};

use crate::helpers::has_index;

#[derive(DeriveMigrationName)]
pub struct Migration;

const NAME_MODE_INDEX: &str = "uq_geofence_name_mode";
const NAME_INDEX: &str = "uq_geofence_name";

/// Names of the unique indexes made of `geofence.name` alone.
async fn name_only_unique_indexes(manager: &SchemaManager<'_>) -> Result<Vec<String>, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT CAST(INDEX_NAME AS CHAR) FROM information_schema.STATISTICS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = 'geofence' AND NON_UNIQUE = 0 \
         AND INDEX_NAME <> 'PRIMARY' \
         GROUP BY INDEX_NAME \
         HAVING COUNT(*) = 1 AND MAX(COLUMN_NAME) = 'name'"
            .to_string(),
    );
    conn.query_all(stmt)
        .await?
        .iter()
        .map(|row| row.try_get_by_index::<String>(0))
        .collect()
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for index in name_only_unique_indexes(manager).await? {
            log::info!("[MIGRATION_GEOFENCE_NAME_MODE] dropping unique index {index} on name");
            db.execute_unprepared(&format!("DROP INDEX `{index}` ON `geofence`"))
                .await?;
        }

        if !has_index(manager, "geofence", NAME_MODE_INDEX).await? {
            log::info!("[MIGRATION_GEOFENCE_NAME_MODE] adding unique (name, mode)");
            db.execute_unprepared(&format!(
                "ALTER TABLE `geofence` ADD UNIQUE KEY `{NAME_MODE_INDEX}` (`name`, `mode`)"
            ))
            .await?;
        }

        db.execute_unprepared("DROP TABLE IF EXISTS `area_fence`")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // Add the name-only key first: it is the step MySQL refuses when two modes
        // share a name, and failing there leaves the schema as it was.
        if name_only_unique_indexes(manager).await?.is_empty() {
            db.execute_unprepared(&format!(
                "ALTER TABLE `geofence` ADD UNIQUE KEY `{NAME_INDEX}` (`name`)"
            ))
            .await?;
        }
        if has_index(manager, "geofence", NAME_MODE_INDEX).await? {
            db.execute_unprepared(&format!("DROP INDEX `{NAME_MODE_INDEX}` ON `geofence`"))
                .await?;
        }

        // The table as `m20260529_000003_dragonite_linkage` created it.
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS `area_fence` (
  `id`                BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  `dragonite_area_id` INT UNSIGNED NOT NULL,
  `mode`              ENUM('base','pokemon','quest','fort') NOT NULL,
  `geofence_id`       INT UNSIGNED NOT NULL,
  `created_at`        DATETIME NOT NULL,
  `updated_at`        DATETIME NOT NULL,
  UNIQUE KEY `uq_area_mode` (`dragonite_area_id`, `mode`),
  INDEX `idx_geofence` (`geofence_id`),
  CONSTRAINT `fk_area_fence_geofence` FOREIGN KEY (`geofence_id`)
    REFERENCES `geofence` (`id`) ON DELETE CASCADE
)
            "#,
        )
        .await?;
        Ok(())
    }
}
