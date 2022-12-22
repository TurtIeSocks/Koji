use super::*;

use geojson::{Feature, JsonValue};
use serde_json::json;

use crate::{
    models::{
        api::{Args, Response},
        KojiDb, ToCollection,
    },
    queries::{area, instance},
};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();

    println!("\n[INSTANCE-ALL] Scanner Type: {}", scanner_type);

    let instances = if scanner_type.eq("rdm") {
        instance::all(&conn.data_db, None).await
    } else if let Some(unown_db) = conn.unown_db.as_ref() {
        area::all(unown_db).await
    } else {
        Ok(Vec::<Feature>::new())
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[INSTANCE_ALL] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(instances.to_collection(None))),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
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
        return Ok(HttpResponse::BadRequest().json(Response::send_error("invalid_scanner_type")));
    }

    let instances = instance::all(&conn.data_db, Some(instance_type))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let instances: Vec<String> = instances
        .into_iter()
        .map(|inst| {
            inst.property("name")
                .unwrap_or(&JsonValue::String("".to_string()))
                .to_string()
        })
        .collect();

    println!("[INSTANCE-TYPE] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(instances))
}

#[post("/area")]
async fn get_area(
    conn: web::Data<KojiDb>,
    payload: web::Json<Args>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let instance = payload.into_inner().instance.unwrap_or("".to_string());

    println!(
        "\n[INSTANCE] Scanner Type: {}, Instance: {}",
        scanner_type, instance
    );

    if instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(Response::send_error("no_instance_provided")));
    }

    let instance_data = if scanner_type.eq("rdm") {
        instance::route(&conn.data_db, &instance).await
    } else {
        area::route(&conn.unown_db.as_ref().unwrap(), &instance).await
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    // println!("[INSTANCE-AREA] Returning {} coords\n", instance_data.geometry len(),);
    Ok(HttpResponse::Ok().json(instance_data))
}
