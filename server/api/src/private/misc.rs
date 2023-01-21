use super::*;

use crate::private::admin::Search;

use actix_session::Session;
use actix_web::http::header;

use geojson::Value;
use model::api::args::{Auth, ConfigResponse, Response};
use serde_json::json;

#[get("/")]
async fn config(scanner_type: web::Data<String>, session: Session) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref().to_string();
    let start_lat: f64 = std::env::var("START_LAT")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let start_lon: f64 = std::env::var("START_LON")
        .unwrap_or("0.0".to_string())
        .parse()
        .unwrap();
    let tile_server = std::env::var("TILE_SERVER").unwrap_or("".to_string());
    Ok(HttpResponse::Ok().json(ConfigResponse {
        start_lat,
        start_lon,
        tile_server,
        scanner_type,
        logged_in: if let Ok(logged_in) = session.get::<bool>("logged_in") {
            logged_in.unwrap_or(false)
        } else {
            false
        },
        dangerous: std::env::var("DANGEROUS").is_ok(),
    }))
}

#[post("/login")]
async fn login(payload: web::Json<Auth>, session: Session) -> Result<HttpResponse, Error> {
    if payload.password == std::env::var("KOJI_SECRET").unwrap_or("".to_string()) {
        return match session.insert("logged_in", true) {
            Ok(_) => Ok(HttpResponse::Ok().finish()),
            Err(err) => {
                println!("[API] Error logging in: {:?}", err);
                Ok(HttpResponse::Unauthorized().finish())
            }
        };
    }
    Ok(HttpResponse::Unauthorized().finish())
}

#[get("/logout")]
async fn logout(session: Session) -> Result<HttpResponse, Error> {
    session.clear();
    Ok(HttpResponse::Found()
        .append_header((header::LOCATION, "/"))
        .finish())
}

#[get("/nominatim")]
async fn search_nominatim(
    nominatim_client: web::Data<nominatim::Client>,
    url: web::Query<Search>,
) -> Result<HttpResponse, Error> {
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
            err
        })
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let results: FeatureCollection = results
        .into_iter()
        .filter_map(|feat| {
            if let Some(geometry) = feat.geometry.as_ref() {
                match geometry.value {
                    Value::Polygon(_) | Value::MultiPolygon(_) => return Some(feat),
                    _ => {
                        if let Some(id) = feat.property("osm_id") {
                            if let Some(id) = id.as_u64() {
                                log::info!("[NOMINATIM] Filtered OSM ID: {} | Not a Polygon or MultiPolygon", id)
                            }
                        }
                        return None;
                    }
                }
            }
            None
        })
        .collect();
    log::info!("[NOMINATIM] Results Found: {}", results.features.len());

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
