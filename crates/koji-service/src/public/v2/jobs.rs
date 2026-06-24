//! v2 jobs API (`/api/v2/jobs`) + the algorithm-metadata endpoint.
//!
//! `POST /jobs` is the single typed, always-async calc entry point: it takes a
//! [`CalcJobRequest`] (the op `mode` + data `category` are body fields now, not
//! URL path segments), resolves the area + data points, enqueues a HIGH-priority
//! `calculate` job with content dedup, and returns `202` + a `Location` header.
//! `GET /jobs/{id}` reads the job *record* (with `?wait=N` it long-polls up to N
//! seconds for a terminal state, then returns the record regardless — never a
//! 504; the job keeps running). `GET /jobs` lists jobs (paginated), `DELETE
//! /jobs/{id}` requests cancellation, and `GET /algorithms` lists the available
//! clustering / routing / bootstrap modes. All responses use the
//! [`ApiResponse`](crate::utils::api_response::ApiResponse) envelope; handlers
//! return `Result<HttpResponse, ServiceError>` (the typed error→status mapping).

use std::time::Duration;

use actix_web::{HttpResponse, delete, get, http::StatusCode, post, web};
use koji_db::KojiDb;
use koji_jobs::{AwaitError, JobId, JobQueue, dedup_key};
use serde::Deserialize;
use serde_json::json;

use crate::public::v2::calc::{CALC_KIND, CalcPayload};
use crate::requests::{CalcJobRequest, area_collection};
use crate::utils::error::ServiceError;
use crate::utils::pagination::Pagination;
use crate::utils::{self, api_response::ApiError, api_response::ApiResponse};

/// Priority for calc jobs (job-queue-design §4/§8: calc = HIGH).
const PRIORITY_HIGH: i16 = 100;
/// Max sync-bridge wait the `GET /jobs/{id}?wait=` long-poll honors.
const MAX_WAIT_SECS: u64 = 290;

// ---------------------------------------------------------------------------
// POST /jobs — typed, always-async
// ---------------------------------------------------------------------------

/// `POST /api/v2/jobs` — enqueue a calc job (always async) → `202 { job_id }` with
/// a `Location` header. Resolves the area + data points, builds the `CalcPayload`,
/// and enqueues HIGH with content dedup.
#[utoipa::path(
    post,
    path = "/api/v2/jobs",
    tag = "jobs",
    request_body = CalcJobRequest,
    responses(
        (status = 202, description = "Job enqueued; `Location` header points at the job record", body = Object),
        (status = 400, description = "No area/instance/dataPoints/parent provided", body = ApiError),
        (status = 500, description = "Internal error (resolve/enqueue)", body = ApiError),
    ),
)]
#[post("/jobs")]
async fn create_job(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    body: web::Json<CalcJobRequest>,
) -> Result<HttpResponse, ServiceError> {
    let body = body.into_inner();
    let op = body.op();
    let category = body.category;
    let calc_request = body.request;

    let inputs = calc_request.enqueue_inputs();
    let area = area_collection(&inputs.area);

    if area.features.is_empty()
        && inputs.instance.is_empty()
        && inputs.data_points.is_empty()
        && inputs.parent.is_none()
    {
        return Err(ServiceError::Invalid {
            field: Some("area".to_string()),
            message: "no area, instance, dataPoints, or parent provided".to_string(),
        });
    }

    // (async) resolve the area, then the data points within it.
    let area = utils::create_or_find_collection(
        &inputs.instance,
        &conn,
        area,
        &inputs.parent,
        &inputs.data_points,
    )
    .await?;

    let data_points = if op == "bootstrap" {
        vec![]
    } else if op == "reroute" {
        inputs.data_points
    } else if inputs.data_points.is_empty() {
        use koji_golbat::GenericDataToVec;
        utils::points_from_area(
            &area,
            &category,
            &conn,
            inputs.data_filter.last_seen,
            inputs.data_filter.tth,
        )
        .await?
        .to_single_vec()
    } else {
        inputs.data_points
    };

    let request = serde_json::to_value(&calc_request).map_err(ServiceError::internal)?;
    let calc_payload = CalcPayload {
        mode: op.to_string(),
        category,
        request,
        area,
        data_points,
        clusters: inputs.clusters,
    };

    let key = dedup_key(
        CALC_KIND,
        &serde_json::to_value(&calc_payload).map_err(ServiceError::internal)?,
    );
    let id = jobs
        .enqueue_or_attach(CALC_KIND, Some(&key), &calc_payload, PRIORITY_HIGH)
        .await
        .map_err(ServiceError::internal)?;

    Ok(HttpResponse::build(StatusCode::ACCEPTED)
        .insert_header(("Location", format!("/api/v2/jobs/{id}")))
        .json(serde_json::json!({ "status": "ok", "data": { "job_id": id } })))
}

// ---------------------------------------------------------------------------
// GET /jobs/{id} (+ ?wait=) — the job record
// ---------------------------------------------------------------------------

/// `?wait=N` long-polls up to N seconds (clamped to `MAX_WAIT_SECS`) for a terminal
/// state, then returns the job record regardless (running record on timeout — never
/// a 504; the job keeps running).
#[derive(Debug, Deserialize)]
struct JobWaitQuery {
    #[serde(default)]
    wait: Option<u64>,
}

/// `GET /api/v2/jobs/{id}` — the job record. With `?wait=N`, blocks up to N seconds
/// for a terminal state first.
#[utoipa::path(
    get,
    path = "/api/v2/jobs/{id}",
    tag = "jobs",
    params(
        ("id" = String, Path, description = "Job id"),
        ("wait" = Option<u64>, Query, description = "Long-poll up to N seconds for a terminal state"),
    ),
    responses(
        (status = 200, description = "The job record", body = Object),
        (status = 404, description = "No such job", body = ApiError),
    ),
)]
#[get("/jobs/{id}")]
async fn get_job(
    jobs: web::Data<JobQueue>,
    path: web::Path<String>,
    query: web::Query<JobWaitQuery>,
) -> Result<HttpResponse, ServiceError> {
    let id = path
        .into_inner()
        .parse::<JobId>()
        .map_err(|_| ServiceError::Invalid {
            field: Some("id".to_string()),
            message: "not a valid job id".to_string(),
        })?;

    if let Some(secs) = query.wait.filter(|w| *w > 0) {
        // Block for a terminal state; ignore the outcome — we return the record.
        let _ = jobs
            .await_result(id, Duration::from_secs(secs.min(MAX_WAIT_SECS)))
            .await;
    }

    match jobs.get(id).await {
        Ok(record) => Ok(ApiResponse::success(record)),
        Err(AwaitError::NotFound) => Err(ServiceError::NotFound {
            field: "job",
            message: format!("no job {id}"),
        }),
        Err(e) => Err(ServiceError::internal(e)),
    }
}

// ---------------------------------------------------------------------------
// GET /jobs (list) + DELETE /jobs/{id} (cancel)
// ---------------------------------------------------------------------------

/// `?status=` optional filter for the job list.
///
/// `page` and `per_page` are inlined as explicit fields rather than
/// `#[serde(flatten)] Pagination` because `serde_urlencoded` (used by actix
/// query extraction) does not support flattened structs — it would return 400
/// on any request that supplies `?page=` or `?per_page=`.
#[derive(Debug, Deserialize)]
struct JobListQuery {
    #[serde(default)]
    status: Option<koji_jobs::JobStatus>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    per_page: Option<i64>,
}

/// `GET /api/v2/jobs` — list jobs (newest first), paginated.
#[utoipa::path(
    get,
    path = "/api/v2/jobs",
    tag = "jobs",
    params(
        ("status" = Option<String>, Query, description = "Filter by job status"),
        ("page" = Option<i64>, Query, description = "1-based page number"),
        ("per_page" = Option<i64>, Query, description = "Page size (clamped to [1, 500])"),
    ),
    responses(
        (status = 200, description = "Paginated job records (with a `meta` block)", body = Object),
    ),
)]
#[get("/jobs")]
async fn list_jobs(
    jobs: web::Data<JobQueue>,
    query: web::Query<JobListQuery>,
) -> Result<HttpResponse, ServiceError> {
    let q = query.into_inner();
    let pagination = Pagination::from_parts(q.page, q.per_page);
    let (page, per_page) = (pagination.page(), pagination.per_page());
    let (rows, total) = jobs
        .list(page, per_page, q.status)
        .await
        .map_err(ServiceError::internal)?;
    Ok(ApiResponse::success_paginated(
        rows,
        crate::utils::api_response::Meta::build(total, page, per_page),
    ))
}

/// `DELETE /api/v2/jobs/{id}` — request cancellation.
#[utoipa::path(
    delete,
    path = "/api/v2/jobs/{id}",
    tag = "jobs",
    params(("id" = String, Path, description = "Job id")),
    responses(
        (status = 202, description = "Cancellation requested", body = Object),
        (status = 404, description = "No such job", body = ApiError),
    ),
)]
#[delete("/jobs/{id}")]
async fn cancel_job(
    jobs: web::Data<JobQueue>,
    path: web::Path<String>,
) -> Result<HttpResponse, ServiceError> {
    let id = path
        .into_inner()
        .parse::<JobId>()
        .map_err(|_| ServiceError::Invalid {
            field: Some("id".to_string()),
            message: "not a valid job id".to_string(),
        })?;
    // `JobQueue::cancel()` returns `Ok(())` even when no row is matched (it
    // runs UPDATE statements without checking affected rows).  Check existence
    // first so a missing id returns 404 instead of 202.
    match jobs.get(id).await {
        Err(AwaitError::NotFound) => {
            return Err(ServiceError::NotFound {
                field: "job",
                message: format!("no job {id}"),
            });
        }
        Err(e) => return Err(ServiceError::internal(e)),
        Ok(_) => {}
    }
    match jobs.cancel(id).await {
        Ok(()) => Ok(ApiResponse::success_with_status(
            StatusCode::ACCEPTED,
            json!({ "canceled": id }),
        )),
        Err(AwaitError::NotFound) => Err(ServiceError::NotFound {
            field: "job",
            message: format!("no job {id}"),
        }),
        Err(e) => Err(ServiceError::internal(e)),
    }
}

// ---------------------------------------------------------------------------
// GET /algorithms
// ---------------------------------------------------------------------------

/// `GET /api/v2/algorithms` — available clustering / routing / bootstrap modes
/// (plugins included). Was `/meta/algorithms`.
#[utoipa::path(
    get,
    path = "/api/v2/algorithms",
    tag = "jobs",
    responses(
        (status = 200, description = "Available clustering / routing / bootstrap modes", body = Object),
    ),
)]
#[get("/algorithms")]
async fn algorithms() -> Result<HttpResponse, ServiceError> {
    // `::algorithms` (absolute crate path) — the local handler `algorithms`
    // shadows the crate name inside this fn body, so a bare `use algorithms::…`
    // would resolve to this unit struct, not the external crate.
    use ::algorithms::{bootstrap, clustering, routing};
    Ok(ApiResponse::success(json!({
        "clustering": clustering::all_clustering_options(),
        "routing": routing::all_routing_options(),
        "bootstrap": bootstrap::all_bootstrap_options(),
    })))
}
