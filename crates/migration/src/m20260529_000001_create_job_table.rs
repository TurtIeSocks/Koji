use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

// Raw MySQL DDL — the `job` table from the job-queue design spec §4. Raw SQL
// (vs the schema builder) keeps the ENUM values, JSON columns, BIGINT UNSIGNED
// PK, ULID CHAR(26), and the three composite claim/lease/dedup indexes exactly
// as specified. Koji DB is MySQL 8+ / MariaDB 10.6+ (`FOR UPDATE SKIP LOCKED`).
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_JOB] creating job table");
        manager
            .get_connection()
            .execute_unprepared(
                r#"
CREATE TABLE IF NOT EXISTS `job` (
  `id`            BIGINT UNSIGNED AUTO_INCREMENT PRIMARY KEY,
  `public_id`     CHAR(26)     NOT NULL UNIQUE,
  `kind`          VARCHAR(64)  NOT NULL,
  `dedup_key`     CHAR(64)     NULL,
  `status`        ENUM('queued','running','succeeded','failed','canceled') NOT NULL DEFAULT 'queued',
  `priority`      SMALLINT     NOT NULL DEFAULT 0,
  `payload`       JSON         NOT NULL,
  `result`        JSON         NULL,
  `error`         TEXT         NULL,
  `progress`      FLOAT        NOT NULL DEFAULT 0,
  `phase`         VARCHAR(32)  NULL,
  `attempts`      INT          NOT NULL DEFAULT 0,
  `max_attempts`  INT          NOT NULL DEFAULT 1,
  `locked_by`     VARCHAR(64)  NULL,
  `lease_expires` DATETIME     NULL,
  `created_at`    DATETIME     NOT NULL,
  `started_at`    DATETIME     NULL,
  `finished_at`   DATETIME     NULL,
  INDEX `idx_claim` (`status`, `priority`, `id`),
  INDEX `idx_lease` (`status`, `lease_expires`),
  INDEX `idx_dedup` (`dedup_key`, `status`, `finished_at`)
);
                "#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_JOB] dropping job table");
        manager
            .get_connection()
            .execute_unprepared("DROP TABLE IF EXISTS `job`;")
            .await?;
        Ok(())
    }
}
