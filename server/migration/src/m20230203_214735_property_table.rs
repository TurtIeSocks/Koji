use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        println!("[MIGRATION] Creating Property Table");
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
            .await?;

        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"ALTER TABLE property
                    ADD UNIQUE `unique_index`(`name`, `category`)"#.to_owned(),
            )).await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                match self.down(manager).await
                {
                    Ok(_) => Err(DbErr::Custom(format!(
                        "Failed to add unique constraint to (`property.`name`, `property`.`category`), successfully reverted the migration. Full error: {}",
                        e
                    ))),
                    Err(e) => Err(DbErr::Custom(format!(
                        "Failed to add unique constraint, and failed to drop the property table. You will need to drop it manually.\nError: {}",
                        e
                    ))),
                }
            }
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
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
