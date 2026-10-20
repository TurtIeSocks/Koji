//! DB-gated: the `(name, mode)` migration runs up and down on a database that
//! has `area_fence`. Gated on `KOJI_DB_URL` (skips cleanly without it) and runs
//! in a scratch database it creates and drops, so it never touches the shared
//! test data.
//! Run with: `set -a; source ./.env.test; set +a && cargo test -p migration --test geofence_name_mode -- --nocapture`

use migration::{Migrator, MigratorTrait};
use sea_orm_migration::sea_orm::{
    ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};

fn scratch_url(base: &str, scratch: &str) -> String {
    let (head, _) = base.rsplit_once('/').expect("KOJI_DB_URL has a database");
    format!("{head}/{scratch}")
}

async fn table_exists(db: &DatabaseConnection, table: &str) -> bool {
    let stmt = Statement::from_sql_and_values(
        DbBackend::MySql,
        "SELECT 1 FROM information_schema.TABLES WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ?",
        [table.into()],
    );
    db.query_one(stmt).await.unwrap().is_some()
}

async fn insert_fence(db: &DatabaseConnection, name: &str, mode: &str) -> Result<(), String> {
    db.execute_unprepared(&format!(
        "INSERT INTO `geofence` (`name`, `mode`, `geometry`, `geo_type`) \
         VALUES ('{name}', '{mode}', '{{}}', 'Polygon')"
    ))
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[tokio::test]
async fn name_mode_migration_runs_up_and_down_with_area_fence_present() {
    let Ok(base) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return;
    };
    let Ok(admin) = Database::connect(&base).await else {
        eprintln!("skip: could not connect to KOJI_DB_URL");
        return;
    };
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let scratch = format!("koji_mig_{nanos}");
    if let Err(e) = admin
        .execute_unprepared(&format!("CREATE DATABASE `{scratch}`"))
        .await
    {
        eprintln!("skip: cannot create a scratch database: {e}");
        return;
    }

    let result = async {
        let db = Database::connect(&scratch_url(&base, &scratch))
            .await
            .unwrap();
        let before_ours = Migrator::migrations().len() as u32 - 1;

        // Everything but this migration: `area_fence` exists, `name` alone is unique.
        Migrator::up(&db, Some(before_ours)).await.unwrap();
        assert!(table_exists(&db, "area_fence").await);
        insert_fence(&db, "Boston", "unset").await.unwrap();
        assert!(
            insert_fence(&db, "Boston", "pokemon").await.is_err(),
            "name alone is unique before the migration"
        );

        // Up: one place holds a fence per mode, `area_fence` is gone.
        Migrator::up(&db, Some(1)).await.unwrap();
        assert!(!table_exists(&db, "area_fence").await);
        insert_fence(&db, "Boston", "pokemon").await.unwrap();
        assert!(
            insert_fence(&db, "Boston", "pokemon").await.is_err(),
            "(name, mode) is unique after the migration"
        );

        // Down refuses while two modes share a name, then restores both once
        // the operator has resolved it.
        assert!(Migrator::down(&db, Some(1)).await.is_err());
        db.execute_unprepared("DELETE FROM `geofence` WHERE `mode` = 'pokemon'")
            .await
            .unwrap();
        Migrator::down(&db, Some(1)).await.unwrap();
        assert!(table_exists(&db, "area_fence").await);
        assert!(insert_fence(&db, "Boston", "fort").await.is_err());

        // And forward again.
        Migrator::up(&db, None).await.unwrap();
        assert!(!table_exists(&db, "area_fence").await);
    }
    .await;

    admin
        .execute_unprepared(&format!("DROP DATABASE `{scratch}`"))
        .await
        .unwrap();
    result
}
