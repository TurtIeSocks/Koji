use super::*;

use koji_core::AdminReq;
use koji_db::ModelError;
use serde_json::json;

use koji_db::{KojiDb, db};

use crate::private::Search;
use crate::utils::response::ok_response;

#[get("/{resource}/")]
async fn paginate(
    db: web::Data<KojiDb>,
    query: web::Query<AdminReq>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let parsed = query.into_inner().parse();
    let resource = path.into_inner();

    let paginated_results = resource_dispatch!(resource, DbErr, paginate(&db.koji, parsed))
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(paginated_results)))
}

#[get("/{resource}/parent")]
async fn parent_list(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let resource = path.into_inner();

    let results = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::unique_parents(&db.koji).await,
        "route" => db::route::Query::unique_geofence(&db.koji).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(results)))
}

#[get("/{resource}/all/")]
async fn get_all(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let resource = path.into_inner();

    let results = resource_dispatch!(resource, DbErr, get_json_cache(&db.koji))
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(results)))
}

#[get("/{resource}/{id}/")]
async fn get_one(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, String)>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();

    let result = match resource.to_lowercase().as_str() {
        "geofence" => db::geofence::Query::get_one_json_with_related(&db.koji, id).await,
        "project" => db::project::Query::get_one_json_with_related(&db.koji, id).await,
        "property" => db::property::Query::get_one_json(&db.koji, id).await,
        "route" => db::route::Query::get_one_json(&db.koji, id).await,
        "tileserver" => db::tile_server::Query::get_one_json(&db.koji, id).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(result)))
}

#[post("/{resource}/")]
async fn create(
    db: web::Data<KojiDb>,
    payload: web::Json<serde_json::Value>,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let payload = payload.into_inner();
    let resource = path.into_inner();

    let result = resource_dispatch!(resource, ModelError, upsert_json_return(&db.koji, 0, payload))
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(result)))
}

#[patch("/{resource}/{id}/")]
async fn update(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, u32)>,
    payload: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();
    let payload = payload.into_inner();

    let result = resource_dispatch!(resource, ModelError, upsert_json_return(&db.koji, id, payload))
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(result)))
}

#[delete("/{resource}/{id}/")]
async fn remove(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<(String, u32)>,
) -> Result<HttpResponse, Error> {
    let (resource, id) = path.into_inner();

    let result = resource_dispatch!(resource, DbErr, delete(&db.koji, id))
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(ok_response(json!(result.rows_affected)))
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
        "geofence" => db::geofence::Query::assign(&db.koji, id, property, payload).await,
        // "project" => db::project::Query::search(&db.koji_db, search.query).await,
        // "property" => db::property::Query::search(&db.koji_db, search.query).await,
        // "route" => db::route::Query::search(&db.koji_db, search.query).await,
        // "tileserver" => db::tile_server::Query::search(&db.koji_db, search.query).await,
        _ => Err(ModelError::Custom("Invalid Resource".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ok_response(json!(results)))
}

#[get("/search/{resource}/")]
async fn search(
    db: web::Data<KojiDb>,
    path: actix_web::web::Path<String>,
    url: web::Query<Search>,
) -> Result<HttpResponse, Error> {
    let search = url.into_inner();
    let resource = path.into_inner();

    let results = resource_dispatch!(resource, DbErr, search(&db.koji, search.query))
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(ok_response(json!(results)))
}
