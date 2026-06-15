use super::*;

use serde::Deserialize;
use serde_json::json;

use koji_db::{KojiDb, db::geofence_project};

use crate::utils::response::{Response, ok_response};

#[get("/all/")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let items = geofence_project::Query::get_all(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(items)))
}

#[post("/")]
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<geofence_project::Model>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = geofence_project::Query::create(&conn.koji, payload)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(return_payload)))
}

#[derive(Debug, Deserialize)]
struct UpdateManyToMany {
    geofence_id: Option<u32>,
    project_id: Option<u32>,
}

#[patch("/")]
async fn update(
    conn: web::Data<KojiDb>,
    payload: web::Json<UpdateManyToMany>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();

    let result =
        geofence_project::Query::update(&conn.koji, payload.geofence_id, payload.project_id)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(result)))
}

#[patch("/{table}/{id}/")]
async fn update_by_id(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<(String, u32)>,
    payload: web::Json<Vec<u32>>,
) -> Result<HttpResponse, Error> {
    let (table, id) = id.into_inner();
    let payload = payload.into_inner();

    if table == "geofence" || table == "project" {
        geofence_project::Query::update_by_id(&conn.koji, id, table, payload)
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

#[delete("/")]
async fn remove(
    conn: web::Data<KojiDb>,
    payload: web::Json<UpdateManyToMany>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let projects =
        geofence_project::Query::delete(&conn.koji, payload.geofence_id, payload.project_id)
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(projects.rows_affected)))
}
