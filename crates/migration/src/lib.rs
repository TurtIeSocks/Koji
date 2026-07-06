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
mod m20230203_224735_property_table_unique;
mod m20230203_231010_geofence_property_table;
mod m20230204_153412_insert_property_values;
mod m20230204_162006_add_geometry_column;
mod m20230205_121524_fix_timestamps;
mod m20230221_130117_geofence_mode_enums;
mod m20230221_143509_route_mode_enums;
mod m20230301_051446_tile_server_table;
mod m20230407_045757_parent_column;
mod m20230505_150751_hop_count;
mod m20230626_155916_project_description;
mod m20260529_000001_create_job_table;
mod m20260529_000002_create_event_tables;
mod m20260529_000003_dragonite_linkage;
mod m20260529_000004_create_plugin_config;
mod m20260613_000001_type_to_mode;
mod m20260620_000001_rename_project_scanner_to_golbat;
mod m20260706_000001_webhook_subscription_projects;

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
            Box::new(m20230203_224735_property_table_unique::Migration),
            Box::new(m20230203_231010_geofence_property_table::Migration),
            Box::new(m20230204_153412_insert_property_values::Migration),
            Box::new(m20230204_162006_add_geometry_column::Migration),
            Box::new(m20230205_121524_fix_timestamps::Migration),
            Box::new(m20230221_130117_geofence_mode_enums::Migration),
            Box::new(m20230221_143509_route_mode_enums::Migration),
            Box::new(m20230301_051446_tile_server_table::Migration),
            Box::new(m20230407_045757_parent_column::Migration),
            Box::new(m20230505_150751_hop_count::Migration),
            Box::new(m20230626_155916_project_description::Migration),
            Box::new(m20260529_000001_create_job_table::Migration),
            Box::new(m20260529_000002_create_event_tables::Migration),
            Box::new(m20260529_000003_dragonite_linkage::Migration),
            Box::new(m20260529_000004_create_plugin_config::Migration),
            Box::new(m20260613_000001_type_to_mode::Migration),
            Box::new(m20260620_000001_rename_project_scanner_to_golbat::Migration),
            Box::new(m20260706_000001_webhook_subscription_projects::Migration),
        ]
    }
}
