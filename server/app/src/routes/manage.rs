use super::*;

use entity::geofence as geoEntity;
use migration::Order;
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
async fn get_geofence(
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

#[post("/geofence")]
async fn post_geofence(
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

#[patch("/geofence")]
async fn patch_geofence(
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

#[derive(Debug, Deserialize)]
struct GeofenceId {
    id: u32,
}

#[delete("/geofence")]
async fn delete_geofence(
    conn: web::Data<KojiDb>,
    url: web::Query<GeofenceId>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner();

    let geofences = geofence::Query::delete(&conn.koji_db, url.id)
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
