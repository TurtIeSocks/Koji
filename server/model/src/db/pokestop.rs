//! SeaORM Entity. Generated by sea-orm-codegen 0.10.1
use super::*;

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pokestop")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub lat: f64,
    pub lon: f64,
    pub name: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub description: Option<String>,
    pub url: Option<String>,
    pub lure_expire_timestamp: Option<u32>,
    pub last_modified_timestamp: Option<u32>,
    pub updated: u32,
    pub enabled: Option<u8>,
    pub quest_type: Option<u32>,
    pub quest_timestamp: Option<u32>,
    pub quest_target: Option<u16>,
    #[sea_orm(column_type = "Text", nullable)]
    pub quest_conditions: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub quest_rewards: Option<String>,
    pub quest_template: Option<String>,
    pub quest_title: Option<String>,
    pub quest_reward_type: Option<i16>,
    pub quest_item_id: Option<i16>,
    pub quest_reward_amount: Option<i16>,
    pub cell_id: Option<u64>,
    pub deleted: u8,
    pub lure_id: Option<i16>,
    pub first_seen_timestamp: u32,
    pub sponsor_id: Option<u16>,
    pub partner_id: Option<String>,
    pub quest_pokemon_id: Option<i16>,
    pub ar_scan_eligible: Option<u8>,
    pub power_up_level: Option<u16>,
    pub power_up_points: Option<u32>,
    pub power_up_end_timestamp: Option<u32>,
    pub alternative_quest_type: Option<u32>,
    pub alternative_quest_timestamp: Option<u32>,
    pub alternative_quest_target: Option<u16>,
    #[sea_orm(column_type = "Text", nullable)]
    pub alternative_quest_conditions: Option<String>,
    #[sea_orm(column_type = "Text", nullable)]
    pub alternative_quest_rewards: Option<String>,
    pub alternative_quest_template: Option<String>,
    pub alternative_quest_title: Option<String>,
    pub alternative_quest_pokemon_id: Option<i16>,
    pub alternative_quest_reward_type: Option<i16>,
    pub alternative_quest_item_id: Option<i16>,
    pub alternative_quest_reward_amount: Option<i16>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

impl Query {
    pub async fn all(conn: &DatabaseConnection, last_seen: u32) -> Result<Vec<GenericData>, DbErr> {
        let items = Entity::find()
            .select_only()
            .column(Column::Lat)
            .column(Column::Lon)
            .filter(Column::Updated.gt(last_seen))
            .filter(Column::Deleted.eq(false))
            .filter(Column::Enabled.eq(true))
            .limit(2_000_000)
            .into_model::<api::point_struct::PointStruct>()
            .all(conn)
            .await?;
        Ok(utils::normalize::fort(items, "p"))
    }

    pub async fn bound(
        conn: &DatabaseConnection,
        payload: &api::args::BoundsArg,
    ) -> Result<Vec<GenericData>, DbErr> {
        let items = Entity::find()
            .select_only()
            .column(Column::Lat)
            .column(Column::Lon)
            .filter(Column::Lat.between(payload.min_lat, payload.max_lat))
            .filter(Column::Lon.between(payload.min_lon, payload.max_lon))
            .filter(
                Column::Updated.gt(if let Some(last_seen) = payload.last_seen {
                    last_seen
                } else {
                    0
                }),
            )
            .filter(Column::Deleted.eq(false))
            .filter(Column::Enabled.eq(true))
            .limit(2_000_000)
            .into_model::<api::point_struct::PointStruct>()
            .all(conn)
            .await?;
        Ok(utils::normalize::fort(items, "p"))
    }

    pub async fn area(
        conn: &DatabaseConnection,
        area: &FeatureCollection,
        last_seen: u32,
    ) -> Result<Vec<GenericData>, DbErr> {
        let items = Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::MySql,
                format!("SELECT lat, lon FROM pokestop WHERE enabled = 1 AND deleted = 0 AND updated >= {} AND ({}) LIMIT 2000000", last_seen, utils::sql_raw(area)).as_str(),
                vec![],
            ))
            .into_model::<api::point_struct::PointStruct>()
            .all(conn)
            .await?;
        Ok(utils::normalize::fort(items, "p"))
    }

    pub async fn stats(
        conn: &DatabaseConnection,
        area: &FeatureCollection,
        last_seen: u32,
    ) -> Result<Total, DbErr> {
        let items = Entity::find()
            .column_as(Column::Id.count(), "count")
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::MySql,
                format!("SELECT COUNT(*) AS total FROM pokestop WHERE enabled = 1 AND deleted = 0 AND updated >= {} AND ({})", last_seen, utils::sql_raw(area)).as_str(),
                vec![],
            ))
            .into_model::<Total>()
            .one(conn)
            .await?;
        Ok(if let Some(item) = items {
            item
        } else {
            Total { total: 0 }
        })
    }
}
