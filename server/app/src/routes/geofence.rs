use serde_json::json;

use super::*;

use crate::models::api::{Args, ArgsUnwrapped, Response};
use crate::models::{KojiDb, ToCollection};
use crate::queries::{area, geofence, instance};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    // url: actix_web::web::Path<Option<String>>,
) -> Result<HttpResponse, Error> {
    let features = geofence::all(&conn.koji_db)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[GEOFENCES_ALL] Returning {} instances\n", features.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(features.to_collection(None, None))),
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

    let (inserts, updates) = geofence::save(&conn.koji_db, area)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("Rows Updated: {}, Rows Inserted: {}", updates, inserts);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updates": updates, "inserts": inserts })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/save-scanner")]
async fn save_scanner(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("geofence_save"));

    let (inserts, updates) = if scanner_type == "rdm" {
        instance::save(&conn.data_db, area).await
    } else {
        area::save(&conn.unown_db.as_ref().unwrap(), area).await
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("Rows Updated: {}, Rows Inserted: {}", updates, inserts);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updates": updates, "inserts": inserts })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
