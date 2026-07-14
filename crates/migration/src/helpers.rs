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

/// True if `table` has an index named exactly `name` (current database).
/// Sibling of [`has_column`] — same idempotency-guard purpose, for
/// `CREATE INDEX`/`DROP INDEX` (MySQL has no `IF [NOT] EXISTS` clause for
/// either, unlike columns via `ALTER TABLE`).
pub(crate) async fn has_index(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT INDEX_NAME FROM information_schema.STATISTICS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND INDEX_NAME = ? LIMIT 1",
        [table.into(), name.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}

/// True if `table` has a FOREIGN KEY constraint named exactly `name`
/// (current database). Sibling of [`has_column`]/[`has_index`] — same
/// idempotency-guard purpose, for `ADD CONSTRAINT`/`DROP FOREIGN KEY` (MySQL
/// has no `IF [NOT] EXISTS` clause for either).
pub(crate) async fn has_foreign_key(
    manager: &SchemaManager<'_>,
    table: &str,
    name: &str,
) -> Result<bool, DbErr> {
    let conn = manager.get_connection();
    let stmt = Statement::from_sql_and_values(
        conn.get_database_backend(),
        "SELECT CONSTRAINT_NAME FROM information_schema.TABLE_CONSTRAINTS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? AND CONSTRAINT_NAME = ? \
         AND CONSTRAINT_TYPE = 'FOREIGN KEY' LIMIT 1",
        [table.into(), name.into()],
    );
    Ok(conn.query_one(stmt).await?.is_some())
}
