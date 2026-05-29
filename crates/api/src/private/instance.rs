use super::*;

use model::{
    api::args::ApiQueryArgs,
    db::{route, NameTypeId},
    ScannerType,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    model::{
        db::{area, geofence, instance},
        KojiDb,
    },
    utils::response::Response,
};

#[get("/from_scanner")]
async fn from_scanner(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    log::info!("[INSTANCE-ALL] Scanner Type: {}", conn.scanner_type);

    let instances = if conn.scanner_type == ScannerType::Unown {
        area::Query::all(&conn.controller).await
    } else {
        instance::Query::get_json_cache(&conn.controller).await
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[INSTANCE_ALL] Returning {} instances", instances.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(instances)),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}

#[get("/from_koji")]
async fn from_koji(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    log::info!("[INSTANCE-ALL] Scanner Type: {}", conn.scanner_type);

    let fences = geofence::Query::get_all_no_fences(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let mut fences: Vec<NameTypeId> = fences
        .into_iter()
        .map(|instance| NameTypeId {
            id: instance.id,
            name: instance.name,
            mode: instance.mode,
            geo_type: None,
        })
        .collect();

    let routes = route::Query::get_all_no_fences(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    routes.into_iter().for_each(|instance| {
        fences.push(NameTypeId {
            id: instance.id,
            name: instance.name,
            mode: instance.mode,
            geo_type: None,
        })
    });

    log::info!("[INSTANCE_ALL] Returning {} instances", fences.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fences)),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}

#[derive(Debug, Deserialize)]
struct UrlVars {
    source: String,
    id: u32,
    instance_type: String,
}

#[get("/one/{source}/{id}/{instance_type}")]
async fn route_from_db(
    conn: web::Data<KojiDb>,
    instance: actix_web::web::Path<UrlVars>,
) -> Result<HttpResponse, Error> {
    let UrlVars {
        source,
        id,
        instance_type,
    } = instance.into_inner();

    let feature = if source.eq("scanner") {
        if conn.scanner_type == ScannerType::Unown {
            area::Query::feature(&conn.controller, id, instance_type).await
        } else {
            instance::Query::feature(&conn.controller, id).await
        }
    } else {
        if instance_type.eq("circle_pokemon")
            || instance_type.eq("circle_smart_pokemon")
            || instance_type.eq("circle_quest")
            || instance_type.eq("circle_raid")
            || instance_type.eq("circle_smart_raid")
        {
            route::Query::feature(&conn.koji, id, true).await
        } else {
            geofence::Query::get_one_feature(&conn.koji, id.to_string(), &ApiQueryArgs::default())
                .await
        }
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(feature)),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}
