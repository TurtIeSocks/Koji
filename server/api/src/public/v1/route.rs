use crate::utils::request::send_api_req;

use super::*;

use serde_json::json;

use model::{
    api::{
        args::{get_return_type, ApiQueryArgs, Args, ArgsUnwrapped, Response, ReturnTypeArg},
        GeoFormats, ToCollection,
    },
    db::{area, instance, project, route},
    KojiDb,
};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let fc = route::Query::as_collection(&conn.koji_db, Some(args))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[PUBLIC_API] Returning {} routes\n", fc.features.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fc)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/area/{area_name}")]
async fn get_area(
    conn: web::Data<KojiDb>,
    area: actix_web::web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let area = area.into_inner();
    let args = args.into_inner();
    let feature = route::Query::feature_from_name(&conn.koji_db, area, Some(args))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!(
        "[PUBLIC_API] Returning feature for {:?}\n",
        feature.property("name")
    );
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(feature)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/save-koji")]
async fn save_koji(
    conn: web::Data<KojiDb>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("geofence_save"));

    let (inserts, updates) =
        route::Query::upsert_from_geometry(&conn.koji_db, GeoFormats::FeatureCollection(area))
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("Rows Updated: {}, Rows Inserted: {}", updates, inserts);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updates": updates, "inserts": inserts })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/push/{id}")]
async fn push_to_prod(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    id: actix_web::web::Path<u32>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let scanner_type = scanner_type.as_ref();

    let feature = route::Query::feature(&conn.koji_db, id, None)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let (inserts, updates) = if scanner_type == "rdm" {
        instance::Query::upsert_from_geometry(&conn.data_db, GeoFormats::Feature(feature), false)
            .await
    } else {
        area::Query::upsert_from_geometry(
            &conn.unown_db.as_ref().unwrap(),
            GeoFormats::Feature(feature),
        )
        .await
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let project = project::Query::get_scanner_project(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    if let Some(project) = project {
        send_api_req(project, Some(scanner_type))
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;
    }
    log::info!("Rows Updated: {}, Rows Inserted: {}", updates, inserts);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updates": updates, "inserts": inserts })),
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

    let fc = route::Query::as_collection(&conn.koji_db, Some(args))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!(
        "[GEOFENCES_ALL] Returning {} instances\n",
        fc.features.len()
    );
    Ok(utils::response::send(fc, return_type, None, false, None))
}

#[get("/{return_type}/{project_name}")]
async fn specific_project(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<(String, String)>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let (return_type, project_name) = url.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(return_type, &ReturnTypeArg::FeatureCollection);
    let features = route::Query::by_geofence(&conn.koji_db, project_name, Some(args))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!(
        "[GEOFENCES_FC_ALL] Returning {} instances\n",
        features.len()
    );
    Ok(utils::response::send(
        features.to_collection(None, None),
        return_type,
        None,
        false,
        None,
    ))
}
