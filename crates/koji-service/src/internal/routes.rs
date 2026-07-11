//! Bespoke `GET /internal/routes` — paginated flat rows for the shadmin
//! DataTable. Reuses `route::Query::paginate` (koji-db) — serialization only.
use actix_web::{HttpResponse, web};
use koji_db::{KojiDb, query_args::AdminReqParsed};
use serde::{Deserialize, Serialize};

use crate::utils::api_response::{ApiResponse, Meta};
use crate::utils::error::ServiceError;

/// `?page&per_page&sortBy|sort_by&order&q&mode&geofenceid&pointsmin&pointsmax`
#[derive(Debug, Deserialize)]
pub(crate) struct RowQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    #[serde(alias = "sortBy")]
    sort_by: Option<String>,
    order: Option<String>,
    q: Option<String>,
    mode: Option<String>,
    geofenceid: Option<u32>,
    pointsmin: Option<u32>,
    pointsmax: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RouteRow {
    id: i64,
    name: String,
    description: Option<String>,
    mode: String,
    geofence_id: i64,
    points: usize,
}

pub(crate) async fn list_rows(
    conn: web::Data<KojiDb>,
    query: web::Query<RowQuery>,
) -> Result<HttpResponse, ServiceError> {
    let paging = crate::utils::pagination::Pagination::from_parts(query.page, query.per_page);
    let (page, per_page) = (paging.page(), paging.per_page());
    let args = AdminReqParsed {
        page: (page - 1) as u64,
        per_page: per_page as u64,
        sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
        order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
        q: query.q.clone().unwrap_or_default(),
        geofenceid: query.geofenceid,
        mode: query.mode.clone(),
        pointsmin: query.pointsmin,
        pointsmax: query.pointsmax,
        // Unused by route::Query::paginate
        geotype: None,
        project: None,
        parent: None,
    };

    let (rows, total, has_next, has_prev) = koji_db::db::route::Query::paginate(&conn.koji, args)
        .await?
        .into_parts();

    let data: Vec<RouteRow> = rows
        .into_iter()
        .map(|r| RouteRow {
            id: r.get("id").and_then(serde_json::Value::as_i64).unwrap_or(0),
            name: r
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            description: r.get("description").and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    v.as_str().map(str::to_string)
                }
            }),
            mode: r
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("unset")
                .to_string(),
            geofence_id: r
                .get("geofence_id")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            points: r
                .get("points")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0) as usize,
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
