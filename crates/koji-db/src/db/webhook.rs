//! CRUD entity + Query for `webhook_subscription` — the admin-facing twin of
//! `koji_events::entity::webhook_subscription` (which is delivery-facing and
//! stringly-typed). Both mirror the DDL in
//! `crates/migration/src/m20260529_000002_create_event_tables.rs` +
//! `m20260706_000001_webhook_subscription_projects.rs`. The v2_webhooks_db
//! parity test asserts the two entities read the same row identically.
//!
//! **Not** `#[macros::crud_query]`: that macro's generated `get_one`/`delete`
//! hard-code a `u32` PK (`id.parse::<u32>()`, `Entity::delete_by_id(id: u32)`),
//! but `webhook_subscription.id` is `BIGINT UNSIGNED` (`u64`). All five Query
//! methods are hand-written here to match (mirroring `route.rs`'s hand-rolled
//! `get_one`/`delete`, which predate `crud_query` for the same reason —
//! `route` doesn't use the macro either).
//!
//! **PATCH merge:** the `koji_resource!`-generated `update` handler serializes
//! only the patch DTO's *present* fields (`skip_serializing_if =
//! "Option::is_none"`) and passes that partial JSON straight to
//! `upsert_json_return` — it does NOT pre-merge against the old row. Today
//! `project`/`tile_server`/`property`'s `upsert` all rebuild the whole
//! ActiveModel from the patch JSON alone (`json.to_project()` etc.), so a
//! partial PATCH against those resources silently clobbers any omitted field
//! back to its "missing" default (and even errors if a required field like
//! `name` is omitted). That is a pre-existing gap, not something to fix here
//! (flagged in the Task 3 report instead). `Query::upsert` below avoids
//! inheriting it for webhooks: on update, it overlays the patch JSON's keys
//! onto the old row's JSON representation before calling `to_webhook`, so an
//! `{"active": false}` patch preserves `url`/`name`/etc.

use crate::query_args::AdminReqParsed;
use crate::utils::{json::JsonToModel, parse_order};

use super::sea_orm_active_enums::{WebhookMethod, WebhookMode};
use super::*;
use chrono::Utc;
use sea_orm::entity::prelude::*;
use serde_json::json;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "webhook_subscription")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: u64,
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub topics: Json,
    pub active: bool,
    pub project_id: Option<u32>,
    pub mode: WebhookMode,
    pub method: WebhookMethod,
    pub headers: Option<Json>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id"
    )]
    Project,
}

impl ActiveModelBehavior for ActiveModel {}

pub struct Query;

/// Overlay `patch`'s keys onto `old`'s JSON (flat object merge). A key present
/// in the patch replaces the old value — including explicit `null`, which is
/// how nullable columns (`secret`, `project_id`, `headers`) are cleared.
///
/// Non-nullable-with-default fields (`topics`, `active`, `mode`, `method`)
/// CANNOT receive `null` through the API: the koji_resource! Patch DTO types
/// them `Option<T>`, and serde maps JSON null to `None`, which
/// `skip_serializing_if` then omits entirely. A direct Rust caller passing
/// `null` for one of them falls through to_webhook's default (`topics` keeps
/// the literal JSON Null, since its column is the JSON type — see tests) —
/// documented behavior, not a supported path.
fn merge_patch(old: &Model, patch: &Json) -> Json {
    let mut merged = serde_json::to_value(old).expect("Model serializes to a JSON object");
    if let (Some(merged_obj), Some(patch_obj)) = (merged.as_object_mut(), patch.as_object()) {
        for (k, v) in patch_obj {
            merged_obj.insert(k.clone(), v.clone());
        }
    }
    merged
}

impl Query {
    pub async fn get_one(db: &DatabaseConnection, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u64>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(&id)).one(db).await?,
        };
        // "Does not exist" is the shared miss sentinel `__not_found_or` (in the
        // `koji_resource!` macro) sniffs to map a `ModelError` to 404 — see
        // `crud_query`'s identical arm and its doc comment in
        // `crates/macros/src/lib.rs`.
        record.ok_or_else(|| ModelError::Custom("Does not exist".to_string()))
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        Ok(json!(Query::get_one(db, id).await?))
    }

    pub async fn paginate(
        db: &DatabaseConnection,
        args: AdminReqParsed,
    ) -> Result<PaginateResults<Vec<Json>>, DbErr> {
        let mut select = Entity::find()
            .order_by(
                Column::from_str(&args.sort_by).unwrap_or(Column::Name),
                parse_order(&args.order),
            )
            .filter(Column::Name.like(format!("%{}%", args.q).as_str()));
        if let Some(project) = args.project {
            select = select.filter(Column::ProjectId.eq(project));
        }
        let paginator = select.paginate(db, args.per_page);
        let total = paginator.num_items_and_pages().await?;
        let results: Vec<Json> = paginator
            .fetch_page(args.page)
            .await?
            .into_iter()
            .map(|m| json!(m))
            .collect();
        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: args.page > 0,
            has_next: args.page + 1 < total.number_of_pages,
        })
    }

    /// Insert (`id == 0`, no existing row) or update, honoring PATCH's partial
    /// contract: on update, the incoming `json`'s keys are overlaid onto the old
    /// row's own JSON representation, so omitted keys keep their stored value
    /// instead of round-tripping through `to_webhook`'s "missing → default"
    /// rules. See the module doc for why this differs from
    /// `project`/`tile_server`/`property`'s upsert.
    pub async fn upsert(db: &DatabaseConnection, id: u32, json: Json) -> Result<Model, ModelError> {
        let old_model: Option<Model> = Entity::find_by_id(id as u64).one(db).await?;

        let model = if let Some(old_model) = old_model {
            let merged = merge_patch(&old_model, &json);
            let mut new_model = merged.to_webhook()?;
            new_model.id = Set(old_model.id);
            new_model.updated_at = Set(Utc::now().naive_utc());
            new_model.update(db).await?
        } else {
            let mut new_model = json.to_webhook()?;
            let now = Utc::now().naive_utc();
            new_model.created_at = Set(now);
            new_model.updated_at = Set(now);
            new_model.insert(db).await?
        };
        Ok(model)
    }

    pub async fn upsert_json_return(
        db: &DatabaseConnection,
        id: u32,
        json: Json,
    ) -> Result<Json, ModelError> {
        Ok(json!(Query::upsert(db, id, json).await?))
    }

    pub async fn delete(db: &DatabaseConnection, id: u32) -> Result<DeleteResult, DbErr> {
        Entity::delete_by_id(id as u64).exec(db).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pure reproduction of the merge step inside `Query::upsert`'s update
    /// branch (DB-free — `update`/`insert` need a live connection, but the
    /// overlay-then-`to_webhook` composition does not). Guards the #1 PATCH
    /// risk: a partial patch must NOT clobber fields it didn't mention.
    fn merge_and_convert(old: &Model, patch: Json) -> webhook::ActiveModel {
        let merged = merge_patch(old, &patch);
        merged.to_webhook().unwrap()
    }

    fn sample_old() -> Model {
        Model {
            id: 1,
            name: "reactmap".to_string(),
            url: "http://rm/reload".to_string(),
            secret: Some("s".to_string()),
            topics: json!(["geofence.updated"]),
            active: true,
            project_id: Some(7),
            mode: WebhookMode::Ping,
            method: WebhookMethod::Post,
            headers: Some(json!({"react-map-secret": "v"})),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn partial_patch_preserves_unset_fields() {
        let old = sample_old();
        let patched = merge_and_convert(&old, json!({ "active": false }));
        // The one field the patch mentioned changed…
        assert!(!patched.active.unwrap());
        // …everything else survived the round trip untouched.
        assert_eq!(patched.name.unwrap(), old.name);
        assert_eq!(patched.url.unwrap(), old.url);
        assert_eq!(patched.secret.unwrap(), old.secret);
        assert_eq!(patched.topics.unwrap(), old.topics);
        assert_eq!(patched.project_id.unwrap(), old.project_id);
        assert_eq!(patched.mode.unwrap(), old.mode);
        assert_eq!(patched.method.unwrap(), old.method);
        assert_eq!(patched.headers.unwrap(), old.headers);
    }

    #[test]
    fn partial_patch_can_null_out_optional_field() {
        // Explicit `"secret": null` in the patch overlays as Null, and
        // `to_webhook` reads a Null `secret` as absent → None (cleared).
        let old = sample_old();
        let patched = merge_and_convert(&old, json!({ "secret": null }));
        assert_eq!(patched.secret.unwrap(), None);
        // Unrelated fields still untouched.
        assert_eq!(patched.url.unwrap(), old.url);
    }

    #[test]
    fn null_on_defaulted_field_unreachable_via_api() {
        // Unreachable via the API (serde drops null Option<T> fields); pinned so
        // the fallback is deliberate, not accidental. Direct Rust callers passing
        // null for defaulted fields fall through to_webhook's defaults:
        // topics.cloned().unwrap_or([]) keeps Null, but active.and_then(as_bool)
        // .unwrap_or(true) treats Null as missing and resets to true.
        let old = sample_old();
        let merged = merge_patch(&old, &json!({"topics": null, "active": null}));
        let m = merged.to_webhook().unwrap();
        assert_eq!(m.topics.unwrap(), json!(null)); // Null stays.
        assert!(m.active.unwrap()); // Null → as_bool none → true default.
    }

    #[test]
    fn non_object_patch_is_noop_overlay() {
        let old = sample_old();
        let merged = merge_patch(&old, &json!("not an object"));
        let m = merged.to_webhook().unwrap();
        assert_eq!(m.url.unwrap(), old.url);
        assert_eq!(m.name.unwrap(), old.name);
    }
}
