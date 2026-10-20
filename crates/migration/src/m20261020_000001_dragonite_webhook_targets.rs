//! A Dragonite is a webhook subscription in `dragonite` mode, and a geofence's
//! Dragonite Area is stored per target instead of on the geofence.
//!
//! `up` adds `dragonite` to `webhook_subscription.mode`, creates
//! `dragonite_link` (one Area id per Dragonite row and geofence), and drops
//! `geofence.dragonite_area_id` and the unused `area_fence` table. The old
//! single link names no target, so it is not carried over.
//!
//! `down` restores the column (empty) and removes the `dragonite` rows, which
//! the narrower `mode` enum cannot hold, along with their links.

use sea_orm_migration::prelude::*;

use crate::helpers::{has_column, has_index};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        log::info!("[MIGRATION_DRAGONITE_TARGETS] webhook_subscription.mode gains dragonite");
        db.execute_unprepared(
            "ALTER TABLE `webhook_subscription` \
             MODIFY COLUMN `mode` ENUM('event','ping','dragonite') NOT NULL DEFAULT 'event'",
        )
        .await?;

        log::info!("[MIGRATION_DRAGONITE_TARGETS] creating dragonite_link");
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS `dragonite_link` (
  `webhook_id`        BIGINT UNSIGNED NOT NULL,
  `geofence_id`       INT UNSIGNED NOT NULL,
  `dragonite_area_id` INT UNSIGNED NOT NULL,
  `created_at`        DATETIME NOT NULL,
  `updated_at`        DATETIME NOT NULL,
  PRIMARY KEY (`webhook_id`, `geofence_id`),
  INDEX `idx_dragonite_link_geofence` (`geofence_id`),
  CONSTRAINT `fk_dragonite_link_webhook` FOREIGN KEY (`webhook_id`)
    REFERENCES `webhook_subscription` (`id`) ON DELETE CASCADE,
  CONSTRAINT `fk_dragonite_link_geofence` FOREIGN KEY (`geofence_id`)
    REFERENCES `geofence` (`id`) ON DELETE CASCADE
)
            "#,
        )
        .await?;

        if has_index(manager, "geofence", "idx_dragonite_area").await? {
            db.execute_unprepared("DROP INDEX `idx_dragonite_area` ON `geofence`")
                .await?;
        }
        if has_column(manager, "geofence", "dragonite_area_id").await? {
            log::info!("[MIGRATION_DRAGONITE_TARGETS] dropping geofence.dragonite_area_id");
            db.execute_unprepared("ALTER TABLE `geofence` DROP COLUMN `dragonite_area_id`")
                .await?;
        }
        db.execute_unprepared("DROP TABLE IF EXISTS `area_fence`")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        if !has_column(manager, "geofence", "dragonite_area_id").await? {
            db.execute_unprepared(
                r#"
ALTER TABLE `geofence`
  ADD COLUMN `dragonite_area_id` INT UNSIGNED NULL,
  ADD INDEX `idx_dragonite_area` (`dragonite_area_id`)
                "#,
            )
            .await?;
        }
        db.execute_unprepared("DROP TABLE IF EXISTS `dragonite_link`")
            .await?;
        db.execute_unprepared("DELETE FROM `webhook_subscription` WHERE `mode` = 'dragonite'")
            .await?;
        db.execute_unprepared(
            "ALTER TABLE `webhook_subscription` \
             MODIFY COLUMN `mode` ENUM('event','ping') NOT NULL DEFAULT 'event'",
        )
        .await?;
        Ok(())
    }
}
