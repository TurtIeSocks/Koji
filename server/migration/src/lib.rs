pub use sea_orm_migration::prelude::*;

mod m20221207_120629_create_geofence;
mod m20221207_122452_create_project;
mod m20221207_122501_create_geofence_project;
mod m20221229_163230_change_fks;
mod m20230108_204408_add_type_column;
mod m20230117_010422_routes_table;
mod m20230121_184556_add_project_api;
mod m20230122_134517_route_description;
mod m20230203_214735_property_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20221207_120629_create_geofence::Migration),
            Box::new(m20221207_122452_create_project::Migration),
            Box::new(m20221207_122501_create_geofence_project::Migration),
            Box::new(m20221229_163230_change_fks::Migration),
            Box::new(m20230108_204408_add_type_column::Migration),
            Box::new(m20230117_010422_routes_table::Migration),
            Box::new(m20230121_184556_add_project_api::Migration),
            Box::new(m20230122_134517_route_description::Migration),
            Box::new(m20230203_214735_property_table::Migration),
        ]
    }
}
