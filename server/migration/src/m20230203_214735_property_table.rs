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
                                Category::Enum,
                                vec![
                                    Category::Boolean,
                                    Category::String,
                                    Category::Number,
                                    Category::Object,
                                    Category::Array,
                                    Category::Database,
                                    Category::Color,
                                ],
                            )
                            .not_null(),
                    )
                    .col(ColumnDef::new(Property::DefaultValue).text())
                    .col(
                        ColumnDef::new(Property::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(Property::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra(
                                "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                            ),
                    )
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
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Category {
    #[iden = "category"]
    Enum,
    #[iden = "boolean"]
    Boolean,
    #[iden = "string"]
    String,
    #[iden = "number"]
    Number,
    #[iden = "object"]
    Object,
    #[iden = "array"]
    Array,
    #[iden = "database"]
    Database,
    #[iden = "color"]
    Color,
}
