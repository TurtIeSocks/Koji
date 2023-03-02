use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TileServer::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TileServer::Id)
                            .integer()
                            .not_null()
                            .unsigned()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(TileServer::Name).string().not_null())
                    .col(ColumnDef::new(TileServer::Url).string().not_null())
                    .col(
                        ColumnDef::new(TileServer::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(TileServer::UpdatedAt)
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
        manager
            .drop_table(Table::drop().table(TileServer::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum TileServer {
    Table,
    Id,
    Name,
    Url,
    CreatedAt,
    UpdatedAt,
}
