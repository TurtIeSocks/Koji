//! What a geofence publish writes, decided before anything is sent.
//!
//! [`plan_publish`] is the pure core: given the geofences asked for and the
//! Dragonite targets that reach them it lists each write (a geofence to one
//! target's Area) and each geofence that sends nothing, with the reason. The
//! `POST /geofences/publish` handler in [`super::geofences`] plans, and sends
//! exactly the writes the plan lists unless `dry_run`, so the preview and the
//! send cannot disagree.

use std::collections::{HashMap, HashSet};

use koji_db::db::{
    dragonite_link::PublishTargets, geofence::GeofenceSummary, sea_orm_active_enums::Mode,
};
use serde::Serialize;

use crate::dragonite::{no_link_message, no_target_message};

/// One geofence going to one Dragonite Area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishWrite {
    pub geofence: u32,
    pub name: String,
    pub mode: Mode,
    pub webhook_id: u64,
    pub webhook_name: String,
    pub dragonite_area_id: u32,
    /// The Area's name on that Dragonite, when it could be read (a dry run
    /// asks each Dragonite; `null` if one does not answer or has no such Area).
    pub dragonite_area_name: Option<String>,
    /// The queued event, once the write is sent (absent on a dry run).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_id: Option<String>,
}

/// Why a geofence sends nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SkipReason {
    /// No geofence has this id.
    NotFound,
    /// No active Dragonite webhook reaches the geofence.
    NoTarget,
    /// The webhooks that reach it hold no link for it.
    NoLink,
}

/// A geofence that sends nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishSkip {
    pub geofence: u32,
    /// Absent for a [`SkipReason::NotFound`] id.
    pub name: Option<String>,
    pub mode: Option<Mode>,
    pub reason: SkipReason,
    pub message: String,
}

/// The writes a publish sends and the geofences it skips.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct PublishPlan {
    pub writes: Vec<PublishWrite>,
    pub skipped: Vec<PublishSkip>,
}

/// Plan publishing `ids` (in that order, each once). `found` holds the targets
/// that reach each existing geofence, as [`PublishTargets`] reads them.
pub(crate) fn plan_publish(
    ids: &[u32],
    geofences: &[GeofenceSummary],
    found: &HashMap<u32, PublishTargets>,
) -> PublishPlan {
    let by_id: HashMap<u32, &GeofenceSummary> = geofences.iter().map(|g| (g.id, g)).collect();
    let mut seen = HashSet::new();
    let mut plan = PublishPlan::default();
    for &id in ids.iter().filter(|id| seen.insert(**id)) {
        let Some(geofence) = by_id.get(&id) else {
            plan.skipped.push(PublishSkip {
                geofence: id,
                name: None,
                mode: None,
                reason: SkipReason::NotFound,
                message: format!("no geofence {id}"),
            });
            continue;
        };
        let skip = |reason, message| PublishSkip {
            geofence: id,
            name: Some(geofence.name.clone()),
            mode: Some(geofence.mode),
            reason,
            message,
        };
        let Some(targets) = found.get(&id).filter(|t| !t.reaching.is_empty()) else {
            plan.skipped.push(skip(
                SkipReason::NoTarget,
                no_target_message(&geofence.name),
            ));
            continue;
        };
        if targets.linked.is_empty() {
            plan.skipped.push(skip(
                SkipReason::NoLink,
                no_link_message(&geofence.name, &targets.reaching),
            ));
            continue;
        }
        for (target, link) in &targets.linked {
            plan.writes.push(PublishWrite {
                geofence: id,
                name: geofence.name.clone(),
                mode: geofence.mode,
                webhook_id: target.id,
                webhook_name: target.name.clone(),
                dragonite_area_id: link.dragonite_area_id,
                dragonite_area_name: None,
                event_id: None,
            });
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_db::db::{
        dragonite_link,
        sea_orm_active_enums::{WebhookMethod, WebhookMode},
        webhook,
    };

    fn fence(id: u32, name: &str, mode: Mode) -> GeofenceSummary {
        GeofenceSummary {
            id,
            name: name.to_string(),
            mode,
        }
    }

    fn hook(id: u64, name: &str) -> webhook::Model {
        webhook::Model {
            id,
            name: name.to_string(),
            url: format!("http://dragonite-{id}"),
            secret: None,
            topics: serde_json::json!([]),
            active: true,
            project_id: None,
            mode: WebhookMode::Dragonite,
            method: WebhookMethod::Post,
            headers: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    fn link(webhook_id: u64, geofence_id: u32, area: u32) -> dragonite_link::Model {
        dragonite_link::Model {
            webhook_id,
            geofence_id,
            dragonite_area_id: area,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    #[test]
    fn lists_one_write_per_linked_target_and_skips_the_rest_with_a_reason() {
        let east = hook(1, "east");
        let west = hook(2, "west");
        let fences = [
            fence(10, "linked", Mode::Quest),
            fence(11, "unlinked", Mode::Pokemon),
            fence(12, "unreached", Mode::Fort),
        ];
        let found = HashMap::from([
            (
                10,
                PublishTargets {
                    reaching: vec![east.clone(), west.clone()],
                    linked: vec![
                        (east.clone(), link(1, 10, 7)),
                        (west.clone(), link(2, 10, 9)),
                    ],
                },
            ),
            (
                11,
                PublishTargets {
                    reaching: vec![east],
                    linked: vec![],
                },
            ),
            (
                12,
                PublishTargets {
                    reaching: vec![],
                    linked: vec![],
                },
            ),
        ]);

        let plan = plan_publish(&[10, 11, 12, 99, 10], &fences, &found);

        let writes: Vec<_> = plan
            .writes
            .iter()
            .map(|w| (w.geofence, w.mode, w.webhook_id, w.dragonite_area_id))
            .collect();
        assert_eq!(
            writes,
            [(10, Mode::Quest, 1, 7), (10, Mode::Quest, 2, 9)],
            "a repeated id writes once"
        );
        let skipped: Vec<_> = plan
            .skipped
            .iter()
            .map(|s| (s.geofence, s.reason))
            .collect();
        assert_eq!(
            skipped,
            [
                (11, SkipReason::NoLink),
                (12, SkipReason::NoTarget),
                (99, SkipReason::NotFound)
            ]
        );
        assert!(plan.skipped[0].message.contains("`east` (id 1)"));
    }
}
