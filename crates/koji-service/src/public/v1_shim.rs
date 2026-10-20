//! `/api/v1` calc shim, so an unchanged Dragonite can use Koji v2.
//!
//! v2 removed `/api/v1`, but Dragonite still calls `{koji_url}/api/v1/<endpoint>`
//! with a Bearer token. This serves the four routes it uses (`GET /health`,
//! `POST /calc/bootstrap`, `POST /calc/route/{category}`), reads the flat v1
//! request body, runs the v2 calc inline (the same path as `POST /jobs` with
//! `sync: true`, no timeout of its own) and answers in v1's
//! `{message,status,status_code,data,stats}` envelope.
//!
//! It also serves `GET /geofence/{return_type}/{project}`, which Golbat,
//! ReactMap, Poracle and Fletchling read their fences from.
//!
//! A failed calculation is always a non-200 with `data: null`: Dragonite
//! overwrites the stored route with an empty one on a 200 with null `data`.
//!
//! Persistence (`save_to_db`, ...) was not part of what Dragonite sends and is
//! not offered here.

use actix_web::{HttpRequest, HttpResponse, ResponseError, get, http::StatusCode, post, web};
use algorithms::clustering::{CalculationMode, ClusterMode};
use algorithms::routing::SortBy;
use geojson::FeatureCollection;
use koji_core::{KojiGeometryCollection, Precision};
use koji_db::KojiDb;
use koji_db::db::geofence;
use koji_db::query_args::ApiQueryArgs;
use koji_jobs::JobQueue;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

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
    fn ok(data: Value, stats: Option<Value>) -> Self {
        V1Response {
            message: "Success".to_string(),
            status: "ok",
            status_code: 200,
            data: Some(data),
            stats,
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
///
/// A bootstrap that yields no points means the area had nothing to fill (no
/// polygon), not that the area is empty: reporting that as a 200 would have
/// Dragonite replace its route with an empty one. A route over an area with no
/// points is a legitimate empty result and stays a 200.
fn envelope(
    result: Value,
    op: &str,
    return_type: ReturnTypeArg,
) -> Result<HttpResponse, ServiceError> {
    let Value::Object(mut result) = result else {
        return Err(ServiceError::internal("calc result is not an object"));
    };
    let data = result.remove("data").unwrap_or(Value::Null);
    let stats = result.remove("stats").unwrap_or(Value::Null);
    if data.is_null() {
        return Err(ServiceError::internal("calc produced no data"));
    }
    let collection: FeatureCollection =
        serde_json::from_value(data).map_err(ServiceError::internal)?;
    let collection =
        KojiGeometryCollection::try_from(collection).map_err(ServiceError::internal)?;
    if op == "bootstrap" && collection.to_single_vec().is_empty() {
        return Err(ServiceError::Unprocessable {
            field: Some("area".to_string()),
            message: "bootstrap produced no points; the area needs a Polygon or MultiPolygon"
                .to_string(),
        });
    }
    Ok(HttpResponse::Ok().json(V1Response::ok(
        response_body(&collection, return_type),
        Some(stats),
    )))
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
    Ok(envelope(result, op, return_type)?)
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

/// The shapes `GET /geofence/{return_type}/{project}` can answer in.
///
/// Most are v2's `ReturnTypeArg`. `geometry`, `geometry-vec` and `feature-vec`
/// answer as v1 did (a bare list of geometries or features), which v2's own
/// renderer no longer has.
enum GeofenceFormat {
    Standard(ReturnTypeArg),
    Geometry,
    GeometryVec,
    FeatureVec,
}

impl GeofenceFormat {
    /// Kebab-case names only: v1's camelCase and snake_case spellings, and its
    /// silent fallback to `feature-collection`, are gone.
    fn parse(name: &str) -> Option<Self> {
        use ReturnTypeArg::*;
        Some(match name {
            "alt-text" => Self::Standard(AltText),
            "text" => Self::Standard(Text),
            "array" | "single-array" => Self::Standard(SingleArray),
            "multi-array" => Self::Standard(MultiArray),
            "struct" | "single-struct" => Self::Standard(SingleStruct),
            "multi-struct" => Self::Standard(MultiStruct),
            "feature" => Self::Standard(Feature),
            "feature-collection" => Self::Standard(FeatureCollection),
            "poracle" => Self::Standard(Poracle),
            "sql" => Self::Standard(Sql),
            "geometry" => Self::Geometry,
            "geometry-vec" | "geometries" => Self::GeometryVec,
            "feature-vec" => Self::FeatureVec,
            _ => return None,
        })
    }

    fn render(&self, collection: &KojiGeometryCollection) -> Value {
        match self {
            Self::Standard(return_type) => response_body(collection, *return_type),
            Self::FeatureVec => json!(FeatureCollection::from(collection).features),
            Self::Geometry | Self::GeometryVec => {
                let mut geometries: Vec<_> = FeatureCollection::from(collection)
                    .features
                    .into_iter()
                    .filter_map(|feature| feature.geometry)
                    .collect();
                if matches!(self, Self::Geometry) && geometries.len() == 1 {
                    json!(geometries.remove(0))
                } else {
                    json!(geometries)
                }
            }
        }
    }
}

/// `GET /geofence/{return_type}/{project}`: a project's geofences, by project
/// name or id. An unknown project is an empty result, not an error.
#[get("/{return_type}/{project}")]
async fn project_geofences(
    conn: web::Data<KojiDb>,
    path: web::Path<(String, String)>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, V1Error> {
    let (return_type, project) = path.into_inner();
    let Some(format) = GeofenceFormat::parse(&return_type) else {
        return Err(ServiceError::NotFound {
            field: "return_type",
            message: format!("unknown return type {return_type:?}"),
        }
        .into());
    };
    let collection = geofence::Query::project_as_koji(&conn.koji, project, &args)
        .await
        .map_err(ServiceError::from)?;
    Ok(HttpResponse::Ok().json(V1Response::ok(format.render(&collection), None)))
}

/// A malformed query string gets v1's envelope too.
fn query_error(err: actix_web::error::QueryPayloadError, _req: &HttpRequest) -> actix_web::Error {
    V1Error(ServiceError::Invalid {
        field: None,
        message: format!("invalid_request: {err}"),
    })
    .into()
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
        )
        .service(
            web::scope("/geofence")
                .app_data(web::QueryConfig::default().error_handler(query_error))
                .service(project_geofences),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two features `project_as_feature` hands back for a seeded project:
    /// "Paris" and its child "Paris Centre", each a closed square.
    fn collection() -> KojiGeometryCollection {
        let square = |lon: f64, lat: f64| {
            json!({
                "type": "Polygon",
                "coordinates": [[
                    [lon, lat], [lon + 0.1, lat], [lon + 0.1, lat + 0.1],
                    [lon, lat + 0.1], [lon, lat]
                ]]
            })
        };
        let fc: FeatureCollection = serde_json::from_value(json!({
            "type": "FeatureCollection",
            "features": [
                { "type": "Feature", "geometry": square(2.0, 48.0),
                  "properties": { "name": "Paris", "color": "red" } },
                { "type": "Feature", "geometry": square(2.5, 48.5),
                  "properties": { "name": "Paris Centre", "parent": "Paris" } },
            ]
        }))
        .unwrap();
        KojiGeometryCollection::try_from(fc).unwrap()
    }

    /// Just the first of `collection()`'s geofences.
    fn first_only() -> KojiGeometryCollection {
        let fc = FeatureCollection::from(&collection());
        KojiGeometryCollection::try_from(FeatureCollection {
            features: fc.features.into_iter().take(1).collect(),
            ..fc
        })
        .unwrap()
    }

    fn render(return_type: &str) -> Value {
        GeofenceFormat::parse(return_type)
            .unwrap_or_else(|| panic!("{return_type} is a return type"))
            .render(&collection())
    }

    #[test]
    fn return_types_are_kebab_case_only() {
        for known in [
            "alt-text",
            "text",
            "array",
            "single-array",
            "multi-array",
            "struct",
            "single-struct",
            "multi-struct",
            "geometry",
            "geometry-vec",
            "geometries",
            "feature",
            "feature-vec",
            "poracle",
            "feature-collection",
            "sql",
        ] {
            assert!(GeofenceFormat::parse(known).is_some(), "{known}");
        }
        for old in [
            "featureCollection",
            "featurecollection",
            "feature_collection",
            "FEATURE-COLLECTION",
            "altText",
            "alttext",
            "single_array",
            "nonsense",
            "",
        ] {
            assert!(GeofenceFormat::parse(old).is_none(), "{old}");
        }
    }

    #[test]
    fn single_array_is_every_ring_flat_as_lat_lon() {
        assert_eq!(
            render("single-array"),
            json!([
                [48.0, 2.0],
                [48.0, 2.1],
                [48.1, 2.1],
                [48.1, 2.0],
                [48.0, 2.0],
                [48.5, 2.5],
                [48.5, 2.6],
                [48.6, 2.6],
                [48.6, 2.5],
                [48.5, 2.5],
            ])
        );
        assert_eq!(render("array"), render("single-array"));
    }

    #[test]
    fn multi_array_is_one_ring_per_geofence() {
        assert_eq!(
            render("multi-array"),
            json!([
                [
                    [48.0, 2.0],
                    [48.0, 2.1],
                    [48.1, 2.1],
                    [48.1, 2.0],
                    [48.0, 2.0]
                ],
                [
                    [48.5, 2.5],
                    [48.5, 2.6],
                    [48.6, 2.6],
                    [48.6, 2.5],
                    [48.5, 2.5]
                ],
            ])
        );
    }

    #[test]
    fn struct_types_name_lat_and_lon() {
        let first = json!({ "lat": 48.0, "lon": 2.0 });
        assert_eq!(render("single-struct")[0], first);
        // The flat list is closed back to its first point, as v1 did.
        assert_eq!(render("single-struct").as_array().unwrap().len(), 11);
        assert_eq!(render("struct"), render("single-struct"));
        assert_eq!(render("multi-struct")[0][0], first);
        assert_eq!(render("multi-struct").as_array().unwrap().len(), 2);
    }

    #[test]
    fn text_types_are_strings() {
        assert_eq!(
            render("alt-text"),
            json!("48 2,48 2.1,48.1 2.1,48.1 2,48 2,48.5 2.5,48.5 2.6,48.6 2.6,48.6 2.5,48.5 2.5")
        );
        let text = render("text");
        let text = text.as_str().unwrap();
        assert!(text.starts_with("[Geofence 1]\n48,2\n48,2.1\n"), "{text}");
        assert!(text.contains("\n[Geofence 2]\n48.5,2.5\n"), "{text}");
    }

    #[test]
    fn feature_collection_keeps_properties() {
        let v = render("feature-collection");
        assert_eq!(v["type"], "FeatureCollection");
        let features = v["features"].as_array().unwrap();
        assert_eq!(features.len(), 2);
        assert_eq!(features[0]["properties"]["name"], "Paris");
        assert_eq!(features[0]["properties"]["color"], "red");
        assert_eq!(features[1]["properties"]["parent"], "Paris");
        assert_eq!(features[0]["geometry"]["type"], "Polygon");
    }

    #[test]
    fn feature_is_a_bare_feature_for_one_and_a_list_for_many() {
        let v = render("feature");
        assert_eq!(v.as_array().unwrap().len(), 2);
        assert_eq!(v[0]["type"], "Feature");
        let one = GeofenceFormat::parse("feature")
            .unwrap()
            .render(&first_only());
        assert_eq!(one["type"], "Feature");
        assert_eq!(one["properties"]["name"], "Paris");
    }

    #[test]
    fn feature_vec_is_always_a_list_of_features() {
        let v = render("feature-vec");
        assert_eq!(v.as_array().unwrap().len(), 2);
        assert_eq!(v[1]["properties"]["name"], "Paris Centre");
    }

    #[test]
    fn geometry_is_a_list_of_geometries_unless_there_is_one() {
        let v = render("geometry");
        assert_eq!(v.as_array().unwrap().len(), 2);
        assert_eq!(v[0]["type"], "Polygon");
        assert_eq!(render("geometry-vec"), v);
        assert_eq!(render("geometries"), v);
        let one = GeofenceFormat::parse("geometry")
            .unwrap()
            .render(&first_only());
        assert_eq!(one["type"], "Polygon");
        let one_vec = GeofenceFormat::parse("geometry-vec")
            .unwrap()
            .render(&first_only());
        assert_eq!(one_vec.as_array().unwrap().len(), 1);
    }

    #[test]
    fn poracle_takes_name_group_and_a_running_id() {
        let v = render("poracle");
        assert_eq!(v[0]["name"], "Paris");
        assert_eq!(v[0]["id"], 1);
        assert_eq!(v[1]["name"], "Paris Centre");
        assert_eq!(v[1]["group"], "Paris");
        assert_eq!(v[1]["id"], 2);
        assert_eq!(v[0]["path"][0], json!([48.0, 2.0]));
    }

    #[test]
    fn sql_filters_on_the_bounding_box_and_geometry() {
        let v = render("sql");
        let sql = v.as_str().unwrap();
        assert!(sql.contains("ST_CONTAINS"), "{sql}");
        assert!(sql.contains("\nOR"), "{sql}");
    }

    #[test]
    fn an_empty_project_renders_empty() {
        let empty = KojiGeometryCollection::try_from(FeatureCollection {
            bbox: None,
            features: vec![],
            foreign_members: None,
        })
        .unwrap();
        for (rt, expected) in [
            ("single-array", json!([])),
            ("multi-array", json!([])),
            ("feature", json!([])),
            ("feature-vec", json!([])),
            ("geometry", json!([])),
            ("poracle", json!([])),
        ] {
            assert_eq!(
                GeofenceFormat::parse(rt).unwrap().render(&empty),
                expected,
                "{rt}"
            );
        }
        let fc = GeofenceFormat::parse("feature-collection")
            .unwrap()
            .render(&empty);
        assert_eq!(fc["features"], json!([]));
    }
}
