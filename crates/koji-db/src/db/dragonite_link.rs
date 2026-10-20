//! A geofence's Dragonite Area, stored per target.
//!
//! A target is a `webhook_subscription` row in `dragonite` mode. Its link table
//! (`dragonite_link`, migration `m20261020_000001_dragonite_webhook_targets`) maps
//! `(target, geofence)` to the Area id on that Dragonite, so two Dragonites can
//! reuse the same Area ids.
//!
//! A target **reaches** a geofence when it is active and either has no project
//! (every geofence) or the geofence is in its project. Links outlive reach: a
//! link on a geofence a target no longer reaches is kept and ignored.
//!
//! A target's Area takes at most one geofence per mode. The table cannot say so
//! (the mode lives on `geofence`), so every write here checks it under a lock on
//! the target's row.

use std::collections::HashMap;

use chrono::Utc;
use sea_orm::{
    ConnectionTrait, DbBackend, FromQueryResult, QueryOrder, QuerySelect, Statement,
    TransactionTrait, Value, entity::prelude::*,
};
use serde::{Deserialize, Serialize};

use super::geofence::GeofenceLinkRow;
use super::sea_orm_active_enums::{Mode, WebhookMode};
use super::{geofence, geofence_project, webhook};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "dragonite_link")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub webhook_id: u64,
    #[sea_orm(primary_key, auto_increment = false)]
    pub geofence_id: u32,
    pub dragonite_area_id: u32,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    /// The link as a geofence's record carries it: its target and Area.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "webhook_id": self.webhook_id,
            "dragonite_area_id": self.dragonite_area_id,
        })
    }
}

/// One requested change to a geofence's links: set the Area on a target, or
/// clear it with `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct LinkChange {
    pub webhook_id: u64,
    pub dragonite_area_id: Option<u32>,
}

/// Why a link write was refused.
#[derive(Debug, thiserror::Error)]
pub enum LinkError {
    #[error("no webhook {0}")]
    NoTarget(u64),
    #[error("webhook {0} is not a Dragonite target")]
    NotDragonite(u64),
    #[error("no geofence {0}")]
    NoGeofence(u32),
    /// The target's Area already belongs to another geofence of the same mode.
    #[error(
        "Dragonite area {dragonite_area_id} on webhook {webhook_id} already belongs to geofence {held_by} in mode {mode}"
    )]
    AreaTaken {
        webhook_id: u64,
        dragonite_area_id: u32,
        mode: String,
        held_by: u32,
    },
    #[error(transparent)]
    Db(#[from] DbErr),
}

/// A geofence holding an Area on a target.
#[derive(Debug, Clone, PartialEq, Eq, FromQueryResult)]
pub struct AreaHolder {
    pub dragonite_area_id: u32,
    pub geofence_id: u32,
    pub name: String,
    pub mode: Mode,
}

/// The Dragonite targets that reach a geofence, and which of them hold a link.
#[derive(Debug, Clone)]
pub struct PublishTargets {
    /// Every active Dragonite target that reaches the geofence.
    pub reaching: Vec<webhook::Model>,
    /// The reaching targets that hold a link for it, with that link.
    pub linked: Vec<(webhook::Model, Model)>,
}

/// Whether `target` is a Dragonite row that can reach anything at all.
fn can_reach(target: &webhook::Model) -> bool {
    target.active && target.mode == WebhookMode::Dragonite
}

/// Ids per `IN (...)` list, well under the placeholder limit.
const ID_CHUNK: usize = 5_000;

pub struct Query;

impl Query {
    /// Every link a geofence holds, one per target, by target id.
    pub async fn for_geofence<C: ConnectionTrait>(
        db: &C,
        geofence_id: u32,
    ) -> Result<Vec<Model>, DbErr> {
        Entity::find()
            .filter(Column::GeofenceId.eq(geofence_id))
            .order_by_asc(Column::WebhookId)
            .all(db)
            .await
    }

    /// A geofence's links as its record carries them, one per target.
    pub async fn json_for_geofence<C: ConnectionTrait>(
        db: &C,
        geofence_id: u32,
    ) -> Result<serde_json::Value, DbErr> {
        let links = Self::for_geofence(db, geofence_id).await?;
        Ok(links.iter().map(Model::to_json).collect())
    }

    /// Whether `target` reaches the geofence.
    pub async fn reaches<C: ConnectionTrait>(
        db: &C,
        target: &webhook::Model,
        geofence_id: u32,
    ) -> Result<bool, DbErr> {
        if !can_reach(target) {
            return Ok(false);
        }
        let Some(project_id) = target.project_id else {
            return Ok(true);
        };
        Ok(geofence_project::Entity::find()
            .filter(geofence_project::Column::GeofenceId.eq(geofence_id))
            .filter(geofence_project::Column::ProjectId.eq(project_id))
            .one(db)
            .await?
            .is_some())
    }

    /// The Dragonite targets that reach `geofence_id` and the links they hold
    /// for it, for publishing. Both lists are by target id.
    pub async fn publish_targets<C: ConnectionTrait>(
        db: &C,
        geofence_id: u32,
    ) -> Result<PublishTargets, DbErr> {
        let mut found = Self::publish_targets_for(db, &[geofence_id]).await?;
        Ok(found
            .remove(&geofence_id)
            .unwrap_or_else(|| PublishTargets {
                reaching: Vec::new(),
                linked: Vec::new(),
            }))
    }

    /// [`Query::publish_targets`] for many geofences in a fixed number of
    /// queries, so a bulk publish does not read per geofence. Every id in
    /// `geofence_ids` is a key of the answer, existing geofence or not.
    pub async fn publish_targets_for<C: ConnectionTrait>(
        db: &C,
        geofence_ids: &[u32],
    ) -> Result<HashMap<u32, PublishTargets>, DbErr> {
        let mut projects: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut links: HashMap<(u64, u32), Model> = HashMap::new();
        for chunk in geofence_ids.chunks(ID_CHUNK) {
            for row in geofence_project::Entity::find()
                .filter(geofence_project::Column::GeofenceId.is_in(chunk.iter().copied()))
                .all(db)
                .await?
            {
                projects
                    .entry(row.geofence_id)
                    .or_default()
                    .push(row.project_id);
            }
            for link in Entity::find()
                .filter(Column::GeofenceId.is_in(chunk.iter().copied()))
                .all(db)
                .await?
            {
                links.insert((link.webhook_id, link.geofence_id), link);
            }
        }
        let active: Vec<webhook::Model> = webhook::Entity::find()
            .filter(webhook::Column::Mode.eq(WebhookMode::Dragonite))
            .filter(webhook::Column::Active.eq(true))
            .order_by_asc(webhook::Column::Id)
            .all(db)
            .await?;
        Ok(geofence_ids
            .iter()
            .map(|&geofence_id| {
                let in_projects = projects.get(&geofence_id);
                let reaching: Vec<webhook::Model> = active
                    .iter()
                    .filter(|target| {
                        target
                            .project_id
                            .is_none_or(|p| in_projects.is_some_and(|ps| ps.contains(&p)))
                    })
                    .cloned()
                    .collect();
                let linked = reaching
                    .iter()
                    .filter_map(|target| {
                        links
                            .get(&(target.id, geofence_id))
                            .map(|link| (target.clone(), link.clone()))
                    })
                    .collect();
                (geofence_id, PublishTargets { reaching, linked })
            })
            .collect())
    }

    /// The target's geofences it reaches, each with the Area it is linked to on
    /// this target, by geofence id. Empty for a row that reaches nothing.
    pub async fn link_rows<C: ConnectionTrait>(
        db: &C,
        target: &webhook::Model,
    ) -> Result<Vec<GeofenceLinkRow>, DbErr> {
        if !can_reach(target) {
            return Ok(Vec::new());
        }
        let sql = "SELECT g.id, g.name, g.mode, l.dragonite_area_id \
                   FROM geofence g \
                   LEFT JOIN dragonite_link l ON l.geofence_id = g.id AND l.webhook_id = ? \
                   WHERE (? IS NULL OR g.id IN \
                          (SELECT gp.geofence_id FROM geofence_project gp WHERE gp.project_id = ?)) \
                   ORDER BY g.id";
        let project: Value = target.project_id.into();
        GeofenceLinkRow::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            sql,
            [target.id.into(), project.clone(), project],
        ))
        .all(db)
        .await
    }

    /// The Areas the target already holds, by `(area, mode)`, with the geofence
    /// holding each. Counts every geofence, in reach or not.
    pub async fn taken_areas<C: ConnectionTrait>(
        db: &C,
        webhook_id: u64,
    ) -> Result<HashMap<(u32, Mode), u32>, DbErr> {
        #[derive(FromQueryResult)]
        struct Held {
            geofence_id: u32,
            dragonite_area_id: u32,
            mode: Mode,
        }
        let held = Held::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT l.geofence_id, l.dragonite_area_id, g.mode \
             FROM dragonite_link l JOIN geofence g ON g.id = l.geofence_id \
             WHERE l.webhook_id = ?",
            [webhook_id.into()],
        ))
        .all(db)
        .await?;
        Ok(held
            .into_iter()
            .map(|h| ((h.dragonite_area_id, h.mode), h.geofence_id))
            .collect())
    }

    /// Every geofence holding an Area on the target, with the name and mode it
    /// holds it in, by geofence id. Counts every geofence, in reach or not.
    pub async fn holders<C: ConnectionTrait>(
        db: &C,
        webhook_id: u64,
    ) -> Result<Vec<AreaHolder>, DbErr> {
        AreaHolder::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT l.dragonite_area_id, l.geofence_id, g.name, g.mode \
             FROM dragonite_link l JOIN geofence g ON g.id = l.geofence_id \
             WHERE l.webhook_id = ? ORDER BY l.geofence_id",
            [webhook_id.into()],
        ))
        .all(db)
        .await
    }

    /// The target row, read `FOR UPDATE` so link writes to it run one at a time.
    /// Fails with [`LinkError::NoTarget`] or [`LinkError::NotDragonite`].
    pub async fn lock_target<C: ConnectionTrait>(
        db: &C,
        webhook_id: u64,
    ) -> Result<webhook::Model, LinkError> {
        let target = webhook::Entity::find_by_id(webhook_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or(LinkError::NoTarget(webhook_id))?;
        if target.mode != WebhookMode::Dragonite {
            return Err(LinkError::NotDragonite(webhook_id));
        }
        Ok(target)
    }

    /// Check what [`Self::set_links`] would check, writing nothing, so a caller
    /// can refuse a request before it changes anything else.
    pub async fn validate_links<C: ConnectionTrait>(
        db: &C,
        geofence_id: u32,
        mode: Mode,
        changes: &[LinkChange],
    ) -> Result<(), LinkError> {
        if geofence::Entity::find_by_id(geofence_id)
            .one(db)
            .await?
            .is_none()
        {
            return Err(LinkError::NoGeofence(geofence_id));
        }
        let mut wanted: std::collections::BTreeMap<u64, Option<u32>> =
            Self::for_geofence(db, geofence_id)
                .await?
                .into_iter()
                .map(|link| (link.webhook_id, Some(link.dragonite_area_id)))
                .collect();
        for change in changes {
            wanted.insert(change.webhook_id, change.dragonite_area_id);
        }
        // Targets in id order, so two writers never lock each other's rows
        // crosswise. A target the geofence is already linked on is locked even
        // when untouched: its Area is re-checked against `mode`.
        for (&webhook_id, area) in &wanted {
            let touched = changes.iter().any(|c| c.webhook_id == webhook_id);
            match Self::lock_target(db, webhook_id).await {
                Ok(_) => {}
                // A kept link on a row since switched to another mode is not
                // re-checked; one in the request must name a Dragonite row.
                Err(LinkError::NotDragonite(_)) if !touched => continue,
                Err(e) => return Err(e),
            }
            let Some(area) = *area else { continue };
            if let Some(held_by) = Self::holder(db, webhook_id, area, mode, geofence_id).await? {
                return Err(LinkError::AreaTaken {
                    webhook_id,
                    dragonite_area_id: area,
                    mode: mode_name(mode),
                    held_by,
                });
            }
        }
        Ok(())
    }

    /// Apply `changes` to a geofence's links, with the geofence in `mode`:
    /// an Area sets that target's link (replacing it), `None` clears it. Every
    /// link the geofence ends up with is checked against the target's other
    /// geofences of `mode`, so a mode change that makes two geofences share an
    /// Area is refused too. All or nothing.
    pub async fn set_links(
        db: &impl TransactionTrait,
        geofence_id: u32,
        mode: Mode,
        changes: &[LinkChange],
    ) -> Result<(), LinkError> {
        let txn = db.begin().await?;
        Self::validate_links(&txn, geofence_id, mode, changes).await?;
        let now = Utc::now().naive_utc();
        for change in changes {
            match change.dragonite_area_id {
                Some(area) => {
                    let row = ActiveModel {
                        webhook_id: sea_orm::Set(change.webhook_id),
                        geofence_id: sea_orm::Set(geofence_id),
                        dragonite_area_id: sea_orm::Set(area),
                        created_at: sea_orm::Set(now),
                        updated_at: sea_orm::Set(now),
                    };
                    Entity::insert(row)
                        .on_conflict(
                            sea_orm::sea_query::OnConflict::columns([
                                Column::WebhookId,
                                Column::GeofenceId,
                            ])
                            .update_columns([Column::DragoniteAreaId, Column::UpdatedAt])
                            .to_owned(),
                        )
                        .exec_without_returning(&txn)
                        .await?;
                }
                None => {
                    Entity::delete_many()
                        .filter(Column::WebhookId.eq(change.webhook_id))
                        .filter(Column::GeofenceId.eq(geofence_id))
                        .exec(&txn)
                        .await?;
                }
            }
        }
        txn.commit().await?;
        Ok(())
    }

    /// The geofence of `mode` other than `geofence_id` holding `area` on the target.
    async fn holder<C: ConnectionTrait>(
        db: &C,
        webhook_id: u64,
        area: u32,
        mode: Mode,
        geofence_id: u32,
    ) -> Result<Option<u32>, DbErr> {
        #[derive(FromQueryResult)]
        struct Holder {
            geofence_id: u32,
        }
        Ok(Holder::find_by_statement(Statement::from_sql_and_values(
            DbBackend::MySql,
            "SELECT l.geofence_id FROM dragonite_link l JOIN geofence g ON g.id = l.geofence_id \
             WHERE l.webhook_id = ? AND l.dragonite_area_id = ? AND g.mode = ? AND l.geofence_id <> ? \
             LIMIT 1",
            [
                webhook_id.into(),
                area.into(),
                mode_name(mode).into(),
                geofence_id.into(),
            ],
        ))
        .one(db)
        .await?
        .map(|h| h.geofence_id))
    }

    /// Link geofences to Areas on the target, skipping any that already hold a
    /// link on it. `links` is `(geofence, area)` pairs.
    pub async fn link_unlinked<C: ConnectionTrait>(
        db: &C,
        webhook_id: u64,
        links: &[(u32, u32)],
    ) -> Result<(), DbErr> {
        if links.is_empty() {
            return Ok(());
        }
        let now = Utc::now().naive_utc();
        let rows = links
            .iter()
            .map(|&(geofence_id, area)| ActiveModel {
                webhook_id: sea_orm::Set(webhook_id),
                geofence_id: sea_orm::Set(geofence_id),
                dragonite_area_id: sea_orm::Set(area),
                created_at: sea_orm::Set(now),
                updated_at: sea_orm::Set(now),
            })
            .collect::<Vec<_>>();
        Entity::insert_many(rows)
            .on_conflict(
                sea_orm::sea_query::OnConflict::columns([Column::WebhookId, Column::GeofenceId])
                    .do_nothing()
                    .to_owned(),
            )
            .do_nothing()
            .exec_without_returning(db)
            .await?;
        Ok(())
    }
}

fn mode_name(mode: Mode) -> String {
    use sea_orm::ActiveEnum;
    mode.to_value()
}
