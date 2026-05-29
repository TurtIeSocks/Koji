//! v2 HTTP endpoints: the generic job API (`/api/v2/jobs`), the calc sugar
//! (`/api/v2/calc/*`, sync-bridged per job-queue spec §7), and the algorithm
//! metadata endpoint. All responses use the [`JSend`](crate::utils::jsend::JSend)
//! envelope.

use std::time::Duration;

use actix_web::{
    delete, get, http::StatusCode, post, web, Error, HttpResponse,
};
use koji_db::{KojiDb, ScannerType};
use koji_jobs::{dedup_key, AwaitError, EnqueueError, JobId, JobOutcome, JobQueue};
use model::api::args::{Args, ArgsUnwrapped};
use serde::Deserialize;
use serde_json::json;

use crate::public::v2::calc::{CalcPayload, CALC_KIND};
use crate::utils::{self, jsend::JSend};

/// Priority for sync-bridged calc jobs (`priority DESC` claim order; spec §6/§8:
/// sync calc = HIGH, async/CLI = 0).
const PRIORITY_HIGH: i16 = 100;

/// Sync-bridge wait budget (spec §7: 290s, then 504 while the job keeps running).
const SYNC_WAIT: Duration = Duration::from_secs(290);

// ---------------------------------------------------------------------------
// Generic job API
// ---------------------------------------------------------------------------

/// Request body for `POST /api/v2/jobs`.
#[derive(Debug, Deserialize)]
pub struct EnqueueBody {
    /// The handler kind (e.g. `calculate`).
    pub kind: String,
    /// The job payload, passed opaquely to the handler.
    #[serde(default)]
    pub payload: serde_json::Value,
}

/// `POST /api/v2/jobs` — enqueue a job (no dedup) → `202 { job_id }`.
#[post("/jobs")]
async fn enqueue_job(
    jobs: web::Data<JobQueue>,
    body: web::Json<EnqueueBody>,
) -> Result<HttpResponse, Error> {
    let body = body.into_inner();
    match jobs.enqueue_or_attach(&body.kind, None, &body.payload, 0).await {
        Ok(id) => Ok(JSend::success_with_status(
            StatusCode::ACCEPTED,
            json!({ "job_id": id }),
        )),
        Err(e) => Ok(enqueue_error_response(e)),
    }
}

/// `GET /api/v2/jobs/{id}` — observe a job's status / progress / result.
#[get("/jobs/{id}")]
async fn get_job(
    jobs: web::Data<JobQueue>,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = match path.into_inner().parse::<JobId>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(JSend::fail(
                StatusCode::BAD_REQUEST,
                json!({ "id": "not a valid job id" }),
            ))
        }
    };
    match jobs.get(id).await {
        Ok(record) => Ok(JSend::success(record)),
        Err(AwaitError::NotFound) => Ok(JSend::error(
            StatusCode::NOT_FOUND,
            "job not found",
            Some("not_found".to_string()),
            None,
        )),
        Err(e) => Ok(JSend::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
            Some("internal_error".to_string()),
            None,
        )),
    }
}

/// `DELETE /api/v2/jobs/{id}` — request cancellation (honored at the next phase
/// boundary for a running job; a queued job is canceled immediately).
#[delete("/jobs/{id}")]
async fn cancel_job(
    jobs: web::Data<JobQueue>,
    path: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = match path.into_inner().parse::<JobId>() {
        Ok(id) => id,
        Err(_) => {
            return Ok(JSend::fail(
                StatusCode::BAD_REQUEST,
                json!({ "id": "not a valid job id" }),
            ))
        }
    };
    match jobs.cancel(id).await {
        Ok(()) => Ok(JSend::success(json!({ "canceled": id }))),
        Err(AwaitError::NotFound) => Ok(JSend::error(
            StatusCode::NOT_FOUND,
            "job not found",
            Some("not_found".to_string()),
            None,
        )),
        Err(e) => Ok(JSend::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
            Some("internal_error".to_string()),
            None,
        )),
    }
}

// ---------------------------------------------------------------------------
// Calc sugar (sync-bridged)
// ---------------------------------------------------------------------------

/// Query for `POST /api/v2/calc/*`: `?wait=1` runs the sync bridge (block up to
/// 290s for the result); otherwise the call is async (returns `202 { job_id }`).
#[derive(Debug, Deserialize)]
pub struct CalcQuery {
    /// When truthy, block for the result via the sync bridge (spec §7).
    #[serde(default)]
    pub wait: Option<u8>,
}

/// `POST /api/v2/calc/{mode}` — calc with the default `pokestop` category.
#[post("/calc/{mode}")]
async fn calc_mode(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    path: web::Path<String>,
    query: web::Query<CalcQuery>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let mode = path.into_inner();
    run_calc(conn, jobs, mode, "pokestop".to_string(), query.into_inner(), body).await
}

/// `POST /api/v2/calc/{mode}/{category}` — calc for an explicit data category.
#[post("/calc/{mode}/{category}")]
async fn calc_mode_category(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    path: web::Path<(String, String)>,
    query: web::Query<CalcQuery>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let (mode, category) = path.into_inner();
    run_calc(conn, jobs, mode, category, query.into_inner(), body).await
}

/// Shared calc flow: resolve the async inputs (area + data points), build the
/// [`CalcPayload`], enqueue (HIGH, deduped), then either sync-bridge the result
/// (`?wait=1`) or return `202 { job_id }`.
async fn run_calc(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    mode: String,
    category: String,
    query: CalcQuery,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    // The raw request JSON is preserved verbatim in the payload so the handler
    // can re-parse it into `Args` (which is `Deserialize`-only — it can't be
    // serialized back, hence accepting the body as `Value` here).
    let request_json = body.into_inner();
    let args: Args = match serde_json::from_value(request_json.clone()) {
        Ok(args) => args,
        Err(e) => {
            return Ok(JSend::fail(
                StatusCode::BAD_REQUEST,
                json!({ "body": format!("invalid calc request: {e}") }),
            ))
        }
    };

    // Resolve the config knobs we need *here* to do the async resolution
    // (area + data points). `init` also applies mode-specific defaults.
    let ArgsUnwrapped {
        area,
        data_points,
        instance,
        parent,
        last_seen,
        tth,
        ..
    } = args.init(Some(&mode));

    if area.features.is_empty()
        && instance.is_empty()
        && data_points.is_empty()
        && parent.is_none()
    {
        return Ok(JSend::fail(
            StatusCode::BAD_REQUEST,
            json!({ "area": "no area, instance, data_points, or parent provided" }),
        ));
    }

    // (async) Resolve the area collection, then the data points within it.
    let area = utils::create_or_find_collection(&instance, &conn, area, &parent, &data_points)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let data_points = if mode == "bootstrap" {
        // Bootstrap consumes the area geometry directly, not point data.
        vec![]
    } else if data_points.is_empty() {
        use koji_scanner::GenericDataToVec;
        utils::points_from_area(&area, &category, &conn, last_seen, tth)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?
            .to_single_vec()
    } else {
        data_points
    };

    let calc_payload = CalcPayload {
        mode: mode.clone(),
        category,
        scanner_is_unown: conn.scanner_type == ScannerType::Unown,
        request: request_json,
        area,
        data_points,
    };

    // Content-addressed dedup so a timed-out client's retry coalesces onto the
    // running job (and a freshly-finished one serves from the grace window).
    let key = dedup_key(
        CALC_KIND,
        &serde_json::to_value(&calc_payload)
            .map_err(actix_web::error::ErrorInternalServerError)?,
    );

    let id = match jobs
        .enqueue_or_attach(CALC_KIND, Some(&key), &calc_payload, PRIORITY_HIGH)
        .await
    {
        Ok(id) => id,
        Err(e) => return Ok(enqueue_error_response(e)),
    };

    // Async path: hand back the id immediately.
    if query.wait.unwrap_or(0) == 0 {
        return Ok(JSend::success_with_status(
            StatusCode::ACCEPTED,
            json!({ "job_id": id }),
        ));
    }

    // Sync bridge: block up to 290s for the terminal outcome.
    match jobs.await_result(id, SYNC_WAIT).await {
        Ok(JobOutcome::Succeeded(result)) => Ok(JSend::success(result)),
        Ok(JobOutcome::Failed { error, code }) => {
            Ok(JSend::error(status_for_code(&code), error, Some(code), None))
        }
        Ok(JobOutcome::Canceled) => Ok(JSend::error(
            StatusCode::CONFLICT,
            "job was canceled",
            Some("canceled".to_string()),
            None,
        )),
        Err(AwaitError::Timeout) => Ok(JSend::error(
            StatusCode::GATEWAY_TIMEOUT,
            "calculation still running; retry with the job id",
            Some("timeout".to_string()),
            Some(json!({ "job_id": id })),
        )),
        Err(AwaitError::NotFound) => Ok(JSend::error(
            StatusCode::NOT_FOUND,
            "job not found",
            Some("not_found".to_string()),
            None,
        )),
        Err(e) => Ok(JSend::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            e.to_string(),
            Some("internal_error".to_string()),
            None,
        )),
    }
}

// ---------------------------------------------------------------------------
// Metadata
// ---------------------------------------------------------------------------

/// `GET /api/v2/meta/algorithms` — the available clustering / routing /
/// bootstrap modes (plugins included).
#[get("/meta/algorithms")]
async fn meta_algorithms() -> Result<HttpResponse, Error> {
    use algorithms::{bootstrap, clustering, routing};
    Ok(JSend::success(json!({
        "clustering": clustering::all_clustering_options(),
        "routing": routing::all_routing_options(),
        "bootstrap": bootstrap::all_bootstrap_options(),
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map an [`EnqueueError`] to a JSend error response.
fn enqueue_error_response(e: EnqueueError) -> HttpResponse {
    JSend::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        e.to_string(),
        Some("internal_error".to_string()),
        None,
    )
}

/// Map a stable job error code to the HTTP status the sync bridge should return.
fn status_for_code(code: &str) -> StatusCode {
    match code {
        "validation_error" => StatusCode::UNPROCESSABLE_ENTITY,
        "not_found" => StatusCode::NOT_FOUND,
        "canceled" => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
