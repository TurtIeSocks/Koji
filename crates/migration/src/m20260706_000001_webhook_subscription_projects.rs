use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{ConnectionTrait, FromQueryResult, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Parse a v1 `project.api_key` into a webhook `headers` JSON map.
///
/// v1 format was `HeaderName:SecretValue` (split on the FIRST `:` — secrets may
/// contain colons). A bare token with no `:` becomes an `Authorization` header
/// so no working config is silently dropped. Empty/whitespace → `None`.
fn parse_api_key(raw: &str) -> Option<serde_json::Value> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    match raw.split_once(':') {
        Some((name, value)) if !name.trim().is_empty() => {
            Some(serde_json::json!({ name.trim(): value.trim() }))
        }
        // ":secret" (empty header name) or bare token → Authorization.
        _ => Some(serde_json::json!({ "Authorization": raw })),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_api_key;
    use serde_json::json;

    #[test]
    fn parses_header_colon_secret() {
        assert_eq!(
            parse_api_key("x-golbat-secret:abc123"),
            Some(json!({"x-golbat-secret": "abc123"}))
        );
    }

    #[test]
    fn splits_on_first_colon_only() {
        assert_eq!(
            parse_api_key("X-Poracle-Secret:se:cr:et"),
            Some(json!({"X-Poracle-Secret": "se:cr:et"}))
        );
    }

    #[test]
    fn bare_token_becomes_authorization() {
        assert_eq!(
            parse_api_key("Bearer abc"),
            Some(json!({"Authorization": "Bearer abc"}))
        );
    }

    #[test]
    fn empty_and_whitespace_are_none() {
        assert_eq!(parse_api_key(""), None);
        assert_eq!(parse_api_key("   "), None);
    }

    #[test]
    fn leading_colon_becomes_authorization() {
        assert_eq!(
            parse_api_key(":lonely"),
            Some(json!({"Authorization": ":lonely"}))
        );
    }

    #[test]
    fn trims_around_split() {
        assert_eq!(
            parse_api_key(" X-Key : v v "),
            Some(json!({"X-Key": "v v"}))
        );
    }
}

use crate::helpers::has_column;

/// A v1 project row carrying push config, read before the drop in migration B.
#[derive(Debug, FromQueryResult)]
struct PushProject {
    id: u32,
    name: String,
    api_endpoint: Option<String>,
    api_key: Option<String>,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    /// Projects-v2 push redesign (spec 2026-07-06): extend `webhook_subscription`
    /// with project scoping + legacy ping mode, then convert every project's v1
    /// push config (`api_endpoint`/`api_key`) into a ping subscription. The
    /// project columns themselves are dropped by the LATER migration
    /// `m20260706_000002` so this one can still read them. Guarded on column
    /// absence so re-runs and fresh DBs are no-ops for both DDL and data move.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if has_column(manager, "webhook_subscription", "project_id").await? {
            log::info!("[MIGRATION] webhook_subscription already extended; skipping");
            return Ok(());
        }
        let db = manager.get_connection();
        log::info!("[MIGRATION] extending webhook_subscription with project push columns");
        db.execute_unprepared(
            r#"
ALTER TABLE `webhook_subscription`
  ADD COLUMN `name`       VARCHAR(255) NOT NULL DEFAULT '',
  ADD COLUMN `project_id` INT UNSIGNED NULL,
  ADD COLUMN `mode`       ENUM('event','ping') NOT NULL DEFAULT 'event',
  ADD COLUMN `method`     ENUM('GET','POST')   NOT NULL DEFAULT 'GET',
  ADD COLUMN `headers`    JSON NULL,
  ADD CONSTRAINT `fk_webhook_subscription_project`
    FOREIGN KEY (`project_id`) REFERENCES `project`(`id`) ON DELETE CASCADE;
            "#,
        )
        .await?;
        // Pre-existing rows (feature was unwired, so normally none): give them a
        // usable label anyway.
        db.execute_unprepared(
            "UPDATE `webhook_subscription` SET `name` = CONCAT('webhook-', `id`) WHERE `name` = ''",
        )
        .await?;

        // Data move: every project with a push endpoint becomes a ping
        // subscription with exact v1 semantics (GET, custom auth header, fire on
        // any change in the project = empty topics + project_id).
        // Skip when project.api_endpoint is already gone (fresh DB where
        // migration B's schema shipped from the start).
        if !has_column(manager, "project", "api_endpoint").await? {
            log::info!("[MIGRATION] project.api_endpoint absent; no v1 push config to migrate");
            return Ok(());
        }
        let rows = PushProject::find_by_statement(Statement::from_string(
            db.get_database_backend(),
            "SELECT `id`, `name`, `api_endpoint`, `api_key` FROM `project` \
             WHERE `api_endpoint` IS NOT NULL AND TRIM(`api_endpoint`) != ''",
        ))
        .all(db)
        .await?;
        log::info!(
            "[MIGRATION] migrating {} project push config(s) to ping webhooks",
            rows.len()
        );
        for row in rows {
            let headers = row.api_key.as_deref().and_then(parse_api_key);
            db.execute(Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO `webhook_subscription` \
                   (`name`, `url`, `secret`, `topics`, `active`, `project_id`, `mode`, `method`, `headers`, `created_at`, `updated_at`) \
                 VALUES (?, ?, NULL, '[]', 1, ?, 'ping', 'GET', ?, NOW(), NOW())",
                [
                    format!("{} reload", row.name).into(),
                    row.api_endpoint.unwrap_or_default().into(),
                    row.id.into(),
                    headers
                        .map(|h| h.to_string())
                        .map(sea_orm::Value::from)
                        .unwrap_or(sea_orm::Value::String(None)),
                ],
            ))
            .await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !has_column(manager, "webhook_subscription", "project_id").await? {
            return Ok(());
        }
        // Data-move rows die with the columns; v1 config restore is migration
        // B's `down` concern (it re-adds the project columns, empty).
        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE `webhook_subscription`
  DROP FOREIGN KEY `fk_webhook_subscription_project`,
  DROP COLUMN `project_id`,
  DROP COLUMN `name`,
  DROP COLUMN `mode`,
  DROP COLUMN `method`,
  DROP COLUMN `headers`;
                "#,
            )
            .await?;
        Ok(())
    }
}
