use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement},
};

use crate::m20221207_120629_create_geofence::Geofence;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_13] Adding geofence.geometry column");
        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"ALTER TABLE `geofence`
                    ADD COLUMN `geometry` JSON AS (JSON_EXTRACT(area, '$.geometry')) NOT NULL"#
                    .to_owned(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_13] Dropping Geometry Column");
        let table = Table::alter()
            .table(Geofence::Table)
            .drop_column(Geofence::Geometry)
            .to_owned();

        manager.alter_table(table).await
    }
}
