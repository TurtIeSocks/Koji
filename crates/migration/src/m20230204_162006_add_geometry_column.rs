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

        let db = manager.get_connection();
        let backend = manager.get_database_backend();

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .add_column(ColumnDef::new(Geofence::Geometry).json())
                    .to_owned(),
            )
            .await?;

        db.execute(Statement::from_string(
            backend,
            r#"UPDATE `geofence` 
                SET `geometry` = JSON_EXTRACT(area, '$.geometry')"#
                .to_owned(),
        ))
        .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(ColumnDef::new(Geofence::Geometry).json().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(ColumnDef::new(Geofence::Area).json().null())
                    .to_owned(),
            )
            .await?;

        db.execute(Statement::from_string(
            backend,
            r#"ALTER TABLE `geofence` 
                ADD COLUMN `geo_type` VARCHAR(20) 
                    GENERATED ALWAYS AS (
                        JSON_UNQUOTE(JSON_EXTRACT(geometry, '$.type'))
                    ) STORED
            "#
            .to_owned(),
        ))
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_13] Dropping Geometry Column");

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .drop_column(Geofence::GeoType)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .drop_column(Geofence::Geometry)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(ColumnDef::new(Geofence::Area).json().not_null())
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}
