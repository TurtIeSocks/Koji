use super::*;

use crate::queries::{area, instance};
use crate::{
    models::{
        api::{CustomError, RouteGeneration},
        scanner::GenericInstance,
        KojiDb,
    },
    utils::convert::geo::arr_to_fc,
};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();

    println!("\n[INSTANCE-ALL] Scanner Type: {}", scanner_type);

    let instances = web::block(move || async move {
        if scanner_type.eq("rdm") {
            instance::all(&conn.data_db, None).await
        } else if conn.unown_db.is_some() {
            area::all(&(conn.unown_db.as_ref().unwrap())).await
        } else {
            Ok(Vec::<GenericInstance>::new())
        }
    })
    .await?
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[INSTANCE_ALL] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(arr_to_fc(instances)))
}

#[get("/type/{instance_type}")]
async fn instance_type(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    instance_type: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let instance_type = instance_type.into_inner();

    println!(
        "\n[INSTANCE-TYPE] Scanner Type: {}, Instance Type: {}",
        scanner_type, instance_type
    );

    if !scanner_type.eq("rdm") {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "invalid_scanner_type".to_string(),
        }));
    }

    let instances =
        web::block(move || async move { instance::all(&conn.data_db, Some(instance_type)).await })
            .await?
            .await
            .map_err(actix_web::error::ErrorInternalServerError)?;

    let instances: Vec<String> = instances.iter().map(|inst| inst.name.clone()).collect();

    println!("[INSTANCE-TYPE] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(instances))
}

#[post("/area")]
async fn get_area(
    conn: web::Data<KojiDb>,
    payload: web::Json<RouteGeneration>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref().clone();
    let instance = payload.into_inner().instance.unwrap_or("".to_string());

    println!(
        "\n[INSTANCE] Scanner Type: {}, Instance: {}",
        scanner_type, instance
    );

    if instance.clone().is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_instance_provided".to_string(),
        }));
    }

    let instance_data = web::block(move || async move {
        if scanner_type.eq("rdm") {
            instance::route::<f32>(&conn.data_db, &instance).await
        } else {
            area::route::<f32>(&conn.unown_db.as_ref().unwrap(), &instance).await
        }
    })
    .await?
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[INSTANCE-AREA] Returning {} coords\n", instance_data.len(),);
    Ok(HttpResponse::Ok().json(instance_data))
}
