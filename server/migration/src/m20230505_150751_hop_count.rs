use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, Statement},
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"ALTER TABLE `route` 
                ADD COLUMN `points` MEDIUMINT unsigned 
                    GENERATED ALWAYS AS (
                        JSON_LENGTH(JSON_EXTRACT(geometry, '$.coordinates'))
                    ) STORED
                "#
                .to_owned(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"ALTER TABLE `route` 
                DROP COLUMN `points`
                "#
                .to_owned(),
            ))
            .await?;
        Ok(())
    }
}
