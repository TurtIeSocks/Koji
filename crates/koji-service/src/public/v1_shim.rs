//! `/api/v1` calc shim, so an unchanged Dragonite can use Koji v2.
//!
//! v2 removed `/api/v1`, but Dragonite still calls `{koji_url}/api/v1/<endpoint>`
//! with a Bearer token. This serves the four routes it uses (`GET /health`,
//! `POST /calc/bootstrap`, `POST /calc/route/{category}`), reads the flat v1
//! request body, runs the v2 calc inline (the same path as `POST /jobs` with
//! `sync: true`, no timeout of its own) and answers in v1's
//! `{message,status,status_code,data,stats}` envelope.
//!
//! A failed calculation is always a non-200 with `data: null`: Dragonite
//! overwrites the stored route with an empty one on a 200 with null `data`.
//!
//! Persistence (`save_to_db`, ...) was not part of what Dragonite sends and is
//! not offered here.

use actix_web::{HttpRequest, HttpResponse, ResponseError, http::StatusCode, post, web};
use algorithms::clustering::{CalculationMode, ClusterMode};
use algorithms::routing::SortBy;
use geojson::FeatureCollection;
use koji_core::{KojiGeometryCollection, Precision};
use koji_db::KojiDb;
use koji_jobs::JobQueue;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::public::v2::jobs::{compute_inline, resolve_calc_payload};
use crate::requests::{
    BootstrapArgs, BootstrapReq, CalcRequest, ClusterReq, ClusteringArgs, DataPointsArg, GeoInput,
    ReturnTypeArg, RoutingArgs, get_return_type,
};
use crate::utils::error::ServiceError;
use crate::utils::response::response_body;

/// v1's request body limit.
const BODY_LIMIT: usize = 1024 * 1024 * 50;

/// Categories `POST /calc/route/{category}` accepts.
const CATEGORIES: [&str; 5] = ["pokestop", "spawnpoint", "fort", "gym", "station"];

/// The subset of v1's flat request body that Dragonite sends. Unknown fields are
/// ignored.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct V1CalcBody {
    instance: Option<String>,
    radius: Option<Precision>,
    min_points: Option<usize>,
    data_points: Option<DataPointsArg>,
    area: Option<GeoInput>,
    return_type: Option<String>,
    calculation_mode: Option<CalculationMode>,
    s2_level: Option<u8>,
    s2_size: Option<u8>,
    cluster_mode: Option<ClusterMode>,
    center_clusters: Option<bool>,
    sort_by: Option<SortBy>,
}

impl V1CalcBody {
    fn return_type(&self) -> ReturnTypeArg {
        match &self.return_type {
            Some(rt) => get_return_type(rt, &ReturnTypeArg::SingleArray),
            None => ReturnTypeArg::SingleArray,
        }
    }

    fn routing(&self) -> RoutingArgs {
        RoutingArgs {
            sort_by: self.sort_by.clone(),
            plugin_args: None,
        }
    }

    fn into_bootstrap(self) -> CalcRequest {
        let routing = self.routing();
        CalcRequest::Bootstrap(BootstrapReq {
            area: self.area,
            bootstrap: BootstrapArgs {
                calculation_mode: self.calculation_mode,
                radius: self.radius,
                s2_level: self.s2_level,
                s2_size: self.s2_size,
                plugin_args: None,
            },
            routing,
            output: Default::default(),
            dev: Default::default(),
            parent: None,
            instance: self.instance,
        })
    }

    fn into_route(self) -> CalcRequest {
        let routing = self.routing();
        CalcRequest::Route(ClusterReq {
            area: self.area,
            data_points: self.data_points,
            clustering: ClusteringArgs {
                radius: self.radius,
                min_points: self.min_points,
                max_clusters: None,
                mode: self.cluster_mode,
                calculation_mode: self.calculation_mode,
                s2_level: self.s2_level,
                s2_size: self.s2_size,
                center_clusters: self.center_clusters,
                plugin_args: None,
            },
            routing,
            output: Default::default(),
            dev: Default::default(),
            data_filter: Default::default(),
            parent: None,
            instance: self.instance,
        })
    }
}

/// v1's response envelope.
#[derive(Debug, Serialize)]
struct V1Response {
    message: String,
    status: &'static str,
    status_code: u16,
    data: Option<Value>,
    stats: Option<Value>,
}

impl V1Response {
    fn ok(data: Value, stats: Value) -> Self {
        V1Response {
            message: "Success".to_string(),
            status: "ok",
            status_code: 200,
            data: Some(data),
            stats: Some(stats),
        }
    }
}

/// A failed request: a non-200 status with v1's envelope and `data: null`.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct V1Error(#[from] ServiceError);

impl ResponseError for V1Error {
    fn status_code(&self) -> StatusCode {
        self.0.status_code()
    }

    fn error_response(&self) -> HttpResponse {
        let (status, error) = self.0.to_api_error();
        HttpResponse::build(status).json(V1Response {
            message: error.message,
            status: "error",
            status_code: status.as_u16(),
            data: None,
            stats: None,
        })
    }
}

/// Render a `{ data, stats }` compute result as v1's success envelope.
fn envelope(result: Value, return_type: ReturnTypeArg) -> Result<HttpResponse, ServiceError> {
    let Value::Object(mut result) = result else {
        return Err(ServiceError::internal("calc result is not an object"));
    };
    let data = result.remove("data").unwrap_or(Value::Null);
    let stats = result.remove("stats").unwrap_or(Value::Null);
    if data.is_null() {
        return Err(ServiceError::internal("calc produced no data"));
    }
    let collection: FeatureCollection = serde_json::from_value(data).map_err(ServiceError::internal)?;
    let collection = KojiGeometryCollection::try_from(collection).map_err(ServiceError::internal)?;
    Ok(HttpResponse::Ok().json(V1Response::ok(response_body(&collection, return_type), stats)))
}

async fn run(
    conn: &KojiDb,
    jobs: &JobQueue,
    op: &str,
    category: String,
    body: V1CalcBody,
) -> Result<HttpResponse, V1Error> {
    let return_type = body.return_type();
    let request = if op == "bootstrap" {
        body.into_bootstrap()
    } else {
        body.into_route()
    };
    let payload = resolve_calc_payload(conn, op, category, &request).await?;
    let result = compute_inline(jobs, payload).await?;
    Ok(envelope(result, return_type)?)
}

#[post("/bootstrap")]
async fn bootstrap(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    body: web::Json<V1CalcBody>,
) -> Result<HttpResponse, V1Error> {
    run(&conn, &jobs, "bootstrap", String::new(), body.into_inner()).await
}

#[post("/route/{category}")]
async fn route(
    conn: web::Data<KojiDb>,
    jobs: web::Data<JobQueue>,
    category: web::Path<String>,
    body: web::Json<V1CalcBody>,
) -> Result<HttpResponse, V1Error> {
    let category = category.into_inner();
    if !CATEGORIES.contains(&category.as_str()) {
        return Err(ServiceError::Invalid {
            field: Some("category".to_string()),
            message: format!("unknown category {category:?}"),
        }
        .into());
    }
    run(&conn, &jobs, "route", category, body.into_inner()).await
}

/// Malformed JSON gets v1's envelope too.
fn json_error(err: actix_web::error::JsonPayloadError, _req: &HttpRequest) -> actix_web::Error {
    V1Error(ServiceError::Invalid {
        field: None,
        message: format!("invalid_request: {err}"),
    })
    .into()
}

/// Register the shim's routes (the caller supplies the `/v1` scope + auth).
pub(crate) fn services(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/health").route(web::get().to(HttpResponse::Ok)))
        .service(
            web::scope("/calc")
                .app_data(
                    web::JsonConfig::default()
                        .limit(BODY_LIMIT)
                        .error_handler(json_error),
                )
                .service(bootstrap)
                .service(route),
        );
}
