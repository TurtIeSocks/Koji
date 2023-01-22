use sea_orm_migration::prelude::*;

use crate::m20221207_120629_create_geofence::Geofence;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        println!("[MIGRATION] Creating Route Table");
        manager
            .create_table(
                Table::create()
                    .table(Route::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Route::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Route::GeofenceId).unsigned().not_null())
                    .col(ColumnDef::new(Route::Name).string().not_null())
                    .col(ColumnDef::new(Route::Mode).string().not_null())
                    .col(ColumnDef::new(Route::Geometry).json().not_null())
                    .col(ColumnDef::new(Route::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Route::UpdatedAt).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_geofence_id_route")
                            .from(Route::Table, Route::GeofenceId)
                            .to(Geofence::Table, Geofence::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .index(Index::create().name("route_name").col(Route::Name))
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Route::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Route {
    Table,
    Id,
    GeofenceId,
    Name,
    Mode,
    Description,
    Geometry,
    CreatedAt,
    UpdatedAt,
}
