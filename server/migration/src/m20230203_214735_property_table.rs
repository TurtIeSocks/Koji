use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_09] Creating Property Table");
        manager
            .create_table(
                Table::create()
                    .table(Property::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Property::Id)
                            .unsigned()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Property::Name).string().not_null())
                    .col(
                        ColumnDef::new(Property::Category)
                            .enumeration(
                                "category",
                                vec![
                                    "boolean", "string", "number", "object", "array", "database",
                                    "color",
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Property::DefaultValue).text())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_09] Dropping Property Table");
        manager
            .drop_table(Table::drop().table(Property::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Property {
    Table,
    Id,
    Name,
    Category,
    DefaultValue,
}
