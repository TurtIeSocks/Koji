use super::*;

use entity::geofence as geoEntity;
use migration::Order;
use models::api::{Args, ArgsUnwrapped};
use serde::Deserialize;
use serde_json::json;

use crate::models::api::Response;
use crate::models::KojiDb;
use crate::queries::geofence;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: usize,
    pub per_page: usize,
    pub sort_by: String,
    pub order: String,
}

#[get("/geofence")]
async fn get_all(
    conn: web::Data<KojiDb>,
    url: web::Query<AdminReq>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner();

    let geofences = geofence::Query::paginate(
        &conn.koji_db,
        url.page,
        url.per_page,
        match url.order.to_lowercase() {
            _ => geoEntity::Column::Name,
        },
        if url.order.to_lowercase().eq("asc") {
            Order::Asc
        } else {
            Order::Desc
        },
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(geofences)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/geofence/{id}")]
async fn get_one(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let geofence = geofence::Query::get_one(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(geofence)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/geofence/{id}")]
async fn post_geofence(
    conn: web::Data<KojiDb>,
    // id: actix_web::web::Path<u32>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    // let id = id.into_inner();
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("bootstrap"));

    let (inserted, updated) = geofence::save(&conn.koji_db, area)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updated": updated, "inserted": inserted })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[patch("/geofence/{id}")]
async fn patch_geofence(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
    payload: web::Json<geoEntity::Model>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let updated_geofence = payload.into_inner();

    let result = geofence::Query::update(&conn.koji_db, id, updated_geofence)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[delete("/geofence/{id}")]
async fn delete_geofence(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let geofences = geofence::Query::delete(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(geofences.rows_affected)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
