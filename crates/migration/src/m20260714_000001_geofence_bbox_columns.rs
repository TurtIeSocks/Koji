//! Persisted lng/lat bbox columns on `geofence` (spec: bbox-task-a).
//!
//! `GET /api/v2/geofences?bbox=` currently loads every geofence and filters in
//! memory (`features_intersecting_bbox`, koji-service `/v2/geofences`). This
//! migration adds four nullable `DOUBLE` columns — `min_lat`/`min_lng`/
//! `max_lat`/`max_lng` — that carry the SAME `[minLng, minLat, maxLng,
//! maxLat]` bbox each geometry already produces via
//! `koji_db::db::geofence::geometry_bbox_lnglat` (which the write path
//! (`Query::upsert`) now keeps in sync on every create/update), so a follow-up
//! change can filter `?bbox=` at the DB instead of loading all rows. This
//! migration only adds + backfills the columns; the read handler still filters
//! in memory.
//!
//! `up` is add-columns -> add-index -> backfill (raw SQL for the DDL, mirroring
//! `m20260706_000002_drop_project_push_columns`; koji-db entities +
//! [`koji_db::db::geofence::geometry_bbox_lnglat`] for the backfill,
//! mirroring the koji-db-importing pattern other data-migrations use).
//! `down` drops the index then the columns. Both are idempotent via
//! `has_column`/`has_index` guards (siblings use the same pattern — MySQL has
//! no `IF [NOT] EXISTS` for either `ALTER TABLE ... ADD/DROP COLUMN` or
//! `CREATE/DROP INDEX`).

use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, EntityTrait, Statement},
};

use koji_db::db::geofence;

use crate::helpers::{has_column, has_index};

#[derive(DeriveMigrationName)]
pub struct Migration;

const BBOX_COLUMNS: [&str; 4] = ["min_lat", "min_lng", "max_lat", "max_lng"];
const BBOX_INDEX: &str = "idx_geofence_bbox";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        for col in BBOX_COLUMNS {
            if !has_column(manager, "geofence", col).await? {
                log::info!("[MIGRATION_GEOFENCE_BBOX] adding geofence.{col}");
                db.execute_unprepared(&format!(
                    "ALTER TABLE `geofence` ADD COLUMN `{col}` DOUBLE NULL"
                ))
                .await?;
            }
        }

        // note: a plain scalar-column B-tree index only prunes ONE dimension
        // well — MySQL can use the leading column(s) for a range scan, but a
        // B-tree can't express a true 2D bbox-overlap intersection, so the
        // trailing columns narrow rather than fully index the query. This
        // composite (min_lng, max_lng, min_lat, max_lat) at least lets the
        // `?bbox=` overlap query (`min_lng <= maxLng AND max_lng >= minLng AND
        // min_lat <= maxLat AND max_lat >= minLat`) use the first two columns
        // for a real range scan. A MySQL SPATIAL/R-tree index over a GEOMETRY
        // bbox column is the real upgrade if fence counts reach ~10k+.
        if !has_index(manager, "geofence", BBOX_INDEX).await? {
            log::info!("[MIGRATION_GEOFENCE_BBOX] adding index {BBOX_INDEX}");
            db.execute_unprepared(&format!(
                "CREATE INDEX `{BBOX_INDEX}` ON `geofence` (`min_lng`, `max_lng`, `min_lat`, `max_lat`)"
            ))
            .await?;
        }

        backfill(db).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        if has_index(manager, "geofence", BBOX_INDEX).await? {
            db.execute_unprepared(&format!("DROP INDEX `{BBOX_INDEX}` ON `geofence`"))
                .await?;
        }

        for col in BBOX_COLUMNS {
            if has_column(manager, "geofence", col).await? {
                log::info!("[MIGRATION_GEOFENCE_BBOX] dropping geofence.{col}");
                db.execute_unprepared(&format!("ALTER TABLE `geofence` DROP COLUMN `{col}`"))
                    .await?;
            }
        }

        Ok(())
    }
}

/// Compute + persist every existing geofence's bbox. Deterministic and safe
/// to re-run (recomputes the same values), so it isn't itself guarded —
/// unlike the column/index DDL above, there's no failure mode from running it
/// twice. Rows whose geometry has no bbox (empty/unparseable) are written back
/// as `NULL` explicitly, matching `Query::upsert`'s write-path behavior.
async fn backfill<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    let rows = geofence::Entity::find().all(db).await?;
    log::info!(
        "[MIGRATION_GEOFENCE_BBOX] backfilling bbox for {} geofence(s)",
        rows.len()
    );

    for row in rows {
        let bbox = geofence::geometry_bbox_lnglat(&row.geometry);
        let [min_lng, min_lat, max_lng, max_lat] = bbox
            .map(|[min_lng, min_lat, max_lng, max_lat]| {
                [Some(min_lng), Some(min_lat), Some(max_lng), Some(max_lat)]
            })
            .unwrap_or([None, None, None, None]);

        let stmt = Statement::from_sql_and_values(
            db.get_database_backend(),
            "UPDATE `geofence` SET `min_lat` = ?, `min_lng` = ?, `max_lat` = ?, `max_lng` = ? WHERE `id` = ?",
            [
                min_lat.into(),
                min_lng.into(),
                max_lat.into(),
                max_lng.into(),
                row.id.into(),
            ],
        );
        db.execute(stmt).await?;
    }

    Ok(())
}
