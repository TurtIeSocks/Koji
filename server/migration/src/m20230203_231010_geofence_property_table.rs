use super::{
    m20221207_120629_create_geofence::Geofence, m20230203_214735_property_table::Property,
};

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_11] Creating Geofence_Property Table");
        manager
            .create_table(
                Table::create()
                    .table(GeofenceProperty::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GeofenceProperty::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(GeofenceProperty::GeofenceId)
                            .unsigned()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GeofenceProperty::PropertyId)
                            .unsigned()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GeofenceProperty::Value).text())
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_geofence_property_id")
                            .from(GeofenceProperty::Table, GeofenceProperty::GeofenceId)
                            .to(Geofence::Table, Geofence::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("FK_property_id")
                            .from(GeofenceProperty::Table, GeofenceProperty::PropertyId)
                            .to(Property::Table, Property::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_11] Dropping Geofence_Property Table");
        manager
            .drop_table(Table::drop().table(GeofenceProperty::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum GeofenceProperty {
    Table,
    Id,
    GeofenceId,
    PropertyId,
    Value,
}
