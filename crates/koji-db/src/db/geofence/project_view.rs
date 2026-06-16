//! Project-projection read paths for the geofence entity: fetch the geofences
//! related to a project (as json / as property-aware `Feature`s / as a
//! `KojiGeometryCollection`), the parent→children walk, and the distinct-parent
//! listing. Split out of the former `geofence.rs` god-file as a **pure
//! relocation** — `impl super::Query`, no logic change.

use std::collections::HashMap;

use koji_core::UnknownId;

use crate::error::ModelError;

use super::*;

impl Query {
    /// Returns all geofence models, as models, that are related to the specified project
    pub async fn by_project(
        db: &DatabaseConnection,
        project_name: String,
    ) -> Result<Vec<Json>, DbErr> {
        Entity::find()
            .order_by(Column::Name, Order::Asc)
            .left_join(project::Entity)
            .filter(match project_name.parse::<u32>() {
                Ok(id) => project::Column::Id.eq(id),
                Err(_) => project::Column::Name.eq(project_name),
            })
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .column(Column::Mode)
            .column(Column::Parent)
            .into_json()
            .all(db)
            .await
    }

    async fn get_helper_maps(
        db: &DatabaseConnection,
        models: &Vec<&Model>,
    ) -> Result<(HashMap<u32, Vec<FullPropertyModel>>, HashMap<u32, String>), ModelError> {
        let mut ids = vec![];

        models.iter().for_each(|item| {
            ids.push(item.id);
            if let Some(parent_id) = item.parent {
                ids.push(parent_id);
            }
        });

        let mut property_map = HashMap::<u32, Vec<FullPropertyModel>>::new();

        let mut name_map: HashMap<u32, String> = Entity::find()
            .filter(Column::Id.is_in(ids.clone()))
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .into_model::<NameId>()
            .all(db)
            .await?
            .into_iter()
            .map(|model| (model.id, model.name))
            .collect();

        geofence_property::Entity::find()
            .filter(geofence_property::Column::GeofenceId.is_in(ids))
            .join(
                sea_orm::JoinType::Join,
                geofence_property::Relation::Property.def(),
            )
            .select_only()
            .column(geofence_property::Column::Id)
            .column(geofence_property::Column::PropertyId)
            .column(geofence_property::Column::GeofenceId)
            .column(geofence_property::Column::Value)
            .column(property::Column::Name)
            .column(property::Column::Category)
            .into_model::<FullPropertyModel>()
            .all(db)
            .await?
            .into_iter()
            .for_each(|prop| {
                if prop.name == "name"
                    && let Some(manual_name) = prop.value.as_ref()
                {
                    name_map.insert(prop.geofence_id, manual_name.clone());
                }
                property_map.entry(prop.geofence_id).or_default().push(prop);
            });
        Ok((property_map, name_map))
    }

    /// Returns all geofence models, as features, that are related to the specified project
    pub async fn project_as_feature(
        db: &DatabaseConnection,
        project_name: String,
        args: &ApiQueryArgs,
    ) -> Result<Vec<Feature>, ModelError> {
        let time = Instant::now();

        let items = Entity::find()
            .order_by(Column::Name, Order::Asc)
            .filter(match project_name.parse::<u32>() {
                Ok(id) => project::Column::Id.eq(id),
                Err(_) => project::Column::Name.eq(project_name),
            })
            .left_join(project::Entity)
            .all(db)
            .await?;

        let (property_map, name_map) = Query::get_helper_maps(db, &items.iter().collect()).await?;

        log::debug!("db query took {:?}", time.elapsed());

        // Resolve the render spec ONCE for the whole request — it parses the
        // comma-separated filter lists, so building it per geofence would be a
        // perf regression (and the legacy `to_feature` re-split them every row).
        let spec = args.feature_render_spec();

        let time = Instant::now();
        let items = items
            .into_iter()
            .filter_map(|result| result.to_feature(&property_map, &name_map, &spec).ok())
            .collect();

        log::debug!("feature conversion took {:?}", time.elapsed());
        Ok(items)
    }

    /// Additive Phase 2 counterpart to `project_as_feature`: fetch the SAME
    /// project-filtered geofence rows and return them as a
    /// `KojiGeometryCollection`, byte-identical to what the v1 endpoint produced
    /// pre-S5b.1. The geofence `to_feature` is heavily property-aware — custom DB
    /// properties plus the arg-driven `name`/`id`/`mode`/`group`/`parent` (and
    /// `__`-prefixed internal keys) — none of which the bare `to_koji_geometry`
    /// reproduces. So we go through the per-row `Feature`s + Phase 1 `TryFrom`
    /// (the exact path the endpoint used) rather than `to_koji_geometry`. Polygon
    /// rings are closed via `EnsurePoints` as the old `to_collection` did.
    ///
    /// NOTE: when the request asks for `?mode` on the non-internal path, the
    /// emitted `mode` property is the legacy 12-value string. As of S5c,
    /// `KojiMeta`'s `mode` field deserializes leniently (mapping the legacy value
    /// to the canonical `Mode` via `Mode::from_legacy`), so the `TryFrom` no
    /// longer fails and the full properties object is preserved. Does not replace
    /// `project_as_feature` (deleted in a later section).
    #[allow(clippy::result_large_err)]
    pub async fn project_as_koji(
        db: &DatabaseConnection,
        project_name: String,
        args: &ApiQueryArgs,
    ) -> Result<KojiGeometryCollection, ModelError> {
        let features = Query::project_as_feature(db, project_name, args).await?;
        let fc = FeatureCollection {
            bbox: None,
            features: features
                .into_iter()
                .map(EnsurePoints::ensure_first_last)
                .collect(),
            foreign_members: None,
        };
        KojiGeometryCollection::try_from(fc)
            .map_err(|e| ModelError::Custom(format!("[GEOMETRY]: {e}")))
    }

    /// Fetch a parent's *direct* children (`Column::Parent.eq`, one level down)
    /// and map each `Model` to a `KojiGeometry` via `to_koji_geometry`, collecting
    /// into a `KojiGeometryCollection`. Parent resolution: a numeric id is used
    /// directly; a name is looked up, and a missing name is
    /// `ModelError::Geofence("Parent not found")` (error-vs-empty behavior the
    /// calculate pipeline relies on). Returns `ModelError` to match
    /// `to_koji_geometry` and the sibling Koji read methods.
    #[allow(clippy::result_large_err)]
    pub async fn by_parent_koji(
        db: &DatabaseConnection,
        parent: &UnknownId,
    ) -> Result<KojiGeometryCollection, ModelError> {
        let parent_id = match parent {
            UnknownId::Number(id) => *id,
            UnknownId::String(name) => {
                let parent = Entity::find().filter(Column::Name.eq(name)).one(db).await?;
                if let Some(parent) = parent {
                    parent.id
                } else {
                    return Err(ModelError::Geofence("Parent not found".to_string()));
                }
            }
        };
        let items = Entity::find()
            .filter(Column::Parent.eq(parent_id))
            .all(db)
            .await?;

        items
            .iter()
            .map(|item| item.to_koji_geometry())
            .collect::<Result<KojiGeometryCollection, ModelError>>()
    }

    pub async fn unique_parents(db: &DatabaseConnection) -> Result<Vec<Json>, ModelError> {
        let items = Entity::find()
            .filter(Column::Parent.is_not_null())
            .select_only()
            .column(Column::Parent)
            .distinct()
            .into_model::<OnlyParent>()
            .all(db)
            .await?;
        let items = Entity::find()
            .order_by(Column::Name, Order::Asc)
            .filter(Column::Id.is_in(items.into_iter().filter_map(|item| item.parent)))
            .select_only()
            .column(Column::Id)
            .column(Column::Name)
            .distinct()
            .into_json()
            .all(db)
            .await?;
        Ok(items)
    }
}
