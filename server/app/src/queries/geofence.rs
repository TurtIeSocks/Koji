use super::*;

use crate::entity::{geofence, project};

pub async fn all(
    conn: &DatabaseConnection,
) -> Result<Vec<(geofence::Model, Vec<project::Model>)>, DbErr> {
    let items = geofence::Entity::find()
        .find_with_related(project::Entity)
        .all(conn)
        .await?;
    Ok(items)
}
