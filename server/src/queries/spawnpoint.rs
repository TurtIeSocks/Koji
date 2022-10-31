use super::*;
use crate::entities::spawnpoint;
use crate::models::{api::MapBounds, scanner::GenericData};
use crate::utils::sql_raw::sql_raw;

#[derive(Debug, Clone, FromQueryResult)]
pub struct TrimmedSpawn {
    pub lat: f64,
    pub lon: f64,
    pub despawn_sec: Option<u16>,
}

fn return_generic(items: Vec<TrimmedSpawn>) -> Vec<GenericData> {
    items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            GenericData::new(
                format!(
                    "{}{}",
                    if item.despawn_sec.is_some() { "v" } else { "u" },
                    i
                ),
                item.lat,
                item.lon,
            )
        })
        .collect()
}

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .into_model::<TrimmedSpawn>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn bound(
    conn: &DatabaseConnection,
    payload: &MapBounds,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .select_only()
        .column(spawnpoint::Column::Lat)
        .column(spawnpoint::Column::Lon)
        .column(spawnpoint::Column::DespawnSec)
        .filter(spawnpoint::Column::Lat.between(payload.min_lat, payload.max_lat))
        .filter(spawnpoint::Column::Lon.between(payload.min_lon, payload.max_lon))
        .into_model::<TrimmedSpawn>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}

pub async fn area(
    conn: &DatabaseConnection,
    area: &Vec<[f64; 2]>,
) -> Result<Vec<GenericData>, DbErr> {
    let items = spawnpoint::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DbBackend::MySql,
            r#"SELECT lat, lon, despawn_sec FROM spawnpoint $1"#,
            vec![(sql_raw(area)).into()],
        ))
        .into_model::<TrimmedSpawn>()
        .all(conn)
        .await?;
    Ok(return_generic(items))
}
