use super::*;
use crate::queries::{gym, instance::query_instance_route, pokestop, spawnpoint};
use crate::utils::pixi_marker::pixi_marker;
use crate::{
    models::{
        api::{CustomError, MapBounds, RouteGeneration},
        scanner::InstanceData,
    },
    utils::to_array::coord_to_array,
};

#[get("/all/{category}")]
async fn all(
    pool: web::Data<DbPool>,
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

    let all_data = web::block(move || {
        let conn = pool.get()?;
        if category == "gym" {
            gym::all(&conn)
        } else if category == "pokestop" {
            pokestop::all(&conn)
        } else {
            spawnpoint::all(&conn)
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let all_data = pixi_marker(&all_data, &category_copy);

    println!(
        "[DATA-ALL] Returning {} {}s\n",
        all_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(all_data))
}

#[post("/bound/{category}")]
async fn bound(
    pool: web::Data<DbPool>,
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

    let bound_data = web::block(move || {
        let conn = pool.get()?;
        if category == "gym" {
            gym::bound(&conn, &payload)
        } else if category == "pokestop" {
            pokestop::bound(&conn, &payload)
        } else {
            spawnpoint::bound(&conn, &payload)
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let bound_data = pixi_marker(&bound_data, &category_copy);

    println!(
        "[DATA-BOUND] Returning {} {}s\n",
        bound_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(bound_data))
}

#[post("/area/{category}")]
async fn area(
    pool: web::Data<DbPool>,
    scanner_type: web::Data<String>,
    category: actix_web::web::Path<String>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let category = category.into_inner();
    let category_copy = category.clone();

    let RouteGeneration {
        instance,
        area,
        radius: _radius,
        generations: _generations,
        devices: _devices,
        data_points: _data_points,
    } = payload.into_inner();
    let instance = instance.unwrap_or("".to_string());
    let area = area.unwrap_or(vec![]);

    println!(
        "\n[DATA-AREA] Scanner Type: {}, Category: {}, Instance: {}, Custom Area: {}",
        scanner_type,
        category,
        instance,
        area.len() > 0
    );

    if !scanner_type.eq("rdm") && area.len() == 0 {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_provided_and_invalid_scanner_type".to_string(),
        }));
    }
    if area.len() == 0 && instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_area_and_empty_instance".to_string(),
        }));
    }

    let area_data = web::block(move || {
        let conn = pool.get()?;
        let area = if area.len() > 0 {
            area
        } else {
            let data = query_instance_route(&conn, &instance)?;
            let data: InstanceData =
                serde_json::from_str(data.data.as_str()).expect("JSON was not well-formatted");
            coord_to_array(data.area[0].clone())
        };
        if category == "gym" {
            gym::area(&conn, &area)
        } else if category == "pokestop" {
            pokestop::area(&conn, &area)
        } else {
            spawnpoint::area(&conn, &area)
        }
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let area_data = pixi_marker(&area_data, &category_copy);

    println!(
        "[DATA-AREA] Returning {} {}s\n",
        area_data.len(),
        category_copy
    );
    Ok(HttpResponse::Ok().json(area_data))
}
