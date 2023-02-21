use sea_orm_migration::{
    prelude::*,
    sea_orm::{ConnectionTrait, DbBackend, Statement},
};

use crate::m20221207_120629_create_geofence::Geofence;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_15] Changing geofence mode column to an enum");

        manager
            .get_connection()
            .query_all(Statement::from_string(
                DbBackend::MySql,
                "UPDATE geofence
                SET mode = (CASE
                    WHEN mode = 'AutoQuest' THEN 'auto_quest'
                    WHEN mode = 'AutoTth' THEN 'auto_tth'
                    WHEN mode = 'PokemonIv' THEN 'pokemon_iv'
                    WHEN mode = 'AutoPokemon' THEN 'auto_pokemon'
                    ELSE 'unset'
                END)"
                    .to_string(),
            ))
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(
                        ColumnDef::new(Geofence::Mode)
                            .enumeration(
                                Fences::Enum,
                                vec![
                                    Fences::AutoQuest,
                                    Fences::AutoPokemon,
                                    Fences::AutoTth,
                                    Fences::PokemonIv,
                                    Fences::Unset,
                                ],
                            )
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        log::info!("[MIGRATION_15] Reversing geofence mode column to an enum");
        manager
            .alter_table(
                Table::alter()
                    .table(Geofence::Table)
                    .modify_column(ColumnDef::new(Geofence::Mode).string())
                    .to_owned(),
            )
            .await?;
        manager
            .get_connection()
            .query_all(Statement::from_string(
                DbBackend::MySql,
                "UPDATE geofence
                SET mode = (CASE
                    WHEN mode = 'auto_quest' THEN 'AutoQuest'
                    WHEN mode = 'auto_tth' THEN 'AutoTth'
                    WHEN mode = 'pokemon_iv' THEN 'PokemonIv'
                    WHEN mode = 'auto_pokemon' THEN 'AutoPokemon'
                    ELSE 'Unset'
                END)"
                    .to_string(),
            ))
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Fences {
    #[iden = "mode"]
    Enum,
    #[iden = "auto_quest"]
    AutoQuest,
    #[iden = "auto_pokemon"]
    AutoPokemon,
    #[iden = "auto_tth"]
    AutoTth,
    #[iden = "pokemon_iv"]
    PokemonIv,
    #[iden = "unset"]
    Unset,
}
