//! Shared guards for idempotent migrations.

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::Statement;

/// True if `table` has a column named exactly `col` (current database).
/// Shared by every column-guarded migration — this was previously copy-pasted
/// per migration and had already drifted into two signatures.
pub(crate) async fn has_column(
    manager: &SchemaManager<'_>,
    table: &str,
    col: &str,
) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT COLUMN_NAME FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND COLUMN_NAME = ? LIMIT 1",
        [table.into(), col.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}
