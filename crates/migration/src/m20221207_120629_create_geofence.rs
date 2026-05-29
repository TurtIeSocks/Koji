use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_01] Creating Geofence Table");
        manager
            .create_table(
                Table::create()
                    .table(Geofence::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Geofence::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Geofence::Name)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Geofence::Area).json().not_null())
                    .col(ColumnDef::new(Geofence::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Geofence::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_01] Dropping Geofence Table");
        manager
            .drop_table(Table::drop().table(Geofence::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Geofence {
    Table,
    Id,
    Name,
    Parent,
    Mode,
    Area,
    Geometry,
    GeoType,
    CreatedAt,
    UpdatedAt,
}
