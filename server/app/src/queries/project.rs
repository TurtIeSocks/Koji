use super::*;

use crate::entity::project;

pub async fn all(conn: &DatabaseConnection) -> Result<Vec<project::Model>, DbErr> {
    let items = project::Entity::find().all(conn).await?;
    Ok(items)
}
