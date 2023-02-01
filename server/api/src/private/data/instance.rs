use super::*;

use model::{
    db::{route, NameTypeId},
    utils::get_enum,
};
use serde::Deserialize;
use serde_json::json;

use crate::model::{
    api::args::Response,
    db::{area, geofence, instance},
    KojiDb,
};

#[get("/from_scanner")]
async fn from_scanner(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();

    println!("\n[INSTANCE-ALL] Scanner Type: {}", scanner_type);

    let instances = if scanner_type.eq("rdm") {
        instance::Query::get_json_cache(&conn.data_db).await
    } else if let Some(unown_db) = conn.unown_db.as_ref() {
        area::Query::all(unown_db).await
    } else {
        Err(DbErr::Custom(
            "[DB] Scanner is not configured correctly".to_string(),
        ))
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[INSTANCE_ALL] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(instances)),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}

#[get("/from_koji")]
async fn from_koji(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();

    println!("\n[INSTANCE-ALL] Scanner Type: {}", scanner_type);

    let fences = geofence::Query::get_all_no_fences(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let mut fences: Vec<NameTypeId> = fences
        .into_iter()
        .map(|instance| NameTypeId {
            id: instance.id,
            name: instance.name,
            mode: get_enum(instance.mode),
            geo_type: None,
        })
        .collect();

    let routes = route::Query::get_all_no_fences(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    routes.into_iter().for_each(|instance| {
        fences.push(NameTypeId {
            id: instance.id,
            name: instance.name,
            mode: get_enum(Some(instance.mode)),
            geo_type: None,
        })
    });

    println!("[INSTANCE_ALL] Returning {} instances\n", fences.len());
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
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();
    let UrlVars {
        source,
        id,
        instance_type,
    } = instance.into_inner();

    let instances = if source.eq("scanner") {
        if scanner_type.eq("rdm") {
            instance::Query::feature(&conn.data_db, id).await
        } else if let Some(unown_db) = conn.unown_db.as_ref() {
            area::Query::feature(unown_db, id, instance_type).await
        } else {
            Ok(Feature::default())
        }
    } else {
        if instance_type.eq("CirclePokemon")
            || instance_type.eq("CircleSmartPokemon")
            || instance_type.eq("ManualQuest")
            || instance_type.eq("CircleRaid")
            || instance_type.eq("CircleSmartRaid")
        {
            route::Query::feature(&conn.koji_db, id).await
        } else {
            geofence::Query::feature(&conn.koji_db, id).await
        }
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    // println!("[INSTANCE_ALL] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(instances)),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}
