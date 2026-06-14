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
//! `down` is the same widen → backfill → shrink in reverse, but **lossy**: the
//! 12→4 collapse cannot be exactly inverted (e.g. both `circle_pokemon` and
//! `pokemon_iv` collapsed to `pokemon`, so we cannot know which to restore). It
//! therefore maps each `Mode` back to a single *representative* `Type`
//! (`pokemon`→`circle_pokemon`, `fort`→`circle_raid`, `quest`→`auto_quest`,
//! everything else→`unset`), restoring a valid 12-value column without faithfully
//! recovering the original value — best-effort, not exact. Accepted for this
//! unpushed, pre-prod branch (see the plan's "Locked decisions").
//!
//! Raw SQL goes through `manager.get_connection().execute_unprepared(...)`,
//! matching the `m20230221_130117_geofence_mode_enums` precedent.

use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

const V1_ENUM_RAW: [&str; 11] = [
    "circle_pokemon",
    "circle_smart_pokemon",
    "circle_raid",
    "circle_smart_raid",
    "auto_quest",
    "circle_quest",
    "circle_station",
    "pokemon_iv",
    "leveling",
    "auto_pokemon",
    "auto_tth",
];

const V2_ENUM_RAW: [&str; 3] = ["pokemon", "fort", "quest"];

const TABLES: [&str; 2] = ["geofence", "route"];

/// Build a MySQL `ENUM(...) NOT NULL DEFAULT 'unset'` column spec from one or
/// more value lists. Every value — plus the always-present `unset` — is rendered
/// as a quoted SQL string literal, and the lists are flattened before joining so
/// multi-list specs (e.g. `wide_enum`) carry a separator across the boundary
/// rather than fusing one list's last value onto the next list's first.
fn create_enum(enums: &[&[&str]]) -> String {
    let values = enums
        .iter()
        .flat_map(|list| list.iter())
        .chain(std::iter::once(&"unset"))
        .map(|value| format!("'{value}'"))
        .collect::<Vec<_>>()
        .join(",");
    format!("ENUM({values}) NOT NULL DEFAULT 'unset'")
}

fn v1_enum() -> String {
    create_enum(&[&V1_ENUM_RAW])
}

fn v2_enum() -> String {
    create_enum(&[&V2_ENUM_RAW])
}

fn wide_enum() -> String {
    create_enum(&[&V1_ENUM_RAW, &V2_ENUM_RAW])
}

/// The 12→4 backfill, expressed as a MySQL `CASE`. Mirrors
/// `koji_core::Mode::from_legacy`.
fn v1_to_v2(table: &str) -> String {
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
ELSE 'unset' END"
    )
}

fn v2_to_v1(table: &str) -> String {
    format!(
        "UPDATE `{table}` SET `mode` = CASE `mode` \
WHEN 'pokemon' THEN 'circle_pokemon' \
WHEN 'fort' THEN 'circle_raid' \
WHEN 'quest' THEN 'auto_quest' \
ELSE 'unset' END"
    )
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_TYPE_TO_MODE] geofence.mode + route.mode: Type(12) -> Mode(4)");
        let db = manager.get_connection();

        for table in TABLES {
            // 1. Widen the ENUM so the new + old strings are both valid.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {}",
                wide_enum()
            ))
            .await?;

            // 2. Backfill each row through the 12->4 collapse.
            db.execute_unprepared(&v1_to_v2(table)).await?;

            // 3. Shrink the ENUM to exactly the 4 Mode values.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {}",
                v2_enum()
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // LOSSY (best-effort): the 12->4 collapse discarded the original legacy
        // value, so there is no faithful inverse. We restore the 12-value `Type`
        // ENUM and map each `Mode` back to a representative `Type` (see `v2_to_v1`).
        log::info!(
            "[MIGRATION_TYPE_TO_MODE] geofence.mode + route.mode: Mode(4) -> Type(12) - this process is lossy, check the results!"
        );
        let db = manager.get_connection();

        for table in TABLES {
            // Widen to the combined Type+Mode ENUM first so the existing `Mode`
            // strings stay valid while the UPDATE rewrites them to `Type` values
            // (avoids an ENUM-truncation error); the shrink to `Type` is below.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {}",
                wide_enum()
            ))
            .await?;

            // Map each `Mode` back to a representative `Type` (4->12, best-effort).
            db.execute_unprepared(&v2_to_v1(table)).await?;

            // Settle on the legacy 12-value Type ENUM.
            db.execute_unprepared(&format!(
                "ALTER TABLE `{table}` MODIFY COLUMN `mode` {}",
                v1_enum()
            ))
            .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_specs_quote_every_value_and_separate_lists() {
        // The final 4-value Mode column: every value quoted, `unset` folded in.
        assert_eq!(
            v2_enum(),
            "ENUM('pokemon','fort','quest','unset') NOT NULL DEFAULT 'unset'"
        );
        // Regression guard for the multi-list join: the last legacy value and the
        // first new value must NOT fuse — a comma has to sit between them.
        assert!(
            wide_enum().contains("'auto_tth','pokemon'"),
            "wide_enum lost the list-boundary comma: {}",
            wide_enum()
        );
        // Legacy values are quoted string literals, not bare identifiers.
        assert!(v1_enum().starts_with("ENUM('circle_pokemon','circle_smart_pokemon',"));
        assert!(v1_enum().ends_with("'auto_tth','unset') NOT NULL DEFAULT 'unset'"));
    }
}
