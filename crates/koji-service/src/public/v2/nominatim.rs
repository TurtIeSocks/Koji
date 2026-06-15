//! v2 Nominatim proxy — `GET /api/v2/nominatim?query=…`.
//!
//! Ports the legacy `/config/nominatim` (`private::misc::search_nominatim`) onto
//! the v2 surface: a thin proxy over the configured Nominatim instance that
//! geocodes the `?query=` and returns only the `Polygon` / `MultiPolygon`
//! results (the only shapes the import dialog can use), wrapped in the
//! [`ApiResponse`] envelope. Upstream failures funnel through
//! [`ServiceError::internal`] (logged; generic 500 to the client).

use actix_web::{HttpResponse, get, web};
use geojson::{FeatureCollection, Value};
use serde::Deserialize;
use serde_json::json;

use crate::utils::api_response::{ApiError, ApiResponse};
use crate::utils::error::ServiceError;

/// The `?query=` parameter for the Nominatim search.
#[derive(Debug, Deserialize)]
struct NominatimQuery {
    query: String,
}

/// `GET /api/v2/nominatim?query=…` — geocode `query` via the configured
/// Nominatim instance, keeping only `Polygon` / `MultiPolygon` features, and
/// return the resulting `FeatureCollection` in the [`ApiResponse`] envelope.
#[utoipa::path(
    get,
    path = "/api/v2/nominatim",
    tag = "nominatim",
    params(("query" = String, Query, description = "Free-text place query to geocode")),
    responses(
        (status = 200, description = "Polygon/MultiPolygon geocoding results as a FeatureCollection", body = Object),
        (status = 500, description = "Upstream Nominatim failure", body = ApiError),
    ),
)]
#[allow(clippy::result_large_err)]
#[get("/nominatim")]
pub(crate) async fn search_nominatim(
    nominatim_client: web::Data<nominatim::Client>,
    url: web::Query<NominatimQuery>,
) -> Result<HttpResponse, ServiceError> {
    let query = url.into_inner();
    log::debug!("[NOMINATIM] Search: \"{}\"", query.query);

    let results = nominatim_client
        .search(
            nominatim::SearchQueryBuilder::default()
                .address_details(true)
                .location_query(nominatim::LocationQuery::Generalised { q: query.query })
                .dedupe(true)
                .limit(Some(50))
                .build()
                .unwrap(),
        )
        .await
        .map_err(|err| {
            log::error!("[NOMINATIM] {:?}", err);
            ServiceError::internal(err)
        })?;

    let results: FeatureCollection = results
        .into_iter()
        .filter_map(|feat| {
            if let Some(geometry) = feat.geometry.as_ref() {
                match geometry.value {
                    Value::Polygon(_) | Value::MultiPolygon(_) => return Some(feat),
                    _ => {
                        if let Some(id) = feat.property("osm_id")
                            && let Some(id) = id.as_u64()
                        {
                            log::info!(
                                "[NOMINATIM] Filtered OSM ID: {} | Not a Polygon or MultiPolygon",
                                id
                            )
                        }
                        return None;
                    }
                }
            }
            None
        })
        .collect();
    log::info!("[NOMINATIM] Results Found: {}", results.features.len());

    Ok(ApiResponse::success(json!(results)))
}
