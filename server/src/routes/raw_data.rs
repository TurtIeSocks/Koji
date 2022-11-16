use super::*;
use crate::models::{
    api::{CustomError, MapBounds, RouteGeneration},
    KojiDb,
};
use crate::queries::{area, gym, instance, pokestop, spawnpoint};
use crate::utils::convert::normalize;

#[get("/all/{category}")]
async fn all(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let category = category.into_inner();
    let category_copy = category.clone();
    let scanner_type = scanner_type.as_ref();

    println!(
        "\n[DATA-ALL] Scanner Type: {}, Category: {}",
        scanner_type, category
    );

    let all_data = web::block(move || async move {
        match category.as_str() {
            "gym" => gym::all(&conn.data_db).await,
            "pokestop" => pokestop::all(&conn.data_db).await,
            "spawnpoint" => spawnpoint::all(&conn.data_db).await,
            _ => Err(DbErr::Custom("invalid_category".to_string())),
        }
    })
    .await?
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!(
        "[DATA-ALL] Returning {} {}s\n",
        all_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(all_data))
}

#[post("/bound/{category}")]
async fn bound(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
    payload: web::Json<MapBounds>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let category = category.into_inner();
    let category_copy = category.clone();

    println!(
        "\n[DATA-BOUND] Scanner Type: {}, Category: {}",
        scanner_type, category
    );

    let bound_data = web::block(move || async move {
        match category.as_str() {
            "gym" => gym::bound(&conn.data_db, &payload).await,
            "pokestop" => pokestop::bound(&conn.data_db, &payload).await,
            "spawnpoint" => spawnpoint::bound(&conn.data_db, &payload).await,
            _ => Err(DbErr::Custom("invalid_category".to_string())),
        }
    })
    .await?
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!(
        "[DATA-BOUND] Returning {} {}s\n",
        bound_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(bound_data))
}

#[post("/area/{category}")]
async fn by_area(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref().clone();
    let category = category.into_inner();
    let category_copy = category.clone();

    let RouteGeneration {
        instance,
        area,
        radius: _radius,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
        min_points: _min_points,
        fast: _fast,
        return_type: _return_type,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let (area, _return_type) = normalize::area_input(area);

    println!(
        "\n[DATA-AREA] Scanner Type: {}, Category: {}, Instance: {}, Custom Area: {}",
        scanner_type,
        category,
        instance,
        !area.features.is_empty(),
    );

    if area.features.is_empty() && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area_data = web::block(move || async move {
        let area = if !area.features.is_empty() {
            area
        } else if scanner_type == "rdm" {
            instance::route(&conn.data_db, &instance).await?
        } else if conn.unown_db.is_some() {
            area::route(&conn.unown_db.as_ref().unwrap(), &instance).await?
        } else {
            area
        };
        if category == "gym" {
            gym::area(&conn.data_db, area).await
        } else if category == "pokestop" {
            pokestop::area(&conn.data_db, area).await
        } else {
            spawnpoint::area(&conn.data_db, area).await
        }
    })
    .await?
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!(
        "[DATA-AREA] Returning {} {}s\n",
        area_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(area_data))
}
