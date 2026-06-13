use crate::utils::response::Response;

use super::*;

use serde_json::json;

use koji_core::{ApiQueryArgs, FeatureCtx, ReturnTypeArg, ToCollection};
use koji_db::{KojiDb, db::geofence};
use model::api::args::{Args, ArgsUnwrapped, get_return_type};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let fc = geofence::Query::get_all_collection(&conn.koji, &args)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[PUBLIC_API] Returning {} instances", fc.features.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fc)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/area/{geofence}")]
async fn get_area(
    conn: web::Data<KojiDb>,
    geofence: actix_web::web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let id = geofence.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt.clone().unwrap_or("feature".to_string()),
        &ReturnTypeArg::Feature,
    );

    let geometry = geofence::Query::get_one_koji(&conn.koji, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!(
        "[PUBLIC_API] Returning feature for {:?}",
        geometry.meta.name
    );
    Ok(utils::response::send(
        koji_core::KojiGeometryCollection::new(vec![geometry]),
        return_type,
        None,
        false,
        None,
    ))
}

#[post("/save-koji")]
async fn save_koji(
    conn: web::Data<KojiDb>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("geofence_save"));

    let area = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    geofence::Query::upsert_from_geometry(&conn.koji, &area)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: None,
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[delete("/{id}")]
async fn remove(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let result = geofence::Query::delete(&conn.koji, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(result.rows_affected)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/reference")]
async fn reference_data(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let fences = geofence::Query::get_all_no_fences(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[GEOFENCES_ALL] Returning {} instances", fences.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fences)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/reference/{project}")]
async fn reference_data_project(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let project = url.into_inner();
    let fences = geofence::Query::by_project(&conn.koji, project)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fences)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/{return_type}")]
async fn specific_return_type(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let return_type = url.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(return_type, &ReturnTypeArg::FeatureCollection);

    let coll = geofence::Query::get_all_koji(&conn.koji, &args)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[GEOFENCES_ALL] Returning {} instances", coll.items.len());
    Ok(utils::response::send(coll, return_type, None, false, None))
}

#[get("/{return_type}/{project}")]
async fn specific_project(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<(String, String)>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let (return_type, project) = url.into_inner();
    let args = args.into_inner();

    let return_type = get_return_type(return_type, &ReturnTypeArg::FeatureCollection);
    let features = geofence::Query::project_as_feature(&conn.koji, project, &args)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[GEOFENCES_FC_ALL] Returning {} instances", features.len());
    let coll =
        koji_core::KojiGeometryCollection::try_from(features.to_collection(&FeatureCtx::default()))
            .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(coll, return_type, None, false, None))
}
