//! Read paths for the geofence entity (single/all fetches, the recursive
//! descendants walk, and `search`). Split out of the former `geofence.rs`
//! god-file as a **pure relocation** — `impl super::Query`, no logic change.

use std::collections::{HashMap, HashSet};

use koji_core::KojiGeometryCollection;

use crate::error::ModelError;

use super::*;

use serde_json::json;

impl Query {
    /// The id, name and mode of each geofence in `ids` that exists, by id.
    /// Ids with no row are left out.
    pub async fn summaries<C: ConnectionTrait>(
        db: &C,
        ids: &[u32],
    ) -> Result<Vec<GeofenceSummary>, DbErr> {
        let mut found = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(5_000) {
            found.extend(
                Entity::find()
                    .select_only()
                    .columns([Column::Id, Column::Name, Column::Mode])
                    .filter(Column::Id.is_in(chunk.iter().copied()))
                    .order_by_asc(Column::Id)
                    .into_model::<GeofenceSummary>()
                    .all(db)
                    .await?,
            );
        }
        Ok(found)
    }

    /// Recursive descendants walk (Phase 2 §7): from `anchor` (a single root by
    /// id/name, or the whole forest = `parent IS NULL`), return a flat
    /// [`KojiGeometryCollection`] of geofences per `spec` — either the cumulative
    /// subtree ([`HierarchySpec::Depth`]) or exactly one level
    /// ([`HierarchySpec::Level`]). Each item's [`KojiMeta::ancestors`] is the
    /// ordered root→parent name path (excluding its own name) and `parent_id`
    /// comes from the row. Uses a bound-param recursive CTE
    /// ([`build_descendants_sql`]); no user input is interpolated.
    ///
    /// [`KojiMeta::ancestors`]: koji_core::KojiMeta::ancestors
    #[allow(clippy::result_large_err)]
    pub async fn descendants(
        db: &DatabaseConnection,
        anchor: Anchor,
        spec: HierarchySpec,
    ) -> Result<KojiGeometryCollection, ModelError> {
        let (sql, binds) = build_descendants_sql(&anchor, &spec);
        let rows = Entity::find()
            .from_raw_sql(Statement::from_sql_and_values(
                DbBackend::MySql,
                &sql,
                binds,
            ))
            .into_model::<DescendantRow>()
            .all(db)
            .await?;

        let rows: Vec<(Model, String)> = rows
            .into_iter()
            .map(|row| {
                let model = Model {
                    id: row.id,
                    name: row.name,
                    parent: row.parent,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                    mode: row.mode,
                    geometry: row.geometry,
                    geo_type: row.geo_type,
                    // Not selected by the descendants CTE (`build_descendants_sql`)
                    // and not needed here — this `Model` only feeds
                    // `to_koji_geometry()`, which reads `geometry` directly.
                    min_lat: None,
                    min_lng: None,
                    max_lat: None,
                    max_lng: None,
                };
                (model, row.ancestry)
            })
            .collect();

        let models: Vec<Model> = rows.iter().map(|(model, _)| model.clone()).collect();
        let parent_names = Query::parent_names(db, &models).await?;
        rows.into_iter()
            .map(|(model, ancestry)| {
                let mut kg = model.to_koji_geometry(parent_name_of(&model, &parent_names))?;
                kg.meta.ancestors = ancestors_from_ancestry(&ancestry);
                Ok(kg)
            })
            .collect::<Result<KojiGeometryCollection, ModelError>>()
    }

    /// The names of the parents of `models`, by id. One query for the parents
    /// that are not themselves in `models`.
    pub(super) async fn parent_names<C: ConnectionTrait>(
        db: &C,
        models: &[Model],
    ) -> Result<HashMap<u32, String>, DbErr> {
        let mut names: HashMap<u32, String> = models
            .iter()
            .map(|model| (model.id, model.name.clone()))
            .collect();
        let missing: Vec<u32> = models
            .iter()
            .filter_map(|model| model.parent)
            .filter(|parent| !names.contains_key(parent))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        if !missing.is_empty() {
            let found = Entity::find()
                .filter(Column::Id.is_in(missing))
                .select_only()
                .column(Column::Id)
                .column(Column::Name)
                .into_model::<NameId>()
                .all(db)
                .await?;
            names.extend(found.into_iter().map(|row| (row.id, row.name)));
        }
        Ok(names)
    }

    /// `models` as Koji-native geometry, each carrying its parent's name.
    #[allow(clippy::result_large_err)]
    pub(super) async fn koji_collection<C: ConnectionTrait>(
        db: &C,
        models: Vec<Model>,
    ) -> Result<KojiGeometryCollection, ModelError> {
        let parent_names = Query::parent_names(db, &models).await?;
        models
            .iter()
            .map(|model| model.to_koji_geometry(parent_name_of(model, &parent_names)))
            .collect()
    }

    /// Returns a single Geofence model. A numeric `id` is a primary key; anything
    /// else is a name, which errors with [`ModelError::AmbiguousGeofence`] when
    /// more than one mode shares it (see [`Query::get_one_in_mode`]).
    pub async fn get_one<C: ConnectionTrait>(db: &C, id: String) -> Result<Model, ModelError> {
        Query::get_one_in_mode(db, id, None).await
    }

    /// Like [`Query::get_one`], but a name is narrowed to `mode` when given. A
    /// numeric `id` ignores `mode`.
    pub async fn get_one_in_mode<C: ConnectionTrait>(
        db: &C,
        id: String,
        mode: Option<Mode>,
    ) -> Result<Model, ModelError> {
        if let Ok(id) = id.parse::<u32>() {
            return Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| ModelError::Geofence("Does not exist".to_string()));
        }
        Query::get_by_name(db, id, mode).await
    }

    /// Resolve a geofence by name, optionally within one mode. Errors with
    /// [`ModelError::AmbiguousGeofence`] when no mode is given and several
    /// modes share the name.
    pub async fn get_by_name<C: ConnectionTrait>(
        db: &C,
        name: String,
        mode: Option<Mode>,
    ) -> Result<Model, ModelError> {
        let mut find = Entity::find().filter(Column::Name.eq(name.as_str()));
        if let Some(mode) = mode {
            find = find.filter(Column::Mode.eq(mode));
        }
        pick_by_name(&name, find.all(db).await?)
    }

    pub async fn get_one_json(db: &DatabaseConnection, id: String) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => Ok(json!(record)),
            Err(err) => Err(err),
        }
    }

    pub async fn get_one_json_with_related(
        db: &DatabaseConnection,
        id: String,
    ) -> Result<Json, ModelError> {
        match Query::get_one(db, id).await {
            Ok(record) => {
                let mut json = json!(record);
                let json = json.as_object_mut().unwrap();
                if json.contains_key("area") {
                    json.remove("area");
                }
                json.insert(
                    "projects".to_string(),
                    json!(
                        record
                            .get_related_projects()
                            .into_model::<NameId>()
                            .all(db)
                            .await?
                            .into_iter()
                            .map(|p| p.id)
                            .collect::<Vec<u32>>()
                    ),
                );
                json.insert(
                    "dragonite_links".to_string(),
                    dragonite_link::Query::json_for_geofence(db, record.id).await?,
                );
                json.insert(
                    "routes".to_string(),
                    json!(record.get_related_routes().into_json().all(db).await?),
                );
                json.insert(
                    "properties".to_string(),
                    json!(
                        record
                            .get_related_properties()
                            .into_model::<FullPropertyModel>()
                            .all(db)
                            .await?
                            .into_iter()
                            .map(|prop| {
                                let property_id = prop.property_id;
                                let mut new_json = json!(prop.parse_db_value(&record));
                                new_json["property_id"] = property_id.into();
                                new_json
                            })
                            .collect::<Vec<Json>>()
                    ),
                );
                Ok(json!(json))
            }
            Err(err) => Err(err),
        }
    }

    /// Returns all Geofence models in the db
    pub async fn get_all(db: &DatabaseConnection) -> Result<Vec<Model>, DbErr> {
        Entity::find().all(db).await
    }

    /// Returns all Geofence models in the db
    pub async fn get_all_koji(
        db: &DatabaseConnection,
    ) -> Result<koji_core::KojiGeometryCollection, ModelError> {
        let results = Query::get_all(db).await?;
        Query::koji_collection(db, results).await
    }

    /// Fetch only the requested geofence ids and map each to Koji-native
    /// geometry, mirroring `get_all_koji` but scoped to `ids` — a 600-fence org
    /// shouldn't pay to read every row when the caller only wants a handful.
    /// An id with no matching row is silently omitted (not an error).
    pub async fn get_koji_by_ids(
        db: &DatabaseConnection,
        ids: &[u32],
    ) -> Result<koji_core::KojiGeometryCollection, ModelError> {
        let results = Entity::find()
            .filter(Column::Id.is_in(ids.iter().copied()))
            .all(db)
            .await?;
        Query::koji_collection(db, results).await
    }

    /// Fetch only the geofences whose persisted bbox (`min_lat`/`min_lng`/
    /// `max_lat`/`max_lng`, kept in sync with `geometry` by `Query::upsert`)
    /// overlaps the query box, and map each to Koji-native geometry — the
    /// DB-level sibling of [`Query::get_koji_by_ids`], scoping `?bbox=` to an
    /// indexed `WHERE` instead of an in-memory filter over every row.
    ///
    /// `bbox` is `[minLng, minLat, maxLng, maxLat]` (the geojson bbox-member
    /// order, matching [`super::geometry_bbox_lnglat`]). The overlap predicate
    /// is the standard AABB test — `min_lng <= qMaxLng AND max_lng >= qMinLng
    /// AND min_lat <= qMaxLat AND max_lat >= qMinLat` — logically identical to
    /// `bbox_overlaps` in koji-service's `/v2/geofences` handler (touching
    /// counts as overlap, via non-strict `<=`/`>=`), so the DB and the pure
    /// in-memory path agree. A row with a `NULL` bbox column (geometry-less or
    /// unparseable geometry) never satisfies these comparisons in SQL and is
    /// correctly excluded — it has no spatial extent to overlap with.
    ///
    /// `mode` further restricts to fences with that exact `Mode` when
    /// `Some`. `id_scope` further restricts to the given ids when `Some` —
    /// note an empty `Some(vec![])` is a REAL filter (yields nothing),
    /// distinct from `None` (no id filter at all); the `/v2/geofences?bbox&
    /// projects=` neighbour filter relies on this to correctly return zero
    /// rows for a project with no linked fences.
    pub async fn get_koji_by_bbox<C: ConnectionTrait>(
        db: &C,
        bbox: [f64; 4],
        mode: Option<Mode>,
        id_scope: Option<Vec<u32>>,
    ) -> Result<koji_core::KojiGeometryCollection, ModelError> {
        let [query_min_lng, query_min_lat, query_max_lng, query_max_lat] = bbox;
        let mut find = Entity::find()
            .filter(Column::MinLng.lte(query_max_lng))
            .filter(Column::MaxLng.gte(query_min_lng))
            .filter(Column::MinLat.lte(query_max_lat))
            .filter(Column::MaxLat.gte(query_min_lat));
        if let Some(m) = mode {
            find = find.filter(Column::Mode.eq(m));
        }
        // NB: an EMPTY id_scope is a real filter (project with no fences →
        // no neighbours), distinct from None (no project filter).
        if let Some(ids) = id_scope {
            find = find.filter(Column::Id.is_in(ids));
        }
        let results = find.all(db).await?;
        Query::koji_collection(db, results).await
    }

    /// Fetch one geofence row by id and map it via `to_koji_geometry`. Returns
    /// `ModelError` to match the sibling Koji read methods.
    #[allow(clippy::result_large_err)]
    pub async fn get_one_koji(
        db: &DatabaseConnection,
        id: String,
    ) -> Result<koji_core::KojiGeometry, ModelError> {
        Query::get_one_koji_in_mode(db, id, None).await
    }

    /// [`Query::get_one_koji`] with a name narrowed to `mode` when given.
    #[allow(clippy::result_large_err)]
    pub async fn get_one_koji_in_mode(
        db: &DatabaseConnection,
        id: String,
        mode: Option<Mode>,
    ) -> Result<koji_core::KojiGeometry, ModelError> {
        let result = Query::get_one_in_mode(db, id, mode).await?;
        let parent_names = Query::parent_names(db, std::slice::from_ref(&result)).await?;
        result.to_koji_geometry(parent_name_of(&result, &parent_names))
    }

    pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
        Entity::find()
            .filter(Column::Name.like(format!("%{}%", search).as_str()))
            .into_json()
            .all(db)
            .await
    }
}

/// Pick the one geofence a name resolves to from every row carrying that name.
/// Zero rows is "Does not exist"; two or more (necessarily distinct modes, as
/// `(name, mode)` is unique) is [`ModelError::AmbiguousGeofence`] naming them.
#[allow(clippy::result_large_err)]
pub(super) fn pick_by_name(name: &str, mut rows: Vec<Model>) -> Result<Model, ModelError> {
    match rows.len() {
        0 => Err(ModelError::Geofence("Does not exist".to_string())),
        1 => Ok(rows.remove(0)),
        _ => {
            let mut modes: Vec<String> = rows.iter().map(|m| m.mode.to_value()).collect();
            modes.sort();
            Err(ModelError::AmbiguousGeofence {
                name: name.to_string(),
                modes,
            })
        }
    }
}

/// The name of `model`'s parent, when it has one and `names` knows it.
fn parent_name_of<'a>(model: &Model, names: &'a HashMap<u32, String>) -> Option<&'a str> {
    model
        .parent
        .and_then(|parent| names.get(&parent))
        .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u32, name: &str, mode: Mode) -> Model {
        Model {
            id,
            name: name.to_string(),
            parent: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            mode,
            geometry: serde_json::json!(null),
            geo_type: String::new(),
            min_lat: None,
            min_lng: None,
            max_lat: None,
            max_lng: None,
        }
    }

    #[test]
    fn pick_by_name_with_no_rows_does_not_exist() {
        let err = pick_by_name("Boston", vec![]).unwrap_err();
        assert!(matches!(err, ModelError::Geofence(m) if m == "Does not exist"));
    }

    #[test]
    fn pick_by_name_with_one_row_returns_it() {
        let got = pick_by_name("Boston", vec![row(3, "Boston", Mode::Pokemon)]).unwrap();
        assert_eq!(got.id, 3);
    }

    #[test]
    fn pick_by_name_with_two_modes_names_both_modes() {
        let rows = vec![
            row(1, "Boston", Mode::Unset),
            row(2, "Boston", Mode::Pokemon),
        ];
        let err = pick_by_name("Boston", rows).unwrap_err();
        match &err {
            ModelError::AmbiguousGeofence { name, modes } => {
                assert_eq!(name, "Boston");
                assert_eq!(modes, &["pokemon", "unset"]);
            }
            other => panic!("expected AmbiguousGeofence, got {other:?}"),
        }
        assert_eq!(
            err.to_string(),
            "[GEOFENCE]: `Boston` is ambiguous, it exists in modes: pokemon, unset"
        );
    }
}
