use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_10] Creating Unique Property Index on (name, category)");
        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"ALTER TABLE property
                    ADD UNIQUE `name_category_unique`(`name`, `category`)"#
                    .to_owned(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_10] Dropping Unique Property Index on (name, category)");
        match manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"DROP INDEX name_category_unique ON property"#.to_owned(),
            ))
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(e),
        }
    }
}
