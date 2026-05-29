use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Raw MySQL DDL for the koji-events outbox + webhook registry
// (docs/superpowers/specs/2026-05-29-koji-v2-p3b-events-design.md). Additive.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_EVENTS] creating event_outbox + webhook_subscription tables");
        let db = manager.get_connection();
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS `event_outbox` (
  `id`              BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  `public_id`       CHAR(26)    NOT NULL UNIQUE,
  `topic`           VARCHAR(64) NOT NULL,
  `payload`         JSON        NOT NULL,
  `status`          ENUM('pending','delivering','delivered','dead') NOT NULL DEFAULT 'pending',
  `attempts`        INT         NOT NULL DEFAULT 0,
  `max_attempts`    INT         NOT NULL DEFAULT 8,
  `next_attempt_at` DATETIME    NOT NULL,
  `locked_by`       VARCHAR(64) NULL,
  `lease_expires`   DATETIME    NULL,
  `last_error`      TEXT        NULL,
  `created_at`      DATETIME    NOT NULL,
  `delivered_at`    DATETIME    NULL,
  INDEX `idx_due` (`status`, `next_attempt_at`),
  INDEX `idx_topic` (`topic`)
);
            "#,
        )
        .await?;
        db.execute_unprepared(
            r#"
CREATE TABLE IF NOT EXISTS `webhook_subscription` (
  `id`         BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  `url`        VARCHAR(512) NOT NULL,
  `secret`     VARCHAR(255) NULL,
  `topics`     JSON         NOT NULL,
  `active`     TINYINT(1)   NOT NULL DEFAULT 1,
  `created_at` DATETIME     NOT NULL,
  `updated_at` DATETIME     NOT NULL
);
            "#,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_EVENTS] dropping event tables");
        let db = manager.get_connection();
        db.execute_unprepared("DROP TABLE IF EXISTS `webhook_subscription`;")
            .await?;
        db.execute_unprepared("DROP TABLE IF EXISTS `event_outbox`;")
            .await?;
        Ok(())
    }
}
