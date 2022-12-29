use super::*;

use entity::geofence_project;
use serde_json::json;

use crate::models::api::Response;
use crate::models::KojiDb;

#[get("/geofence_project/all")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let items = geofence_project::Query::get_all(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(items)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/geofence_project")]
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<geofence_project::Model>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = geofence_project::Query::create(&conn.koji_db, payload)
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

#[derive(Debug, Deserialize)]
struct UpdateManyToMany {
    geofence_id: Option<u32>,
    project_id: Option<u32>,
}

#[patch("/geofence_project")]
async fn update(
    conn: web::Data<KojiDb>,
    payload: web::Json<UpdateManyToMany>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();

    let result =
        geofence_project::Query::update(&conn.koji_db, payload.geofence_id, payload.project_id)
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

#[patch("/geofence_project/{table}/{id}")]
async fn update_by_id(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<(String, u32)>,
    payload: web::Json<Vec<u32>>,
) -> Result<HttpResponse, Error> {
    let (table, id) = id.into_inner();
    let payload = payload.into_inner();

    if table == "geofence" || table == "project" {
        geofence_project::Query::update_by_id(&conn.koji_db, id, table, payload)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }

    Ok(HttpResponse::Ok().json(Response {
        data: None,
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[delete("/geofence_project")]
async fn remove(
    conn: web::Data<KojiDb>,
    payload: web::Json<UpdateManyToMany>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let projects =
        geofence_project::Query::delete(&conn.koji_db, payload.geofence_id, payload.project_id)
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
