use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DbBackend, Statement},
};

use crate::m20230117_010422_routes_table::Route;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_15] Changing route mode column to an enum");

        manager
            .get_connection()
            .query_all(Statement::from_string(
                DbBackend::MySql,
                "UPDATE route
                SET mode = (CASE
                    WHEN mode = 'CirclePokemon' THEN 'circle_pokemon'
                    WHEN mode = 'CircleSmartPokemon' THEN 'circle_smart_pokemon'
                    WHEN mode = 'CircleRaid' THEN 'circle_raid'
                    WHEN mode = 'CircleSmartRaid' THEN 'circle_smart_raid'
                    WHEN mode = 'ManualQuest' THEN 'circle_quest'
                    ELSE 'unset'
                END)"
                    .to_string(),
            ))
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Route::Table)
                    .modify_column(
                        ColumnDef::new(Route::Mode)
                            .enumeration(
                                Routes::Enum,
                                vec![
                                    Routes::CirclePokemon,
                                    Routes::CircleSmartPokemon,
                                    Routes::CircleRaid,
                                    Routes::CircleSmartRaid,
                                    Routes::CircleQuest,
                                    Routes::Unset,
                                ],
                            )
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_16] Reversing route mode column to an enum");
        manager
            .alter_table(
                Table::alter()
                    .table(Route::Table)
                    .modify_column(ColumnDef::new(Route::Mode).string().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .query_all(Statement::from_string(
                DbBackend::MySql,
                "UPDATE route
                SET mode = (CASE
                    WHEN mode = 'circle_pokemon' THEN 'CirclePokemon'
                    WHEN mode = 'circle_smart_pokemon' THEN 'CircleSmartPokemon'
                    WHEN mode = 'circle_raid' THEN 'CircleRaid'
                    WHEN mode = 'circle_smart_raid' THEN 'CircleSmartRaid'
                    WHEN mode = 'circle_quest' THEN 'ManualQuest'
                    ELSE 'Unset'
                END)"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Routes {
    #[iden = "mode"]
    Enum,
    #[iden = "circle_pokemon"]
    CirclePokemon,
    #[iden = "circle_smart_pokemon"]
    CircleSmartPokemon,
    #[iden = "circle_raid"]
    CircleRaid,
    #[iden = "circle_smart_raid"]
    CircleSmartRaid,
    #[iden = "circle_quest"]
    CircleQuest,
    #[iden = "unset"]
    Unset,
}
