use super::*;

use model::error::ModelError;
use serde::Deserialize;
use serde_json::json;

use crate::model::{api::args::Response, db, KojiDb};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminReq {
    pub page: Option<u64>,
    pub per_page: Option<u64>,
    pub sort_by: Option<String>,
    pub order: Option<String>,
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Search {
    pub query: String,
}

pub struct AdminReqParsed {
    pub page: u64,
    pub per_page: u64,
    pub sort_by: String,
    pub order: String,
    pub q: String,
}

impl AdminReq {
    fn parse(self) -> AdminReqParsed {
        AdminReqParsed {
            page: self.page.unwrap_or(0),
            order: self.order.unwrap_or("ASC".to_string()),
            per_page: self.per_page.unwrap_or(25),
            sort_by: self.sort_by.unwrap_or("id".to_string()),
            q: self.q.unwrap_or("".to_string()),
        }
    }
}

#[get("/{resource}/")]
async fn paginate(
    db: web::Data<KojiDb>,
    query: web::Query<AdminReq>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let AdminReqParsed {
        page,
        per_page,
        sort_by,
        order,
        q,
    } = query.into_inner().parse();
    let path = path.into_inner();

    let paginated_results = match path.to_lowercase().as_str() {
        "geofence" => {
            db::geofence::Query::paginate(&db.koji_db, page, per_page, order, sort_by, q).await
        }
        "project" => {
            db::project::Query::paginate(&db.koji_db, page, per_page, order, sort_by, q).await
        }
        "property" => {
            db::property::Query::paginate(&db.koji_db, page, per_page, order, sort_by, q).await
        }
        "route" => db::route::Query::paginate(&db.koji_db, page, per_page, order, sort_by, q).await,
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

#[get("/{resource}/all/")]
async fn get_all(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let results = match path.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::get_json_cache(&db.koji_db).await,
        "project" => db::project::Query::get_json_cache(&db.koji_db).await,
        "property" => db::property::Query::get_json_cache(&db.koji_db).await,
        "route" => db::route::Query::get_json_cache(&db.koji_db).await,
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

#[get("/{resource}/search/")]
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
