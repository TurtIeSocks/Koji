use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION] creating plugin_config table");
        manager
            .create_table(
                Table::create()
                    .table(PluginConfig::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(PluginConfig::Id)
                            .integer()
                            .not_null()
                            .unsigned()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(PluginConfig::Name).string().not_null())
                    .col(ColumnDef::new(PluginConfig::Kind).string().not_null())
                    .col(
                        ColumnDef::new(PluginConfig::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(ColumnDef::new(PluginConfig::ArgsDefault).json().null())
                    .col(ColumnDef::new(PluginConfig::Description).string().null())
                    .col(
                        ColumnDef::new(PluginConfig::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(PluginConfig::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra(
                                "DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP".to_string(),
                            ),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx_plugin_config_kind_name")
                    .table(PluginConfig::Table)
                    .col(PluginConfig::Kind)
                    .col(PluginConfig::Name)
                    .unique()
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION] dropping plugin_config table");
        manager
            .drop_table(Table::drop().table(PluginConfig::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum PluginConfig {
    Table,
    Id,
    Name,
    Kind,
    Enabled,
    ArgsDefault,
    Description,
    CreatedAt,
    UpdatedAt,
}
