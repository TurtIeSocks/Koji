//! Phase 2 — breaking `Type` → `Mode` column migration.
//!
//! Migrates `geofence.mode` and `route.mode` from the legacy 12-value `Type`
//! enum (`circle_pokemon`, `circle_raid`, …) to the 4-value `Mode` enum
//! (`unset`/`pokemon`/`fort`/`quest`), backfilling each row through the
//! `koji_core::Mode::from_legacy` 12→4 collapse.
//!
//! `up` is a three-step, data-preserving widen → backfill → shrink:
//!   1. widen each column's MySQL ENUM to hold the 4 new values *alongside* the
//!      12 legacy ones (so the CASE-`UPDATE` can write the new strings without a
//!      truncation error while the old strings are still valid);
//!   2. `UPDATE … SET mode = CASE mode WHEN 'circle_pokemon' THEN 'pokemon' … END`
//!      per the canonical mapping;
//!   3. shrink each column's ENUM to exactly `('unset','pokemon','fort','quest')`
//!      `NOT NULL DEFAULT 'unset'`.
//!
//! `down` is **intentionally lossy / irreversible**: the 12→4 collapse cannot be
//! exactly inverted (e.g. both `circle_pokemon` and `pokemon_iv` collapsed to
//! `pokemon`, so we cannot know which to restore). It therefore restores each
//! column's ENUM to the full 12-value `Type` set and sets *every* row to
//! `'unset'`. This is the maintainer's accepted decision for this unpushed,
//! pre-prod branch (see the plan's "Locked decisions").
//!
//! Raw SQL goes through `manager.get_connection().execute_unprepared(...)`,
//! matching the `m20230221_130117_geofence_mode_enums` precedent.

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The widened ENUM column spec: all 16 values (12 legacy + 4 new) so the
/// backfill `UPDATE` can write the new strings before the old ones are dropped.
const WIDE_ENUM: &str = "ENUM(\
'circle_pokemon','circle_smart_pokemon','circle_raid','circle_smart_raid',\
'auto_quest','circle_quest','circle_station','pokemon_iv','leveling',\
'auto_pokemon','auto_tth','unset',\
'pokemon','fort','quest') NOT NULL DEFAULT 'unset'";

/// The final, narrow 4-value `Mode` ENUM column spec.
const MODE_ENUM: &str = "ENUM('unset','pokemon','fort','quest') NOT NULL DEFAULT 'unset'";

/// The legacy 12-value `Type` ENUM column spec (used by `down`).
const TYPE_ENUM: &str = "ENUM(\
'circle_pokemon','circle_smart_pokemon','circle_raid','circle_smart_raid',\
'auto_quest','circle_quest','circle_station','pokemon_iv','leveling',\
'auto_pokemon','auto_tth','unset') NOT NULL DEFAULT 'unset'";

/// The 12→4 backfill, expressed as a MySQL `CASE`. Mirrors
/// `koji_core::Mode::from_legacy`.
fn backfill_sql(table: &str) -> String {
    format!(
        "UPDATE `{table}` SET `mode` = CASE `mode` \
WHEN 'circle_pokemon' THEN 'pokemon' \
WHEN 'circle_smart_pokemon' THEN 'pokemon' \
WHEN 'pokemon_iv' THEN 'pokemon' \
WHEN 'auto_pokemon' THEN 'pokemon' \
WHEN 'auto_tth' THEN 'pokemon' \
WHEN 'circle_raid' THEN 'fort' \
WHEN 'circle_smart_raid' THEN 'fort' \
WHEN 'circle_station' THEN 'fort' \
WHEN 'auto_quest' THEN 'quest' \
WHEN 'circle_quest' THEN 'quest' \
WHEN 'leveling' THEN 'unset' \
WHEN 'unset' THEN 'unset' \
ELSE 'unset' END"
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_TYPE_TO_MODE] geofence.mode + route.mode: Type(12) -> Mode(4)");
        let db = manager.get_connection();

        for table in ["geofence", "route"] {
            // 1. Widen the ENUM so the new + old strings are both valid.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {WIDE_ENUM}"
            ))
            .await?;

            // 2. Backfill each row through the 12->4 collapse.
            db.execute_unprepared(&backfill_sql(table)).await?;

            // 3. Shrink the ENUM to exactly the 4 Mode values.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {MODE_ENUM}"
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // IRREVERSIBLE / LOSSY: the 12->4 collapse discarded the original legacy
        // value, so there is no faithful inverse. We restore the column's ENUM to
        // the 12-value `Type` set and default every row to 'unset'.
        log::warn!(
            "[MIGRATION_TYPE_TO_MODE] down() is lossy: restoring Type(12) ENUM, all rows -> 'unset'"
        );
        let db = manager.get_connection();

        for table in ["geofence", "route"] {
            // Widen back to the 12-value Type ENUM (the new 4 values are a subset
            // by string, except 'pokemon'/'fort'/'quest', so widen via the
            // combined set first to avoid truncation, then collapse the data).
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {WIDE_ENUM}"
            ))
            .await?;

            // Lossy: every row to 'unset' (cannot recover the original 12-value).
            db.execute_unprepared(&format!("UPDATE `{table}` SET `mode` = 'unset'"))
                .await?;

            // Settle on the legacy 12-value Type ENUM.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {TYPE_ENUM}"
            ))
            .await?;
        }

        Ok(())
    }
}
