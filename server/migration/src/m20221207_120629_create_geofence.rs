use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Geofence::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Geofence::Id)
                            .integer()
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
    Area,
    CreatedAt,
    UpdatedAt,
}
