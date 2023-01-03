use super::*;

use migration::Order;
use serde_json::json;

use crate::model::{api::args::Response, db::project, KojiDb};

#[get("/project")]
async fn paginate(
    conn: web::Data<KojiDb>,
    url: web::Query<AdminReq>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner().parse();

    let mut projects = project::Query::paginate(
        &conn.koji_db,
        url.page,
        url.per_page,
        match url.sort_by.to_lowercase().as_str() {
            "id" => project::Column::Id,
            _ => project::Column::Name,
        },
        if url.order.to_lowercase().eq("asc") {
            Order::Asc
        } else {
            Order::Desc
        },
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    // ghetto sort
    if url.sort_by == "related.length" {
        projects.results.sort_by(|a, b| {
            if url.order == "ASC" {
                a.1.len().cmp(&b.1.len())
            } else {
                b.1.len().cmp(&a.1.len())
            }
        })
    }
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(projects)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/project/all")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let projects = project::Query::get_all(&conn.koji_db)
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

#[get("/project/{id}")]
async fn get_one(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let project = project::Query::get_one(&conn.koji_db, id)
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

#[post("/project")]
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<project::Model>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = project::Query::create(&conn.koji_db, payload)
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

#[patch("/project/{id}")]
async fn update(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
    payload: web::Json<project::Model>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let updated_project = payload.into_inner();

    let result = project::Query::update(&conn.koji_db, id, updated_project)
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

#[delete("/project/{id}")]
async fn remove(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let projects = project::Query::delete(&conn.koji_db, id)
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
