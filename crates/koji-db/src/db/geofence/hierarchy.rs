//! Recursive-descendants hierarchy concern (Phase 2 §7): the [`Anchor`] /
//! [`HierarchySpec`] request shapes, the bound-param CTE builder
//! ([`build_descendants_sql`]), the [`DescendantRow`] CTE projection, and the
//! ancestry-path helper. The live `Query::descendants` method lives in
//! `reads.rs`. Split out of the former `geofence.rs` god-file as a **pure
//! relocation** — no logic change. The public types are re-exported from
//! `mod.rs` so the path `geofence::{Anchor, HierarchySpec, HierarchyArgError}`
//! is unchanged.

use super::*;

/// The root anchor for a recursive descendants walk (Phase 2 §7).
///
/// - [`Anchor::Id`] / [`Anchor::Name`] anchor on a single root geofence (the
///   single-root `GET /api/v2/geofences/{id_or_name}` path).
/// - [`Anchor::Forest`] anchors on every top-level geofence (`parent IS NULL`),
///   the forest `GET /api/v2/geofences` path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Anchor {
    Id(u32),
    Name(String),
    /// All roots: `parent IS NULL`.
    Forest,
}

/// The recursion shape for a descendants walk (Phase 2 §7). Mutually exclusive
/// at the API surface — see [`HierarchySpec::from_args`].
///
/// - [`HierarchySpec::Depth`]`(N)` = cumulative subtree: root (depth 0) through
///   level `N` inclusive. `Depth(0)` = root(s) only.
/// - [`HierarchySpec::Level`]`(N)` = ONLY the geofences exactly `N` levels below
///   the anchor. `Level(0)` = the root(s) themselves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HierarchySpec {
    Depth(u32),
    Level(u32),
}

/// `?depth` and `?level` are mutually exclusive; supplying both is a client
/// error (HTTP 400 at the service layer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HierarchyArgError;

impl HierarchySpec {
    /// Resolve the optional `?depth` / `?level` query args into a recursion
    /// spec. Both `Some` → [`HierarchyArgError`] (mutually exclusive). Both
    /// `None` → `Ok(None)` (the caller falls back to the non-recursive path).
    pub fn from_args(
        depth: Option<u32>,
        level: Option<u32>,
    ) -> Result<Option<Self>, HierarchyArgError> {
        match (depth, level) {
            (Some(_), Some(_)) => Err(HierarchyArgError),
            (Some(d), None) => Ok(Some(HierarchySpec::Depth(d))),
            (None, Some(l)) => Ok(Some(HierarchySpec::Level(l))),
            (None, None) => Ok(None),
        }
    }
}

/// One row from the recursive `descendants` CTE. Mirrors the geofence columns
/// (so the row can be re-hydrated into a [`Model`] for
/// [`Model::to_koji_geometry`]) plus the synthesized `ancestry` path string
/// (root→self names joined by U+001F).
#[derive(FromQueryResult)]
pub(super) struct DescendantRow {
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) parent: Option<u32>,
    pub(super) created_at: DateTimeUtc,
    pub(super) updated_at: DateTimeUtc,
    pub(super) mode: Mode,
    pub(super) geometry: Json,
    pub(super) geo_type: String,
    pub(super) dragonite_area_id: Option<u32>,
    pub(super) ancestry: String,
}

/// The unit separator (U+001F) joining names in the `ancestry` path string.
/// Chosen over a printable delimiter so it can never collide with a geofence
/// name.
const ANCESTRY_SEP: char = '\u{1f}';

/// Split an `ancestry` path (root→self names joined by U+001F) into the ordered
/// root→parent ancestor names, dropping the row's own trailing name. A root row
/// (ancestry == its own name) yields `[]`.
pub(super) fn ancestors_from_ancestry(ancestry: &str) -> Vec<String> {
    let mut parts = ancestry
        .split(ANCESTRY_SEP)
        .map(str::to_string)
        .collect::<Vec<String>>();
    parts.pop(); // drop own name
    parts
}

/// Build the recursive descendants SQL + bound params for the given anchor and
/// recursion spec. NEVER string-interpolates user input — the anchor id/name and
/// the recursion bound / exact-level are all bound `?` params (see the returned
/// `Vec<Value>`). The emitted SQL uses explicit column lists (no `g.*`) and a
/// collision-safe U+001F (`CHAR(31 USING utf8mb4)`) separator for the `ancestry`
/// path, which is widened to `CHAR(4096)` in the anchor row.
pub(super) fn build_descendants_sql(anchor: &Anchor, spec: &HierarchySpec) -> (String, Vec<Value>) {
    let mut binds: Vec<Value> = Vec::new();

    // Anchor predicate of the CTE seed.
    let anchor_where = match anchor {
        Anchor::Id(id) => {
            binds.push((*id).into());
            "g.id = ?"
        }
        Anchor::Name(name) => {
            binds.push(name.clone().into());
            "g.name = ?"
        }
        Anchor::Forest => "g.parent IS NULL",
    };

    // Recursion bound (depth < N) — N is the same for both variants.
    let bound = match spec {
        HierarchySpec::Depth(n) | HierarchySpec::Level(n) => *n,
    };
    binds.push(bound.into());

    // Level(N) additionally filters the final select to exactly depth N.
    let outer_where = match spec {
        HierarchySpec::Depth(_) => String::new(),
        HierarchySpec::Level(n) => {
            binds.push((*n).into());
            "\nWHERE depth = ?".to_string()
        }
    };

    let sql = format!(
        "WITH RECURSIVE descendants AS (
  SELECT g.id, g.name, g.parent, g.created_at, g.updated_at, g.mode, g.geometry, g.geo_type, g.dragonite_area_id,
         CAST(0 AS SIGNED) AS depth,
         CAST(g.name AS CHAR(4096)) AS ancestry
  FROM geofence g
  WHERE {anchor_where}
  UNION ALL
  SELECT g.id, g.name, g.parent, g.created_at, g.updated_at, g.mode, g.geometry, g.geo_type, g.dragonite_area_id,
         d.depth + 1,
         CONCAT(d.ancestry, CHAR(31 USING utf8mb4), g.name)
  FROM geofence g JOIN descendants d ON g.parent = d.id
  WHERE d.depth < ?
)
SELECT id, name, parent, created_at, updated_at, mode, geometry, geo_type, dragonite_area_id, ancestry
FROM descendants{outer_where};"
    );

    (sql, binds)
}

/// DB-free unit tests for the recursive-descendants helpers (Phase 2 §7): the
/// ancestry-path split, the emitted CTE SQL fragments + bind params per anchor/
/// spec variant, and the mutually-exclusive `from_args` resolver. The live
/// `Query::descendants` half needs a DB and is exercised by integration.
#[cfg(test)]
mod descendants_tests {
    use super::*;

    #[test]
    fn ancestors_root_only_is_empty() {
        assert_eq!(ancestors_from_ancestry("country"), Vec::<String>::new());
    }

    #[test]
    fn ancestors_drops_own_trailing_name() {
        assert_eq!(
            ancestors_from_ancestry("country\u{1f}state\u{1f}county"),
            vec!["country".to_string(), "state".to_string()]
        );
    }

    #[test]
    fn ancestors_empty_string_is_empty() {
        // "" splits to [""], pop drops it → [].
        assert_eq!(ancestors_from_ancestry(""), Vec::<String>::new());
    }

    /// Pull a `&str` out of a `sea_orm::Value::String` bind for assertions.
    fn as_str(v: &Value) -> &str {
        match v {
            Value::String(Some(s)) => s.as_str(),
            other => panic!("expected string bind, got {other:?}"),
        }
    }

    /// Pull a `u32` out of a `sea_orm::Value::Unsigned` bind for assertions.
    fn as_u32(v: &Value) -> u32 {
        match v {
            Value::Unsigned(Some(n)) => *n,
            other => panic!("expected u32 bind, got {other:?}"),
        }
    }

    /// Every emitted SQL — regardless of anchor/spec — carries the recursion
    /// bound, the collision-safe U+001F separator, and the widened ancestry CAST.
    fn assert_common_fragments(sql: &str) {
        assert!(sql.contains("WITH RECURSIVE descendants AS"));
        assert!(sql.contains("WHERE d.depth < ?"));
        assert!(sql.contains("CONCAT(d.ancestry, CHAR(31 USING utf8mb4), g.name)"));
        assert!(sql.contains("CAST(g.name AS CHAR(4096)) AS ancestry"));
        // explicit column lists, never g.*
        assert!(!sql.contains("g.*"));
    }

    #[test]
    fn sql_id_depth() {
        let (sql, binds) = build_descendants_sql(&Anchor::Id(42), &HierarchySpec::Depth(2));
        assert_common_fragments(&sql);
        assert!(sql.contains("WHERE g.id = ?"));
        assert!(!sql.contains("WHERE depth = ?")); // Depth → no outer filter
        // binds = [anchor id, bound]
        assert_eq!(binds.len(), 2);
        assert_eq!(as_u32(&binds[0]), 42);
        assert_eq!(as_u32(&binds[1]), 2);
    }

    #[test]
    fn sql_name_level() {
        let (sql, binds) =
            build_descendants_sql(&Anchor::Name("x".to_string()), &HierarchySpec::Level(3));
        assert_common_fragments(&sql);
        assert!(sql.contains("WHERE g.name = ?"));
        assert!(sql.contains("WHERE depth = ?")); // Level → outer filter
        // binds = [anchor name, bound, level_outer]
        assert_eq!(binds.len(), 3);
        assert_eq!(as_str(&binds[0]), "x");
        assert_eq!(as_u32(&binds[1]), 3);
        assert_eq!(as_u32(&binds[2]), 3);
    }

    #[test]
    fn sql_forest_depth() {
        let (sql, binds) = build_descendants_sql(&Anchor::Forest, &HierarchySpec::Depth(2));
        assert_common_fragments(&sql);
        assert!(sql.contains("WHERE g.parent IS NULL"));
        assert!(!sql.contains("WHERE depth = ?"));
        // binds = [bound] (no anchor bind for the forest)
        assert_eq!(binds.len(), 1);
        assert_eq!(as_u32(&binds[0]), 2);
    }

    #[test]
    fn sql_forest_level() {
        let (sql, binds) = build_descendants_sql(&Anchor::Forest, &HierarchySpec::Level(2));
        assert_common_fragments(&sql);
        assert!(sql.contains("WHERE g.parent IS NULL"));
        assert!(sql.contains("WHERE depth = ?"));
        // binds = [bound, level_outer]
        assert_eq!(binds.len(), 2);
        assert_eq!(as_u32(&binds[0]), 2);
        assert_eq!(as_u32(&binds[1]), 2);
    }

    #[test]
    fn from_args_both_is_err() {
        assert_eq!(
            HierarchySpec::from_args(Some(1), Some(2)),
            Err(HierarchyArgError)
        );
    }

    #[test]
    fn from_args_depth_only() {
        assert_eq!(
            HierarchySpec::from_args(Some(2), None),
            Ok(Some(HierarchySpec::Depth(2)))
        );
    }

    #[test]
    fn from_args_level_only() {
        assert_eq!(
            HierarchySpec::from_args(None, Some(3)),
            Ok(Some(HierarchySpec::Level(3)))
        );
    }

    #[test]
    fn from_args_neither_is_none() {
        assert_eq!(HierarchySpec::from_args(None, None), Ok(None));
    }
}
