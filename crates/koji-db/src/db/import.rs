//! Atomic bulk import: validate a batch (dry-run) then commit it in one
//! transaction. Geofences upsert first (collecting name->id) so routes resolve
//! their parent fence in the same tx; any hard failure rolls the whole tx back.

use std::collections::{HashMap, HashSet};

use sea_orm::{ActiveEnum, DatabaseConnection, EntityTrait, TransactionTrait};
use serde_json::json;

use crate::db::sea_orm_active_enums::Mode;
use crate::db::{geofence, route};
use crate::error::ModelError;
use crate::utils::get_enum;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportKind {
    Geofence,
    Route,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnCollision {
    Skip,
    Overwrite,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportAction {
    Create,
    Update,
    Skip,
    Fail,
}

#[derive(Clone, Debug)]
pub struct ImportItem {
    pub kind: ImportKind,
    pub name: String,
    pub geometry: serde_json::Value,
    pub mode: Option<String>,
    pub parent: Option<String>,
    pub projects: Vec<u32>,
    pub route_parent: Option<String>,
    pub on_collision: OnCollision,
}

#[derive(Clone, Debug)]
pub struct ImportOutcome {
    pub index: usize,
    pub name: String,
    pub action: ImportAction,
    pub id: Option<u32>,
    pub reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ImportSummary {
    pub create: usize,
    pub update: usize,
    pub skip: usize,
    pub fail: usize,
}

#[derive(Clone, Debug)]
pub struct ImportResult {
    pub committed: bool,
    pub summary: ImportSummary,
    pub results: Vec<ImportOutcome>,
}

/// A geofence is keyed by its name and mode together.
type FenceKey = (String, Mode);

/// The storage mode an item lands in (`unset` when it names none).
fn item_mode(it: &ImportItem) -> Mode {
    get_enum(it.mode.as_deref())
}

/// What a parent name points at among the known fences.
#[derive(Debug, PartialEq, Eq)]
enum ParentRef {
    Found(FenceKey),
    Missing,
    /// Several modes share the name and none is the child's own.
    Ambiguous(Vec<Mode>),
}

/// Resolve a parent given by name only. The fence in the child's own mode wins;
/// otherwise a name held by a single mode is that fence; otherwise it is
/// ambiguous.
fn resolve_parent<'a>(
    known: impl Iterator<Item = &'a FenceKey>,
    name: &str,
    child_mode: Mode,
) -> ParentRef {
    let mut modes: Vec<Mode> = known.filter(|(n, _)| n == name).map(|(_, m)| *m).collect();
    if modes.contains(&child_mode) {
        return ParentRef::Found((name.to_string(), child_mode));
    }
    match modes.len() {
        0 => ParentRef::Missing,
        1 => ParentRef::Found((name.to_string(), modes.remove(0))),
        _ => ParentRef::Ambiguous(modes),
    }
}

fn mode_list(modes: &[Mode]) -> String {
    let mut modes: Vec<String> = modes.iter().map(|m| m.to_value()).collect();
    modes.sort();
    modes.join(", ")
}

/// Why a parent name cannot be used, or `None` when it resolves.
fn parent_problem<'a>(
    known: impl Iterator<Item = &'a FenceKey>,
    name: &str,
    child_mode: Mode,
    label: &str,
) -> Option<String> {
    match resolve_parent(known, name, child_mode) {
        ParentRef::Found(_) => None,
        ParentRef::Missing => Some(format!("{label} `{name}` not found")),
        ParentRef::Ambiguous(modes) => Some(format!(
            "{label} `{name}` is ambiguous, it exists in modes: {}",
            mode_list(&modes)
        )),
    }
}

/// Pure per-item validation. Returns `Some(reason)` for each item that cannot
/// be imported, else `None`. `existing` = (name, mode) of geofences already in
/// the DB.
pub(crate) fn validate(items: &[ImportItem], existing: &HashSet<FenceKey>) -> Vec<Option<String>> {
    // Fences that WILL exist after this batch (existing ∪ batch).
    let mut known: HashSet<FenceKey> = existing.clone();
    let mut seen_fences: HashSet<FenceKey> = HashSet::new();
    let mut dup_fences: HashSet<FenceKey> = HashSet::new();
    for it in items {
        if it.kind == ImportKind::Geofence {
            let n = it.name.trim().to_string();
            if !n.is_empty() {
                let key = (n, item_mode(it));
                if !seen_fences.insert(key.clone()) {
                    dup_fences.insert(key.clone());
                }
                known.insert(key);
            }
        }
    }

    items
        .iter()
        .map(|it| {
            let name = it.name.trim();
            let mode = item_mode(it);
            if name.is_empty() {
                return Some("empty name".to_string());
            }
            if it.geometry.get("type").and_then(|t| t.as_str()).is_none() {
                return Some("geometry missing a `type`".to_string());
            }
            if it.kind == ImportKind::Geofence && dup_fences.contains(&(name.to_string(), mode)) {
                return Some(format!(
                    "duplicate geofence `{name}` in mode `{}` in batch",
                    mode.to_value()
                ));
            }
            if let Some(p) = it.parent.as_deref()
                && let Some(problem) = parent_problem(known.iter(), p, mode, "parent")
            {
                return Some(problem);
            }
            if it.kind == ImportKind::Route {
                match it.route_parent.as_deref() {
                    Some(p) => {
                        if let Some(problem) = parent_problem(known.iter(), p, mode, "route parent")
                        {
                            return Some(problem);
                        }
                    }
                    None => return Some("route has no parent geofence".to_string()),
                }
            }
            None
        })
        .collect()
}

/// Map existing geofence (name, mode) -> id (for collision + parent resolution).
async fn existing_geofence_ids(
    db: &DatabaseConnection,
) -> Result<HashMap<FenceKey, u32>, ModelError> {
    let rows = geofence::Entity::find().all(db).await?;
    Ok(rows.into_iter().map(|m| ((m.name, m.mode), m.id)).collect())
}

pub async fn import(
    db: &DatabaseConnection,
    items: Vec<ImportItem>,
    dry_run: bool,
) -> Result<ImportResult, ModelError> {
    let mut name_to_id = existing_geofence_ids(db).await?;
    let existing: HashSet<FenceKey> = name_to_id.keys().cloned().collect();
    let reasons = validate(&items, &existing);

    // Predict actions for every item (used by both dry-run and the fail-fast guard).
    let predicted: Vec<ImportOutcome> = items
        .iter()
        .enumerate()
        .map(|(index, it)| {
            let name = it.name.trim().to_string();
            if let Some(reason) = reasons[index].clone() {
                return ImportOutcome {
                    index,
                    name,
                    action: ImportAction::Fail,
                    id: None,
                    reason: Some(reason),
                };
            }
            if it.kind == ImportKind::Geofence && existing.contains(&(name.clone(), item_mode(it)))
            {
                let action = match it.on_collision {
                    OnCollision::Overwrite => ImportAction::Update,
                    OnCollision::Skip => ImportAction::Skip,
                };
                let reason = (action == ImportAction::Skip)
                    .then(|| "collision, on_collision=skip".to_string());
                return ImportOutcome {
                    index,
                    name,
                    action,
                    id: None,
                    reason,
                };
            }
            ImportOutcome {
                index,
                name,
                action: ImportAction::Create,
                id: None,
                reason: None,
            }
        })
        .collect();

    let has_fail = predicted.iter().any(|o| o.action == ImportAction::Fail);

    if dry_run || has_fail {
        return Ok(ImportResult {
            committed: false,
            summary: summarize(&predicted),
            results: predicted,
        });
    }

    // --- Commit pass: one transaction. ---
    let txn = db.begin().await?;
    let mut results = Vec::with_capacity(items.len());

    // Geofences first (so routes resolve parents in the same tx).
    for (index, it) in items.iter().enumerate() {
        if it.kind != ImportKind::Geofence {
            continue;
        }
        let name = it.name.trim().to_string();
        let predicted_action = predicted[index].action;
        if predicted_action == ImportAction::Skip {
            results.push(predicted[index].clone());
            continue;
        }
        // Parent is set in a deferred pass below (after every batch fence is in
        // name_to_id), so a parent that forward-references a fence listed later
        // in the batch still links — never silently dropped.
        let mut body = json!({
            "name": name,
            "geometry": it.geometry,
            "projects": it.projects,
            "properties": [],
        });
        if let Some(mode) = &it.mode {
            body["mode"] = json!(mode);
        }
        match geofence::Query::upsert(&txn, 0, body).await {
            Ok(model) => {
                name_to_id.insert((name.clone(), model.mode), model.id);
                results.push(ImportOutcome {
                    index,
                    name,
                    action: predicted_action,
                    id: Some(model.id),
                    reason: None,
                });
            }
            Err(e) => {
                txn.rollback().await?;
                return Ok(fail_result(index, name, e.to_string()));
            }
        }
    }

    // Parent-association pass: every batch geofence is now in name_to_id, so a
    // parent that forward-references a fence listed later in the batch resolves
    // here (validation already guaranteed it exists). Skipped-collision fences
    // keep whatever parent they already have.
    for (index, it) in items.iter().enumerate() {
        if it.kind != ImportKind::Geofence || predicted[index].action == ImportAction::Skip {
            continue;
        }
        let Some(parent_name) = it.parent.as_deref() else {
            continue;
        };
        let name = it.name.trim().to_string();
        let mode = item_mode(it);
        let parent_id = match resolve_parent(name_to_id.keys(), parent_name, mode) {
            ParentRef::Found(key) => name_to_id.get(&key).copied(),
            _ => None,
        };
        let (Some(child_id), Some(parent_id)) =
            (name_to_id.get(&(name.clone(), mode)).copied(), parent_id)
        else {
            txn.rollback().await?;
            return Ok(fail_result(
                index,
                name,
                format!("parent `{parent_name}` not found"),
            ));
        };
        if let Err(e) = geofence::Query::set_parent(&txn, child_id, Some(parent_id)).await {
            txn.rollback().await?;
            return Ok(fail_result(index, name, e.to_string()));
        }
    }

    // Routes second.
    for (index, it) in items.iter().enumerate() {
        if it.kind != ImportKind::Route {
            continue;
        }
        let name = it.name.trim().to_string();
        let parent_id = it.route_parent.as_deref().and_then(|p| {
            match resolve_parent(name_to_id.keys(), p, item_mode(it)) {
                ParentRef::Found(key) => name_to_id.get(&key).copied(),
                _ => None,
            }
        });
        let Some(parent_id) = parent_id else {
            txn.rollback().await?;
            return Ok(fail_result(
                index,
                name,
                "route parent not found".to_string(),
            ));
        };
        let mut body = json!({
            "name": name,
            "geometry": it.geometry,
            "geofence_id": parent_id,
        });
        if let Some(mode) = &it.mode {
            body["mode"] = json!(mode);
        }
        match route::Query::upsert(&txn, 0, body).await {
            Ok(model) => results.push(ImportOutcome {
                index,
                name,
                action: ImportAction::Create,
                id: Some(model.id),
                reason: None,
            }),
            Err(e) => {
                txn.rollback().await?;
                return Ok(fail_result(index, name, e.to_string()));
            }
        }
    }

    txn.commit().await?;
    Ok(ImportResult {
        committed: true,
        summary: summarize(&results),
        results,
    })
}

fn summarize(results: &[ImportOutcome]) -> ImportSummary {
    let mut s = ImportSummary::default();
    for o in results {
        match o.action {
            ImportAction::Create => s.create += 1,
            ImportAction::Update => s.update += 1,
            ImportAction::Skip => s.skip += 1,
            ImportAction::Fail => s.fail += 1,
        }
    }
    s
}

/// Build a rolled-back result whose only outcome is the failure that aborted
/// the transaction. `committed:false` signals nothing was written.
fn fail_result(index: usize, name: String, reason: String) -> ImportResult {
    let results = vec![ImportOutcome {
        index,
        name,
        action: ImportAction::Fail,
        id: None,
        reason: Some(reason),
    }];
    ImportResult {
        committed: false,
        summary: summarize(&results),
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn item(kind: ImportKind, name: &str) -> ImportItem {
        ImportItem {
            kind,
            name: name.to_string(),
            geometry: serde_json::json!({ "type": "Polygon", "coordinates": [] }),
            mode: None,
            parent: None,
            projects: vec![],
            route_parent: None,
            on_collision: OnCollision::Skip,
        }
    }

    #[test]
    fn empty_name_is_a_failure() {
        let items = vec![item(ImportKind::Geofence, "  ")];
        let reasons = validate(&items, &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("name"));
    }

    #[test]
    fn duplicate_geofence_names_in_batch_fail_both() {
        let items = vec![
            item(ImportKind::Geofence, "Dup"),
            item(ImportKind::Geofence, "Dup"),
        ];
        let reasons = validate(&items, &HashSet::new());
        assert!(reasons[0].is_some() && reasons[1].is_some());
    }

    #[test]
    fn route_parent_unresolvable_fails() {
        let mut r = item(ImportKind::Route, "patrol");
        r.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [] });
        r.route_parent = Some("NoSuchFence".to_string());
        let reasons = validate(&[r], &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("parent"));
    }

    #[test]
    fn route_parent_resolvable_against_batch_geofence_passes() {
        let fence = item(ImportKind::Geofence, "Region-A");
        let mut route = item(ImportKind::Route, "patrol");
        route.geometry = serde_json::json!({ "type": "MultiPoint", "coordinates": [] });
        route.route_parent = Some("Region-A".to_string());
        let reasons = validate(&[fence, route], &HashSet::new());
        assert!(reasons[1].is_none());
    }

    #[test]
    fn missing_geometry_type_fails() {
        let mut g = item(ImportKind::Geofence, "NoGeom");
        g.geometry = serde_json::json!({ "coordinates": [] });
        let reasons = validate(&[g], &HashSet::new());
        assert!(reasons[0].as_deref().unwrap().contains("geometry"));
    }

    fn with_mode(mut it: ImportItem, mode: &str) -> ImportItem {
        it.mode = Some(mode.to_string());
        it
    }

    #[test]
    fn same_name_in_two_modes_is_not_a_duplicate() {
        let items = vec![
            with_mode(item(ImportKind::Geofence, "Boston"), "unset"),
            with_mode(item(ImportKind::Geofence, "Boston"), "pokemon"),
        ];
        let reasons = validate(&items, &HashSet::new());
        assert_eq!(reasons, vec![None, None]);
    }

    #[test]
    fn same_name_and_mode_twice_is_a_duplicate() {
        let items = vec![
            with_mode(item(ImportKind::Geofence, "Boston"), "pokemon"),
            with_mode(item(ImportKind::Geofence, "Boston"), "pokemon"),
        ];
        let reasons = validate(&items, &HashSet::new());
        assert!(
            reasons
                .iter()
                .all(|r| r.as_deref().unwrap().contains("duplicate"))
        );
    }

    #[test]
    fn parent_prefers_the_childs_own_mode() {
        let known: HashSet<FenceKey> = [
            ("Boston".to_string(), Mode::Unset),
            ("Boston".to_string(), Mode::Pokemon),
        ]
        .into();
        assert_eq!(
            resolve_parent(known.iter(), "Boston", Mode::Pokemon),
            ParentRef::Found(("Boston".to_string(), Mode::Pokemon))
        );
    }

    #[test]
    fn parent_in_one_other_mode_still_resolves() {
        let known: HashSet<FenceKey> = [("Boston".to_string(), Mode::Unset)].into();
        assert_eq!(
            resolve_parent(known.iter(), "Boston", Mode::Pokemon),
            ParentRef::Found(("Boston".to_string(), Mode::Unset))
        );
    }

    #[test]
    fn parent_in_two_other_modes_is_ambiguous_and_named_so() {
        let existing: HashSet<FenceKey> = [
            ("Boston".to_string(), Mode::Unset),
            ("Boston".to_string(), Mode::Pokemon),
        ]
        .into();
        let mut child = with_mode(item(ImportKind::Geofence, "Fenway"), "fort");
        child.parent = Some("Boston".to_string());
        let reasons = validate(&[child], &existing);
        let reason = reasons[0].as_deref().unwrap();
        assert!(reason.contains("ambiguous"), "{reason}");
        assert!(reason.contains("pokemon, unset"), "{reason}");
    }

    #[test]
    fn existing_fence_in_another_mode_does_not_collide() {
        let existing: HashSet<FenceKey> = [("Boston".to_string(), Mode::Unset)].into();
        let items = vec![with_mode(item(ImportKind::Geofence, "Boston"), "pokemon")];
        assert_eq!(validate(&items, &existing), vec![None]);
    }
}
