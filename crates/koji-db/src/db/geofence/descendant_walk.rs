//! Which fences `descendants=N` adds to a selection. The walk itself is pure,
//! over the table's `(id, parent, mode)` edges, so it needs no database to test.

use std::collections::{HashMap, HashSet};

use crate::query_args::DescendantSpec;

use super::*;

/// One geofence's place in the hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromQueryResult)]
pub(super) struct Edge {
    pub(super) id: u32,
    pub(super) parent: Option<u32>,
    pub(super) mode: Mode,
}

/// The ids of the descendants `spec` adds below `seeds`, each once and none of
/// them a seed, in the order the walk meets them. A fence is walked at most once,
/// so a cycle in `parent` ends the walk instead of looping.
pub(super) fn descendant_ids(seeds: &[u32], edges: &[Edge], spec: &DescendantSpec) -> Vec<u32> {
    let mut children: HashMap<u32, Vec<&Edge>> = HashMap::new();
    for edge in edges {
        if let Some(parent) = edge.parent {
            children.entry(parent).or_default().push(edge);
        }
    }

    let mut walked: HashSet<u32> = HashSet::new();
    let mut level: Vec<u32> = seeds
        .iter()
        .copied()
        .filter(|id| walked.insert(*id))
        .collect();
    let mut added = vec![];
    for _ in 0..spec.depth {
        let mut next = vec![];
        for edge in level
            .iter()
            .filter_map(|id| children.get(id))
            .flatten()
            .filter(|edge| walked.insert(edge.id))
        {
            next.push(edge.id);
            if spec
                .modes
                .as_ref()
                .is_none_or(|modes| modes.contains(&edge.mode))
            {
                added.push(edge.id);
            }
        }
        level = next;
    }
    added
}

impl Query {
    /// The descendants `spec` adds below the `seeds`, read from the whole table
    /// whatever chose the seeds. No seed is in the result, and no id is in it
    /// twice.
    pub async fn descendant_ids<C: ConnectionTrait>(
        db: &C,
        seeds: &[u32],
        spec: &DescendantSpec,
    ) -> Result<Vec<u32>, DbErr> {
        if spec.depth == 0 || seeds.is_empty() {
            return Ok(vec![]);
        }
        let edges = Entity::find()
            .select_only()
            .column(Column::Id)
            .column(Column::Parent)
            .column(Column::Mode)
            .into_model::<Edge>()
            .all(db)
            .await?;
        Ok(descendant_ids(seeds, &edges, spec))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(id: u32, parent: Option<u32>, mode: Mode) -> Edge {
        Edge { id, parent, mode }
    }

    fn spec(depth: u32, modes: Option<Vec<Mode>>) -> DescendantSpec {
        DescendantSpec { depth, modes }
    }

    // 1 (state)
    //   2 (border, fort)    3 (border, unset)
    //     4 (pokemon)         5 (pokemon)
    //       6 (quest)
    // 7 (unrelated)
    fn tree() -> Vec<Edge> {
        vec![
            edge(1, None, Mode::Unset),
            edge(2, Some(1), Mode::Fort),
            edge(3, Some(1), Mode::Unset),
            edge(4, Some(2), Mode::Pokemon),
            edge(5, Some(3), Mode::Pokemon),
            edge(6, Some(4), Mode::Quest),
            edge(7, None, Mode::Unset),
        ]
    }

    fn sorted(mut ids: Vec<u32>) -> Vec<u32> {
        ids.sort();
        ids
    }

    #[test]
    fn depth_zero_adds_nothing() {
        assert_eq!(
            descendant_ids(&[1], &tree(), &spec(0, None)),
            Vec::<u32>::new()
        );
    }

    #[test]
    fn depth_one_adds_the_children() {
        let ids = descendant_ids(&[1], &tree(), &spec(1, None));
        assert_eq!(sorted(ids), vec![2, 3]);
    }

    #[test]
    fn depth_two_adds_the_grandchildren_too() {
        let ids = descendant_ids(&[1], &tree(), &spec(2, None));
        assert_eq!(sorted(ids), vec![2, 3, 4, 5]);
    }

    #[test]
    fn depth_past_the_leaves_stops_at_them() {
        let ids = descendant_ids(&[1], &tree(), &spec(10, None));
        assert_eq!(sorted(ids), vec![2, 3, 4, 5, 6]);
    }

    #[test]
    fn modes_limit_what_is_added_but_not_what_is_walked() {
        let only_pokemon = spec(2, Some(vec![Mode::Pokemon]));
        assert_eq!(
            sorted(descendant_ids(&[1], &tree(), &only_pokemon)),
            vec![4, 5],
            "the pokemon fences sit under borders of other modes"
        );
        let only_unset = spec(1, Some(vec![Mode::Unset]));
        assert_eq!(descendant_ids(&[1], &tree(), &only_unset), vec![3]);
    }

    #[test]
    fn a_seed_is_never_added_as_a_descendant() {
        let ids = descendant_ids(&[1, 2], &tree(), &spec(2, None));
        assert_eq!(sorted(ids), vec![3, 4, 5, 6]);
    }

    #[test]
    fn a_descendant_of_two_seeds_is_added_once() {
        let ids = descendant_ids(&[1, 3], &tree(), &spec(2, None));
        assert_eq!(sorted(ids), vec![2, 4, 5]);
    }

    #[test]
    fn a_cycle_in_parents_ends_the_walk() {
        // 1 -> 2 -> 3 -> 1
        let edges = vec![
            edge(1, Some(3), Mode::Unset),
            edge(2, Some(1), Mode::Unset),
            edge(3, Some(2), Mode::Unset),
        ];
        let ids = descendant_ids(&[1], &edges, &spec(10, None));
        assert_eq!(sorted(ids), vec![2, 3]);
    }

    #[test]
    fn a_fence_that_is_its_own_parent_adds_nothing() {
        let edges = vec![edge(1, Some(1), Mode::Unset)];
        assert_eq!(
            descendant_ids(&[1], &edges, &spec(10, None)),
            Vec::<u32>::new()
        );
    }
}
