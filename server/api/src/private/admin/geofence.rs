use super::*;

use migration::Order;
use serde_json::json;

use crate::model::{api::args::Response, db::geofence, KojiDb};

#[get("/")]
async fn paginate(
    conn: web::Data<KojiDb>,
    url: web::Query<AdminReq>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner().parse();

    let mut geofences = geofence::Query::paginate(
        &conn.koji_db,
        url.page,
        url.per_page,
        match url.order.to_lowercase().as_str() {
            "id" => geofence::Column::Id,
            _ => geofence::Column::Name,
        },
        if url.order.to_lowercase().eq("asc") {
            Order::Asc
        } else {
            Order::Desc
        },
        url.q,
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    // ghetto sort
    if url.sort_by == "related.length" {
        geofences.results.sort_by(|a, b| {
            if url.order == "ASC" {
                a.1.len().cmp(&b.1.len())
            } else {
                b.1.len().cmp(&a.1.len())
            }
        })
    }
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(geofences)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/all/")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let geofences = geofence::Query::get_all_no_fences(&conn.koji_db)
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

#[get("/ref/")]
async fn get_ref(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let geofences = geofence::Query::get_json_cache(&conn.koji_db)
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

#[get("/{id}/")]
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

#[post("/")]
async fn create(
    conn: web::Data<KojiDb>,
    // id: actix_web::web::Path<u32>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = geofence::Query::create(&conn.koji_db, payload)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(return_payload)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[patch("/{id}/")]
async fn update(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
    payload: web::Json<serde_json::Value>,
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

#[delete("/{id}/")]
async fn remove(
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
