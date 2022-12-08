use serde_json::json;

use super::*;

use crate::models::api::{Args, ArgsUnwrapped, Response};
use crate::models::KojiDb;
use crate::queries::geofence;
use crate::utils::convert::collection;

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
        data: Some(json!(collection::from_features(features))),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[post("/save")]
async fn save(conn: web::Data<KojiDb>, payload: web::Json<Args>) -> Result<HttpResponse, Error> {
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
