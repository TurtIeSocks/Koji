//! Read paths for the geofence entity (single/all fetches, the recursive
//! descendants walk, and `search`). Split out of the former `geofence.rs`
//! god-file as a **pure relocation** — `impl super::Query`, no logic change.

use koji_core::KojiGeometryCollection;

use crate::error::ModelError;

use super::*;

use serde_json::json;

impl Query {
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

        rows.into_iter()
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
                    dragonite_area_id: row.dragonite_area_id,
                    // Not selected by the descendants CTE (`build_descendants_sql`)
                    // and not needed here — this `Model` only feeds
                    // `to_koji_geometry()`, which reads `geometry` directly.
                    min_lat: None,
                    min_lng: None,
                    max_lat: None,
                    max_lng: None,
                };
                let mut kg = model.to_koji_geometry()?;
                kg.meta.ancestors = ancestors_from_ancestry(&row.ancestry);
                Ok(kg)
            })
            .collect::<Result<KojiGeometryCollection, ModelError>>()
    }

    /// Returns a single Geofence model and it's related projects as tuple
    pub async fn get_one<C: ConnectionTrait>(db: &C, id: String) -> Result<Model, ModelError> {
        let record = match id.parse::<u32>() {
            Ok(id) => Entity::find_by_id(id).one(db).await?,
            Err(_) => Entity::find().filter(Column::Name.eq(id)).one(db).await?,
        };
        if let Some(record) = record {
            Ok(record)
        } else {
            Err(ModelError::Geofence("Does not exist".to_string()))
        }
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
        results
            .iter()
            .map(|result| result.to_koji_geometry())
            .collect::<Result<koji_core::KojiGeometryCollection, ModelError>>()
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
        results
            .iter()
            .map(|result| result.to_koji_geometry())
            .collect::<Result<koji_core::KojiGeometryCollection, ModelError>>()
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
        results
            .iter()
            .map(|result| result.to_koji_geometry())
            .collect::<Result<koji_core::KojiGeometryCollection, ModelError>>()
    }

    /// Fetch one geofence row by id and map it via `to_koji_geometry`. Returns
    /// `ModelError` to match the sibling Koji read methods.
    #[allow(clippy::result_large_err)]
    pub async fn get_one_koji(
        db: &DatabaseConnection,
        id: String,
    ) -> Result<koji_core::KojiGeometry, ModelError> {
        let result = Query::get_one(db, id).await?;
        result.to_koji_geometry()
    }

    pub async fn search(db: &DatabaseConnection, search: String) -> Result<Vec<Json>, DbErr> {
        Entity::find()
            .filter(Column::Name.like(format!("%{}%", search).as_str()))
            .into_json()
            .all(db)
            .await
    }
}
