use super::*;

use model::{api::args::AdminReq, error::ModelError};
use serde::Deserialize;
use serde_json::json;

use crate::model::{api::args::Response, db, KojiDb};

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}

#[get("/{resource}/")]
async fn paginate(
    db: web::Data<KojiDb>,
    query: web::Query<AdminReq>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let parsed = query.into_inner().parse();
    let resource = path.into_inner();

    let paginated_results = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::paginate(&db.koji_db, parsed).await,
        "project" => db::project::Query::paginate(&db.koji_db, parsed).await,
        "property" => db::property::Query::paginate(&db.koji_db, parsed).await,
        "route" => db::route::Query::paginate(&db.koji_db, parsed).await,
        "tileserver" => db::tile_server::Query::paginate(&db.koji_db, parsed).await,
        _ => Err(DbErr::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(paginated_results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/geofence/parent")]
async fn parent_list(db: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let results = db::geofence::Query::unique_parents(&db.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/{resource}/all/")]
async fn get_all(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let resource = path.into_inner();

    let results = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::get_json_cache(&db.koji_db).await,
        "project" => db::project::Query::get_json_cache(&db.koji_db).await,
        "property" => db::property::Query::get_json_cache(&db.koji_db).await,
        "route" => db::route::Query::get_json_cache(&db.koji_db).await,
        "tileserver" => db::tile_server::Query::get_json_cache(&db.koji_db).await,
        _ => Err(DbErr::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/{resource}/{id}/")]
async fn get_one(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();

    let result = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::get_one_json_with_related(&db.koji_db, id).await,
        "project" => db::project::Query::get_one_json_with_related(&db.koji_db, id).await,
        "property" => db::property::Query::get_one_json(&db.koji_db, id).await,
        "route" => db::route::Query::get_one_json(&db.koji_db, id).await,
        "tileserver" => db::tile_server::Query::get_one_json(&db.koji_db, id).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/{resource}/")]
async fn create(
    db: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let resource = path.into_inner();

    let result = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::upsert_json_return(&db.koji_db, 0, payload).await,
        "project" => db::project::Query::upsert_json_return(&db.koji_db, 0, payload).await,
        "property" => db::property::Query::upsert_json_return(&db.koji_db, 0, payload).await,
        "route" => db::route::Query::upsert_json_return(&db.koji_db, 0, payload).await,
        "tileserver" => db::tile_server::Query::upsert_json_return(&db.koji_db, 0, payload).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[patch("/{resource}/{id}/")]
async fn update(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, u32)>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();
    let payload = payload.into_inner();

    let result = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::upsert_json_return(&db.koji_db, id, payload).await,
        "project" => db::project::Query::upsert_json_return(&db.koji_db, id, payload).await,
        "property" => db::property::Query::upsert_json_return(&db.koji_db, id, payload).await,
        "route" => db::route::Query::upsert_json_return(&db.koji_db, id, payload).await,
        "tileserver" => db::tile_server::Query::upsert_json_return(&db.koji_db, id, payload).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[delete("/{resource}/{id}/")]
async fn remove(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, u32)>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();

    let result = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::delete(&db.koji_db, id).await,
        "project" => db::project::Query::delete(&db.koji_db, id).await,
        "property" => db::property::Query::delete(&db.koji_db, id).await,
        "route" => db::route::Query::delete(&db.koji_db, id).await,
        "tileserver" => db::tile_server::Query::delete(&db.koji_db, id).await,
        _ => Err(DbErr::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result.rows_affected)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[patch("/assign/{resource}/{property}/{id}/")]
async fn assign(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, String, u32)>,
    payload: web::Json<serde_json::Value>,
    // url: web::Query<(String, String)>,
) -> Result<HttpResponse, Error> {
    // let search = url.into_inner();
    let payload = payload.into_inner();
    let (resource, property, id) = path.into_inner();

    let results = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::assign(&db.koji_db, id, property, payload).await,
        // "project" => db::project::Query::search(&db.koji_db, search.query).await,
        // "property" => db::property::Query::search(&db.koji_db, search.query).await,
        // "route" => db::route::Query::search(&db.koji_db, search.query).await,
        // "tileserver" => db::tile_server::Query::search(&db.koji_db, search.query).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/search/{resource}/")]
async fn search(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
    url: web::Query<Search>,
) -> Result<HttpResponse, Error> {
    let search = url.into_inner();
    let resource = path.into_inner();

    let results = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::search(&db.koji_db, search.query).await,
        "project" => db::project::Query::search(&db.koji_db, search.query).await,
        "property" => db::property::Query::search(&db.koji_db, search.query).await,
        "route" => db::route::Query::search(&db.koji_db, search.query).await,
        "tileserver" => db::tile_server::Query::search(&db.koji_db, search.query).await,
        _ => Err(DbErr::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(results)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
