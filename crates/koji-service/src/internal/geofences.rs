//! Bespoke `GET /internal/geofences` — paginated flat rows for the shadmin
//! DataTable. Reuses the public geofence list DB path
//! (`geofence::Query::paginate` / `AdminReqParsed`) — only the serialization
//! differs (rows, not a GeoJSON FeatureCollection). Contract §2 `GeofenceRow`.
use actix_web::{HttpResponse, web};
use koji_db::{KojiDb, query_args::AdminReqParsed};
use serde::{Deserialize, Serialize};

use crate::utils::api_response::{ApiResponse, Meta};
use crate::utils::error::ServiceError;

/// `?page&per_page&sortBy|sort_by&order&q` + filters `project/parent/geotype/mode`.
#[derive(Debug, Deserialize)]
pub(crate) struct RowQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    #[serde(alias = "sortBy")]
    sort_by: Option<String>,
    order: Option<String>,
    q: Option<String>,
    project: Option<u32>,
    parent: Option<u32>,
    geotype: Option<String>,
    mode: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct GeofenceRow {
    id: i64,
    name: String,
    mode: String,
    parent: Option<i64>,
    geo_type: String,
    projects: Vec<i64>,
    property_count: usize,
}

pub(crate) async fn list_rows(
    conn: web::Data<KojiDb>,
    query: web::Query<RowQuery>,
) -> Result<HttpResponse, ServiceError> {
    let paging = crate::utils::pagination::Pagination::from_parts(query.page, query.per_page);
    let (page, per_page) = (paging.page(), paging.per_page());
    let args = AdminReqParsed {
        page: (page - 1) as u64, // koji-db paginate is 0-based
        per_page: per_page as u64,
        sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
        order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
        q: query.q.clone().unwrap_or_default(),
        geotype: query.geotype.clone(),
        project: query.project,
        mode: query.mode.clone(),
        parent: query.parent,
        geofenceid: None,
        pointsmin: None,
        pointsmax: None,
    };
    let (rows, total, has_next, has_prev) =
        koji_db::db::geofence::Query::paginate(&conn.koji, args)
            .await?
            .into_parts();

    // `paginate` emits JSON objects with fields:
    //   id, name, mode, geo_type, parent,
    //   projects:[{…}], properties:[{…}], routes:[{…}]
    // Reshape to contract GeofenceRow: projects → id list; properties → count.
    let data: Vec<GeofenceRow> = rows
        .into_iter()
        .map(|r| {
            let projects = r
                .get("projects")
                .and_then(|p| p.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|p| p.get("id").and_then(serde_json::Value::as_i64))
                        .collect()
                })
                .unwrap_or_default();
            let property_count = r
                .get("properties")
                .and_then(|p| p.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            GeofenceRow {
                id: r.get("id").and_then(serde_json::Value::as_i64).unwrap_or(0),
                name: r
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                mode: r
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                parent: r.get("parent").and_then(serde_json::Value::as_i64),
                geo_type: r
                    .get("geo_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                projects,
                property_count,
            }
        })
        .collect();

    // Meta::build owns the ceiling math; the DB paginator's has_next/has_prev
    // are authoritative, so they override the derived flags.
    Ok(ApiResponse::success_paginated(
        data,
        Meta {
            has_next,
            has_prev,
            ..Meta::build(total as i64, page, per_page)
        },
    ))
}
