use super::*;
use crate::models::api::CustomError;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::instance::*;

#[get("/type/{instance_type}")]
async fn all(
    pool: web::Data<DbPool>,
    scanner_type: web::Data<String>,
    instance_type: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let instance_type = instance_type.into_inner();

    println!(
        "\n[INSTANCE] Scanner Type: {}, Instance Type: {}",
        scanner_type, instance_type
    );

    if !scanner_type.eq("rdm") {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "invalid_scanner_type".to_string(),
        }));
    }

    let instances = web::block(move || {
        let conn = pool.get()?;
        query_all_instances(&conn, instance_type)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let instances: Vec<String> = instances.iter().map(|inst| inst.name.clone()).collect();

    println!("[DATA-AREA] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(instances))
}

#[post("/area")]
async fn area(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let instance = payload.into_inner().instance.unwrap_or("".to_string());

    println!(
        "\n[INSTANCE] Scanner Type: {}, Instance: {}",
        scanner_type, instance
    );

    if !scanner_type.eq("rdm") {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "invalid_scanner_type".to_string(),
        }));
    }
    if instance.clone().is_empty() {
        return Ok(HttpResponse::BadRequest().json(CustomError {
            message: "no_instance_provided".to_string(),
        }));
    }

    let instance_data = web::block(move || {
        let conn = pool.get()?;
        query_instance_route(&conn, &instance)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let instance_data: InstanceData =
        serde_json::from_str(instance_data.data.as_str()).expect("JSON was not well-formatted");

    println!(
        "[INSTANCE-AREA] Returning {} coords\n",
        instance_data.area.len(),
    );
    Ok(HttpResponse::Ok().json(instance_data.area))
}
