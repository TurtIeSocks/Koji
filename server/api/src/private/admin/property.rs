use super::*;

use serde_json::json;

use crate::model::{api::args::Response, db::property, KojiDb};

#[get("/")]
async fn paginate(
    conn: web::Data<KojiDb>,
    url: web::Query<AdminReq>,
) -> Result<HttpResponse, Error> {
    let url = url.into_inner().parse();

    let properties = property::Query::paginate(
        &conn.koji_db,
        url.page,
        url.per_page,
        url.order,
        url.sort_by,
        url.q,
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(properties)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/all/")]
async fn get_all(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let properties = property::Query::get_all(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(properties)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/ref/")]
async fn get_ref(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let properties = property::Query::get_json_cache(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(properties)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/{id}/")]
async fn get_one(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let property = property::Query::get_one_json(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(property)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/")]
async fn create(
    conn: web::Data<KojiDb>,
    payload: web::Json<property::Model>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let return_payload = property::Query::create(&conn.koji_db, payload)
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
    payload: web::Json<property::Model>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let updated_property = payload.into_inner();

    let result = property::Query::update(&conn.koji_db, id, updated_property)
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

    let propertys = property::Query::delete(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(propertys.rows_affected)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/search/")]
async fn search(conn: web::Data<KojiDb>, url: web::Query<Search>) -> Result<HttpResponse, Error> {
    let url = url.into_inner();

    let propertys = property::Query::search(&conn.koji_db, url.query)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(propertys)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
