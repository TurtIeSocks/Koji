//! Restore the missing `geofence_project` FKs (root-cause fix).
//!
//! `m20221229_163230_change_fks` intended to add `FK_geofence_id` /
//! `FK_project_id` on `geofence_project` (each `ON DELETE CASCADE ON UPDATE
//! CASCADE`) via sea-orm's `Table::alter().add_foreign_key(...)`, but that
//! never actually took in the live schema — live-DB diagnostics confirmed
//! `geofence_project` is InnoDB with `foreign_key_checks=1` yet has **zero**
//! foreign keys in `information_schema.key_column_usage`. Without the FK, an
//! import that links a geofence to a nonexistent project inserts the orphan
//! row successfully instead of erroring, so the enclosing transaction never
//! rolls back (see `rollback_after_begin_leaves_nothing_written` in
//! `crates/koji-db/tests/import_db.rs`, and `koji_db::db::import` — the
//! rollback logic there is correct, it just never fires with no FK to
//! violate).
//!
//! This migration uses raw SQL (`execute_unprepared`) rather than sea-orm's
//! `add_foreign_key` builder, since that builder is what silently failed to
//! take effect the first time.
//!
//! `up` first deletes orphan `geofence_project` rows (rows referencing a
//! `project_id` or `geofence_id` that no longer exists — 43 + 42 found live)
//! since `ADD CONSTRAINT` fails against existing violations, then adds both
//! FKs, guarded by [`has_foreign_key`] for idempotency (MySQL has no `ADD
//! CONSTRAINT IF NOT EXISTS`, mirroring the `has_column`/`has_index` guards
//! used by `m20260714_000001_geofence_bbox_columns`). `down` drops both FKs,
//! also guarded.

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

use crate::helpers::has_foreign_key;

#[derive(DeriveMigrationName)]
pub struct Migration;

const TABLE: &str = "geofence_project";
const FK_GEOFENCE: &str = "FK_geofence_id";
const FK_PROJECT: &str = "FK_project_id";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        let deleted_missing_project = db
            .execute_unprepared(
                "DELETE gp FROM `geofence_project` gp \
                 LEFT JOIN `project` p ON gp.project_id = p.id \
                 WHERE p.id IS NULL",
            )
            .await?
            .rows_affected();
        log::info!(
            "[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] deleted {deleted_missing_project} geofence_project row(s) with a missing project"
        );

        let deleted_missing_geofence = db
            .execute_unprepared(
                "DELETE gp FROM `geofence_project` gp \
                 LEFT JOIN `geofence` g ON gp.geofence_id = g.id \
                 WHERE g.id IS NULL",
            )
            .await?
            .rows_affected();
        log::info!(
            "[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] deleted {deleted_missing_geofence} geofence_project row(s) with a missing geofence"
        );

        if !has_foreign_key(manager, TABLE, FK_GEOFENCE).await? {
            log::info!("[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] adding {FK_GEOFENCE}");
            db.execute_unprepared(&format!(
                "ALTER TABLE `{TABLE}` ADD CONSTRAINT `{FK_GEOFENCE}` \
                 FOREIGN KEY (`geofence_id`) REFERENCES `geofence`(`id`) \
                 ON DELETE CASCADE ON UPDATE CASCADE"
            ))
            .await?;
        }

        if !has_foreign_key(manager, TABLE, FK_PROJECT).await? {
            log::info!("[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] adding {FK_PROJECT}");
            db.execute_unprepared(&format!(
                "ALTER TABLE `{TABLE}` ADD CONSTRAINT `{FK_PROJECT}` \
                 FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) \
                 ON DELETE CASCADE ON UPDATE CASCADE"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        if has_foreign_key(manager, TABLE, FK_GEOFENCE).await? {
            log::info!("[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] dropping {FK_GEOFENCE}");
            db.execute_unprepared(&format!(
                "ALTER TABLE `{TABLE}` DROP FOREIGN KEY `{FK_GEOFENCE}`"
            ))
            .await?;
        }

        if has_foreign_key(manager, TABLE, FK_PROJECT).await? {
            log::info!("[MIGRATION_RESTORE_GEOFENCE_PROJECT_FKS] dropping {FK_PROJECT}");
            db.execute_unprepared(&format!(
                "ALTER TABLE `{TABLE}` DROP FOREIGN KEY `{FK_PROJECT}`"
            ))
            .await?;
        }

        Ok(())
    }
}
