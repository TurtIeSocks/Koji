use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// PR #558 readiness (architecture §7/§9/§10): the per-mode `area_fence` linkage
// table + a `dragonite_area_id` column on Koji's own `geofence` table.
//
// ASSUMPTION (maintainer confirm): the linkage lives on `geofence` (a Koji-owned,
// migratable table) — NOT the `area` entity, which reads the external controller
// DB and isn't ours to migrate. `dragonite_area_id` typed INT UNSIGNED NULL
// (Dragonite area ids are integers); adjust if Dragonite uses a string id.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_DRAGONITE] geofence.dragonite_area_id + area_fence");
        let db = manager.get_connection();
        // Plain ADD (no IF NOT EXISTS) — MySQL 8 doesn't support IF NOT EXISTS on
        // ALTER; sea-orm runs each migration once so the guard isn't needed.
        db.execute_unprepared(
            r#"
ALTER TABLE `geofence`
  ADD COLUMN `dragonite_area_id` INT UNSIGNED NULL,
  ADD INDEX `idx_dragonite_area` (`dragonite_area_id`);
            "#,
        )
        .await?;
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
);
            "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_DRAGONITE] reverting dragonite linkage");
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS `area_fence`;")
            .await?;
        db.execute_unprepared("ALTER TABLE `geofence` DROP COLUMN `dragonite_area_id`;")
            .await?;
        Ok(())
    }
}
