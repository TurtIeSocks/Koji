use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() {
    log::info!("[MIGRATION] Starting migrations");
    cli::run_cli(migration::Migrator).await;
}
