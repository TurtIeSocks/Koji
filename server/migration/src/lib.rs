pub use sea_orm_migration::prelude::*;

mod m20221207_120629_create_geofence;
mod m20221207_122452_create_project;
mod m20221207_122501_create_geofence_project;
mod m20221229_163230_change_fks;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221207_120629_create_geofence::Migration),
            Box::new(m20221207_122452_create_project::Migration),
            Box::new(m20221207_122501_create_geofence_project::Migration),
            Box::new(m20221229_163230_change_fks::Migration),
        ]
    }
}
