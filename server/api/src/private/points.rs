use crate::utils::response::Response;

use super::*;

use model::{
    api::args::{Args, ArgsUnwrapped, BoundsArg},
    db::{gym, pokestop, spawnpoint},
    KojiDb,
};

#[post("/all/{category}")]
async fn all(
    conn: web::Data<KojiDb>,
    category: actix_web::web::Path<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let ArgsUnwrapped { last_seen, tth, .. } = payload.into_inner().init(Some("all_data"));
    let category = category.into_inner();

    log::info!(
        "[DATA_ALL] Scanner Type: {} | Category: {}",
        conn.scanner_type,
        category
    );

    let all_data = match category.as_str() {
        "gym" => gym::Query::all(&conn.scanner, last_seen).await,
        "pokestop" => pokestop::Query::all(&conn.scanner, last_seen).await,
        "spawnpoint" => spawnpoint::Query::all(&conn.scanner, last_seen, tth).await,
        _ => Err(DbErr::Custom("invalid_category".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[DATA-ALL] Returning {} {}s", all_data.len(), category);
    Ok(HttpResponse::Ok().json(all_data))
}

#[post("/bound/{category}")]
async fn bound(
    conn: web::Data<KojiDb>,
    category: actix_web::web::Path<String>,
    payload: web::Json<BoundsArg>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();
    let payload = payload.into_inner();

    log::info!(
        "[DATA_BOUND] Scanner Type: {} | Category: {}",
        conn.scanner_type,
        category
    );

    let bound_data = match category.as_str() {
        "gym" => gym::Query::bound(&conn.scanner, &payload).await,
        "pokestop" => pokestop::Query::bound(&conn.scanner, &payload).await,
        "spawnpoint" => spawnpoint::Query::bound(&conn.scanner, &payload).await,
        _ => Err(DbErr::Custom("invalid_category".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[DATA-BOUND] Returning {} {}s", bound_data.len(), category);
    Ok(HttpResponse::Ok().json(bound_data))
}

#[post("/area/{category}")]
async fn by_area(
    conn: web::Data<KojiDb>,
    category: actix_web::web::Path<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();

    let ArgsUnwrapped {
        area,
        instance,
        last_seen,
        tth,
        ..
    } = payload.into_inner().init(None);

    log::info!(
        "[DATA_AREA] Scanner Type: {} | Category: {}",
        conn.scanner_type,
        category
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }
    let area = utils::create_or_find_collection(&instance, &conn, area, &None, &vec![])
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let area_data = utils::points_from_area(&area, &category, &conn, last_seen, tth)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!("[DATA-AREA] Returning {} {}s", area_data.len(), category);
    Ok(HttpResponse::Ok().json(area_data))
}

#[post("/area_stats/{category}")]
async fn area_stats(
    conn: web::Data<KojiDb>,
    category: actix_web::web::Path<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();

    let ArgsUnwrapped {
        area,
        instance,
        last_seen,
        tth,
        ..
    } = payload.into_inner().init(None);

    log::info!(
        "[DATA_AREA] Scanner Type: {} | Category: {}",
        conn.scanner_type,
        category
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(
            HttpResponse::BadRequest().json(Response::send_error("no_area_and_empty_instance"))
        );
    }

    let area = utils::create_or_find_collection(&instance, &conn, area, &None, &vec![])
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let area_data = match category.as_str() {
        "gym" => gym::Query::stats(&conn.scanner, &area, last_seen).await,
        "pokestop" => pokestop::Query::stats(&conn.scanner, &area, last_seen).await,
        "spawnpoint" => spawnpoint::Query::stats(&conn.scanner, &area, last_seen, tth).await,
        _ => Err(DbErr::Custom("Invalid Category".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    log::info!(
        "[DATA-AREA] Returning {} Total: {}",
        category,
        area_data.total
    );
    Ok(HttpResponse::Ok().json(area_data))
}
