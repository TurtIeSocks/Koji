use crate::utils::response::Response;

use super::*;

use serde_json::json;

use koji_core::{ApiQueryArgs, EnsurePoints, ReturnTypeArg};
use koji_db::{KojiDb, db::route};
use model::api::args::{Args, ArgsUnwrapped, get_return_type};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let args = args.into_inner();
    let fc = route::Query::as_collection(&conn.koji, args.internal.unwrap_or(false))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[PUBLIC_API] Returning {} routes", fc.features.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fc)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/area/{id}")]
async fn get_area(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(
        args.rt.unwrap_or("feature".to_string()),
        &ReturnTypeArg::Feature,
    );

    let geometry = route::Query::get_one_koji(&conn.koji, id)
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

#[get("/reference")]
async fn reference_data(conn: web::Data<KojiDb>) -> Result<HttpResponse, Error> {
    let fences = route::Query::get_all_no_fences(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[ROUTE_REF] Returning {} instances", fences.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(fences)),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}

#[get("/reference/{geofence}")]
async fn reference_data_geofence(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let geofence = url.into_inner();
    let fences = route::Query::by_geofence(&conn.koji, geofence)
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

#[post("/save-koji")]
async fn save_koji(
    conn: web::Data<KojiDb>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped { area, .. } = payload.into_inner().init(Some("geofence_save"));

    let area = koji_core::KojiGeometryCollection::try_from(area)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    let (inserts, updates) = route::Query::upsert_from_geometry(&conn.koji, &area)
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

#[get("/{return_type}")]
async fn specific_return_type(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<String>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let return_type = url.into_inner();
    // `internal` is moot for the self-describing `KojiMeta` collection, so the
    // `?internal=`/`?rt=` query args go unread here (kept in the signature for
    // wire compatibility).
    let _ = args;
    let return_type = get_return_type(return_type, &ReturnTypeArg::FeatureCollection);

    let coll = route::Query::as_koji_collection(&conn.koji)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[GEOFENCES_ALL] Returning {} instances", coll.items.len());
    Ok(utils::response::send(coll, return_type, None, false, None))
}

#[get("/{return_type}/{geofence_name}")]
async fn specific_geofence(
    conn: web::Data<KojiDb>,
    url: actix_web::web::Path<(String, String)>,
    args: web::Query<ApiQueryArgs>,
) -> Result<HttpResponse, Error> {
    let (return_type, geofence_name) = url.into_inner();
    let args = args.into_inner();
    let return_type = get_return_type(return_type, &ReturnTypeArg::FeatureCollection);
    let features = route::Query::by_geofence_feature(
        &conn.koji,
        geofence_name,
        args.internal.unwrap_or(false),
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[GEOFENCES_FC_ALL] Returning {} instances", features.len());
    // Wrap the per-row route features into a geojson `FeatureCollection` (closing
    // polygon rings as the old matrix `to_collection` did) then go Koji-native
    // via the Phase 1 `TryFrom`. No `To*` matrix.
    let fc = geojson::FeatureCollection {
        bbox: None,
        features: features.into_iter().map(EnsurePoints::ensure_first_last).collect(),
        foreign_members: None,
    };
    let coll = koji_core::KojiGeometryCollection::try_from(fc)
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(utils::response::send(coll, return_type, None, false, None))
}
