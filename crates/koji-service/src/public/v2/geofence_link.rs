//! Link Geofences to a Dragonite target's Areas by name.
//!
//! [`plan_links`] is the pure core: given one target's Areas and the Geofences
//! that target reaches it decides which Geofences get linked to which Area, and
//! reports everything else. The `POST /geofences/link-dragonite` handler in
//! [`super::geofences`] reads both sides, plans, and writes the plan unless
//! `dry_run`.

use std::collections::{HashMap, HashSet};

use koji_db::db::{geofence::GeofenceLinkRow, sea_orm_active_enums::Mode};
use koji_dragonite::ApiArea;
use serde::Serialize;

/// A Geofence and the Dragonite Area it is (or would be) linked to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LinkedGeofence {
    pub geofence: u32,
    pub name: String,
    pub mode: Mode,
    pub dragonite_area_id: u32,
}

/// A Geofence that already has a different link than its name's Area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LinkConflict {
    pub geofence: u32,
    pub name: String,
    pub mode: Mode,
    /// The Area the Geofence is linked to now; left as it is.
    pub dragonite_area_id: u32,
    /// The Area whose name the Geofence carries.
    pub wanted_dragonite_area_id: u32,
}

/// A Geofence whose name's Area is already held by another Geofence of the
/// same mode on this target; left unlinked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct AreaTaken {
    pub geofence: u32,
    pub name: String,
    pub mode: Mode,
    pub dragonite_area_id: u32,
    /// The Geofence that holds the Area.
    pub held_by: u32,
}

/// A Dragonite Area no Geofence carries the name of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct UnmatchedArea {
    pub dragonite_area_id: u32,
    pub name: String,
}

/// A Geofence with no link whose name no Dragonite Area carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct UnmatchedGeofence {
    pub geofence: u32,
    pub name: String,
    pub mode: Mode,
}

/// What a link-by-name pass does (or, on a dry run, would do).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct LinkPlan {
    pub linked: Vec<LinkedGeofence>,
    pub already_linked: Vec<LinkedGeofence>,
    pub conflicts: Vec<LinkConflict>,
    pub area_taken: Vec<AreaTaken>,
    pub areas_without_geofence: Vec<UnmatchedArea>,
    pub geofences_without_area: Vec<UnmatchedGeofence>,
}

/// Match each Area to the Geofences with exactly its name, in every mode.
///
/// `geofences` are the ones the target reaches, with their link on it. `taken`
/// is every `(area, mode)` the target holds, with the Geofence holding it, in
/// reach or not.
///
/// - an unlinked Geofence is linked ([`LinkPlan::linked`]), unless another
///   Geofence of its mode holds that Area on this target
///   ([`LinkPlan::area_taken`]); this holds within a pass too
/// - one already linked to that Area is reported, not touched
///   ([`LinkPlan::already_linked`])
/// - one linked to another Area is a conflict and keeps its link; this holds
///   within a pass too, so of two Areas sharing a name the first listed wins
/// - an Area no Geofence is named after, and an unlinked Geofence no Area is
///   named after, are listed
///
/// An Area without a name or with an id Koji cannot store is skipped.
pub(crate) fn plan_links(
    areas: &[ApiArea],
    geofences: &[GeofenceLinkRow],
    taken: &HashMap<(u32, Mode), u32>,
) -> LinkPlan {
    let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
    for (i, geofence) in geofences.iter().enumerate() {
        by_name.entry(geofence.name.as_str()).or_default().push(i);
    }
    // Each Geofence's link as the plan leaves it, so a pass never hands one
    // Geofence to two Areas.
    let mut links: Vec<Option<u32>> = geofences.iter().map(|g| g.dragonite_area_id).collect();
    let mut taken = taken.clone();
    let mut named_by_an_area: HashSet<&str> = HashSet::new();
    let mut plan = LinkPlan::default();

    for area in areas {
        let (Some(name), Some(id)) = (
            area.name.as_deref(),
            area.id.and_then(|id| u32::try_from(id).ok()),
        ) else {
            continue;
        };
        let Some(matches) = by_name.get(name) else {
            plan.areas_without_geofence.push(UnmatchedArea {
                dragonite_area_id: id,
                name: name.to_string(),
            });
            continue;
        };
        named_by_an_area.insert(name);
        for &i in matches {
            let geofence = &geofences[i];
            let linked = |dragonite_area_id| LinkedGeofence {
                geofence: geofence.id,
                name: geofence.name.clone(),
                mode: geofence.mode,
                dragonite_area_id,
            };
            match links[i] {
                None => match taken.get(&(id, geofence.mode)) {
                    Some(&held_by) => plan.area_taken.push(AreaTaken {
                        geofence: geofence.id,
                        name: geofence.name.clone(),
                        mode: geofence.mode,
                        dragonite_area_id: id,
                        held_by,
                    }),
                    None => {
                        links[i] = Some(id);
                        taken.insert((id, geofence.mode), geofence.id);
                        plan.linked.push(linked(id));
                    }
                },
                Some(current) if current == id => plan.already_linked.push(linked(id)),
                Some(current) => plan.conflicts.push(LinkConflict {
                    geofence: geofence.id,
                    name: geofence.name.clone(),
                    mode: geofence.mode,
                    dragonite_area_id: current,
                    wanted_dragonite_area_id: id,
                }),
            }
        }
    }

    plan.geofences_without_area = geofences
        .iter()
        .filter(|g| g.dragonite_area_id.is_none() && !named_by_an_area.contains(g.name.as_str()))
        .map(|g| UnmatchedGeofence {
            geofence: g.id,
            name: g.name.clone(),
            mode: g.mode,
        })
        .collect();
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(id: i64, name: &str) -> ApiArea {
        ApiArea {
            id: Some(id),
            name: Some(name.to_string()),
            ..ApiArea::default()
        }
    }

    fn fence(id: u32, name: &str, mode: Mode, link: Option<u32>) -> GeofenceLinkRow {
        GeofenceLinkRow {
            id,
            name: name.to_string(),
            mode,
            dragonite_area_id: link,
        }
    }

    fn plan(areas: &[ApiArea], geofences: &[GeofenceLinkRow]) -> LinkPlan {
        plan_links(areas, geofences, &HashMap::new())
    }

    fn linked(geofence: u32, name: &str, mode: Mode, area: u32) -> LinkedGeofence {
        LinkedGeofence {
            geofence,
            name: name.to_string(),
            mode,
            dragonite_area_id: area,
        }
    }

    #[test]
    fn every_mode_of_a_name_is_linked_to_its_area() {
        let plan = plan(
            &[area(7, "Denver")],
            &[
                fence(1, "Denver", Mode::Pokemon, None),
                fence(2, "Denver", Mode::Quest, None),
            ],
        );

        assert_eq!(
            plan,
            LinkPlan {
                linked: vec![
                    linked(1, "Denver", Mode::Pokemon, 7),
                    linked(2, "Denver", Mode::Quest, 7)
                ],
                ..LinkPlan::default()
            }
        );
    }

    #[test]
    fn a_geofence_already_on_that_area_is_reported_not_relinked() {
        let plan = plan(
            &[area(7, "Denver")],
            &[fence(1, "Denver", Mode::Pokemon, Some(7))],
        );

        assert_eq!(
            plan,
            LinkPlan {
                already_linked: vec![linked(1, "Denver", Mode::Pokemon, 7)],
                ..LinkPlan::default()
            }
        );
    }

    #[test]
    fn a_geofence_on_a_different_area_is_a_conflict_and_keeps_its_link() {
        let plan = plan(
            &[area(7, "Denver")],
            &[fence(1, "Denver", Mode::Pokemon, Some(3))],
        );

        assert!(plan.linked.is_empty());
        assert_eq!(
            plan.conflicts,
            vec![LinkConflict {
                geofence: 1,
                name: "Denver".into(),
                mode: Mode::Pokemon,
                dragonite_area_id: 3,
                wanted_dragonite_area_id: 7,
            }]
        );
    }

    #[test]
    fn unmatched_areas_and_unlinked_geofences_are_listed() {
        let plan = plan(
            &[area(7, "Denver"), area(8, "Boulder")],
            &[
                fence(1, "Denver", Mode::Fort, None),
                fence(2, "Aurora", Mode::Fort, None),
                // hand-linked under another name: has an Area, so not listed
                fence(3, "Elsewhere", Mode::Fort, Some(40)),
            ],
        );

        assert_eq!(
            plan.areas_without_geofence,
            vec![UnmatchedArea {
                dragonite_area_id: 8,
                name: "Boulder".into()
            }]
        );
        assert_eq!(
            plan.geofences_without_area,
            vec![UnmatchedGeofence {
                geofence: 2,
                name: "Aurora".into(),
                mode: Mode::Fort
            }]
        );
    }

    #[test]
    fn names_match_exactly() {
        let plan = plan(
            &[area(7, "Denver")],
            &[fence(1, "denver", Mode::Fort, None)],
        );

        assert!(plan.linked.is_empty());
        assert_eq!(plan.areas_without_geofence.len(), 1);
        assert_eq!(plan.geofences_without_area.len(), 1);
    }

    #[test]
    fn two_areas_sharing_a_name_do_not_both_claim_a_geofence() {
        let plan = plan(
            &[area(7, "Denver"), area(9, "Denver")],
            &[fence(1, "Denver", Mode::Fort, None)],
        );

        assert_eq!(plan.linked, vec![linked(1, "Denver", Mode::Fort, 7)]);
        assert_eq!(plan.conflicts.len(), 1);
        assert_eq!(plan.conflicts[0].dragonite_area_id, 7);
        assert_eq!(plan.conflicts[0].wanted_dragonite_area_id, 9);
    }

    #[test]
    fn an_area_without_a_usable_id_or_name_is_skipped() {
        let nameless = ApiArea {
            id: Some(5),
            ..ApiArea::default()
        };
        let plan = plan(
            &[nameless, area(-1, "Denver"), area(i64::MAX, "Denver")],
            &[fence(1, "Denver", Mode::Fort, None)],
        );

        assert!(plan.linked.is_empty());
        assert_eq!(plan.geofences_without_area.len(), 1);
    }

    #[test]
    fn an_area_held_by_another_geofence_of_the_mode_is_not_linked() {
        // Geofence 9 (out of reach) already holds Area 7 in fort mode.
        let taken = HashMap::from([((7, Mode::Fort), 9)]);
        let plan = plan_links(
            &[area(7, "Denver")],
            &[
                fence(1, "Denver", Mode::Fort, None),
                fence(2, "Denver", Mode::Quest, None),
            ],
            &taken,
        );

        assert_eq!(plan.linked, vec![linked(2, "Denver", Mode::Quest, 7)]);
        assert_eq!(
            plan.area_taken,
            vec![AreaTaken {
                geofence: 1,
                name: "Denver".into(),
                mode: Mode::Fort,
                dragonite_area_id: 7,
                held_by: 9,
            }]
        );
        assert!(plan.geofences_without_area.is_empty());
    }

    #[test]
    fn two_geofences_of_one_name_and_mode_do_not_share_an_area_in_a_pass() {
        let plan = plan(
            &[area(7, "Denver")],
            &[
                fence(1, "Denver", Mode::Fort, None),
                fence(2, "Denver", Mode::Fort, None),
            ],
        );

        assert_eq!(plan.linked, vec![linked(1, "Denver", Mode::Fort, 7)]);
        assert_eq!(plan.area_taken.len(), 1);
        assert_eq!(plan.area_taken[0].geofence, 2);
        assert_eq!(plan.area_taken[0].held_by, 1);
    }
}
