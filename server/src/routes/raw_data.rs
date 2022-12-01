use super::*;

use crate::models::{
    api::{Args, ArgsUnwrapped, BoundsArg},
    CustomError, KojiDb,
};
use crate::queries::{area, gym, instance, pokestop, spawnpoint};

#[get("/all/{category}")]
async fn all(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();
    let scanner_type = scanner_type.as_ref();

    println!(
        "\n[DATA_ALL] Scanner Type: {} | Category: {}",
        scanner_type, category
    );

    let all_data = match category.as_str() {
        "gym" => gym::all(&conn.data_db).await,
        "pokestop" => pokestop::all(&conn.data_db).await,
        "spawnpoint" => spawnpoint::all(&conn.data_db).await,
        _ => Err(DbErr::Custom("invalid_category".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[DATA-ALL] Returning {} {}s\n", all_data.len(), category);
    Ok(HttpResponse::Ok().json(all_data))
}

#[post("/bound/{category}")]
async fn bound(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
    payload: web::Json<BoundsArg>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let category = category.into_inner();

    println!(
        "\n[DATA_BOUND] Scanner Type: {} | Category: {}",
        scanner_type, category
    );

    let bound_data = match category.as_str() {
        "gym" => gym::bound(&conn.data_db, &payload).await,
        "pokestop" => pokestop::bound(&conn.data_db, &payload).await,
        "spawnpoint" => spawnpoint::bound(&conn.data_db, &payload).await,
        _ => Err(DbErr::Custom("invalid_category".to_string())),
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!(
        "[DATA-BOUND] Returning {} {}s\n",
        bound_data.len(),
        category
    );
    Ok(HttpResponse::Ok().json(bound_data))
}

#[post("/area/{category}")]
async fn by_area(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
    payload: web::Json<Args>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let category = category.into_inner();

    let ArgsUnwrapped {
        area,
        benchmark_mode: _benchmark_mode,
        data_points: _data_points,
        devices: _devices,
        fast: _fast,
        generations: _generations,
        instance,
        min_points: _min_points,
        radius: _radius,
        return_type: _return_type,
        routing_time: _routing_time,
    } = payload.into_inner().init(None);

    println!(
        "\n[DATA_AREA] Scanner Type: {} | Category: {}",
        scanner_type, category
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area = if !area.features.is_empty() && !instance.is_empty() {
        if scanner_type == "rdm" {
            instance::route(&conn.data_db, &instance).await
        } else {
            area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
    } else {
        Ok(area)
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let area_data = if category == "gym" {
        gym::area(&conn.data_db, &area).await
    } else if category == "pokestop" {
        pokestop::area(&conn.data_db, &area).await
    } else {
        spawnpoint::area(&conn.data_db, &area).await
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[DATA-AREA] Returning {} {}s\n", area_data.len(), category);
    Ok(HttpResponse::Ok().json(area_data))
}
