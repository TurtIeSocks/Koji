use super::*;

use migration::Order;
use serde_json::json;

use crate::model::{api::args::Response, db::route, KojiDb};

#[get("/")]
async fn paginate(
    conn: web::Data<KojiDb>,
    url: web::Query<AdminReq>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner().parse();

    let mut routes = route::Query::paginate(
        &conn.koji_db,
        url.page,
        url.per_page,
        match url.sort_by.to_lowercase().as_str() {
            "id" => route::Column::Id,
            _ => route::Column::Name,
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
    if url.sort_by == "hops" {
        routes.results.sort_by(|a, b| {
            if url.order == "ASC" {
                a.hops.cmp(&b.hops)
            } else {
                b.hops.cmp(&a.hops)
            }
        })
    }

    // ghetto sort
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(routes)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/all/")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let projects = route::Query::get_all(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(projects)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/ref/")]
async fn get_ref(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let geofences = route::Query::get_json_cache(&conn.koji_db)
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

    let project = route::Query::get_one(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(project)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/")]
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<route::Model>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = route::Query::create(&conn.koji_db, payload)
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
    payload: web::Json<route::Model>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let updated_project = payload.into_inner();

    let result = route::Query::update(&conn.koji_db, id, updated_project)
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

    let projects = route::Query::delete(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(projects.rows_affected)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
