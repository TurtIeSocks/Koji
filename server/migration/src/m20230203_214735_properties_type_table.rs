use sea_orm_migration::sea_orm::{entity::*, query::*};
use sea_orm_migration::{
    prelude::{self, *},
    sea_orm::{Set, TransactionTrait},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(PropertiesType::Table)
                    .if_not_exists()
                    .col(
                        prelude::ColumnDef::new(PropertiesType::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        prelude::ColumnDef::new(PropertiesType::Name)
                            .string()
                            .not_null(),
                    )
                    .col(
                        prelude::ColumnDef::new(PropertiesType::Category)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .to_owned(),
            )
            .await?;
        let db = manager.get_connection();

        let insert = Query::insert()
            .into_table(PropertiesType::Table)
            .columns([PropertiesType::Name, PropertiesType::Category])
            .values_panic(["text".into(), "string".into()])
            .to_owned();

        manager.exec_stmt(insert).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(PropertiesType::Table).to_owned())
            .await
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum PropertiesType {
    Table,
    Id,
    Name,
    Category,
}
