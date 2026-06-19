//! List / cache read paths for the geofence entity (`paginate`, the admin
//! list grid, and the lightweight `get_json_cache`). Split out of the former
//! `geofence.rs` god-file as a **pure relocation** — `impl super::Query`, no
//! logic change.

use std::str::FromStr;

use super::*;

use futures::future;
use serde_json::json;

impl Query {
    pub async fn get_json_cache(db: &DatabaseConnection) -> Result<Vec<sea_orm::JsonValue>, DbErr> {
        Ok(Query::get_all_no_fences(db).await?.to_json())
    }

    /// Returns paginated Geofence models
    pub async fn paginate(
        db: &DatabaseConnection,
        args: AdminReqParsed,
    ) -> Result<PaginateResults<Vec<Json>>, DbErr> {
        let column = Column::from_str(&args.sort_by).unwrap_or(Column::Name);

        let mut paginator = Entity::find().order_by(column, parse_order(&args.order));

        if !args.q.is_empty() {
            paginator = paginator.filter(Column::Name.like(format!("%{}%", args.q).as_str()));
        }
        if let Some(parent) = args.parent {
            paginator = if parent == 0 {
                paginator.filter(Column::Parent.is_null())
            } else {
                paginator.filter(Column::Parent.eq(parent))
            }
        }
        if let Some(geo_type) = args.geotype {
            paginator = paginator.filter(Column::GeoType.eq(geo_type));
        }
        if let Some(mode) = args.mode {
            paginator = paginator.filter(Column::Mode.eq(mode));
        }
        if let Some(project_id) = args.project {
            paginator = if project_id == 0 {
                paginator
                    .left_join(geofence_project::Entity)
                    .filter(geofence_project::Column::ProjectId.is_null())
            } else {
                paginator
                    .inner_join(project::Entity)
                    .filter(project::Column::Id.eq(project_id))
            }
        }

        let paginator = paginator.paginate(db, args.per_page);

        let total = paginator.num_items_and_pages().await?;

        let results = paginator.fetch_page(args.page).await?;

        let projects = future::try_join_all(
            results
                .iter()
                .map(|result| result.get_related_projects().into_json().all(db)),
        )
        .await?;

        let properties = future::try_join_all(
            results
                .iter()
                .map(|result| result.get_related_properties().into_json().all(db)),
        )
        .await?;

        let routes = future::try_join_all(
            results
                .iter()
                .map(|result| result.get_related_routes().into_json().all(db)),
        )
        .await?;

        let mut results: Vec<Json> = results
            .into_iter()
            .enumerate()
            .map(|(i, fence)| {
                json!({
                    "id": fence.id,
                    "name": fence.name,
                    "mode": fence.mode,
                    "geo_type": fence.geo_type,
                    "parent": fence.parent,
                    // "created_at": fence.created_at,
                    // "updated_at": fence.updated_at,
                    "projects": projects[i],
                    "properties": properties[i],
                    "routes": routes[i],
                })
            })
            .collect();

        if args.sort_by.contains("length") {
            json_related_sort(
                &mut results,
                &args.sort_by.replace(".length", ""),
                args.order,
            );
        }

        Ok(PaginateResults {
            results,
            total: total.number_of_items,
            has_prev: args.page > 0,
            has_next: args.page + 1 < total.number_of_pages,
        })
    }
}
