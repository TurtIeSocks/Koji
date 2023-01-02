use super::*;

use serde_json::json;

use crate::model::{
    api::{args::Response, ToCollection},
    db::{area, instance},
    KojiDb,
};

#[get("/all")]
async fn all(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.to_string();

    println!("\n[INSTANCE-ALL] Scanner Type: {}", scanner_type);

    let instances = if scanner_type.eq("rdm") {
        instance::Query::all(&conn.data_db, None).await
    } else if let Some(unown_db) = conn.unown_db.as_ref() {
        area::Query::all(unown_db).await
    } else {
        Ok(Vec::<Feature>::new())
    }
    .map_err(actix_web::error::ErrorInternalServerError)?;

    println!("[INSTANCE_ALL] Returning {} instances\n", instances.len());
    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!(instances.to_collection(None, None))),
        message: "ok".to_string(),
        status_code: 200,
        status: "Success".to_string(),
        stats: None,
    }))
}

#[get("/area/{instance_name}")]
async fn one(
    conn: web::Data<KojiDb>,
    instance: actix_web::web::Path<String>,
    scanner_type: web::Data<String>,
) -> Result<HttpResponse, Error> {
    let scanner_type = scanner_type.as_ref();
    let instance = instance.into_inner();

    println!(
        "\n[INSTANCE] Scanner Type: {}, Instance: {}",
        scanner_type, instance
    );

    if instance.is_empty() {
        return Ok(HttpResponse::BadRequest().json(Response::send_error("no_instance_provided")));
    }

    let instance_data = utils::load_feature(&instance, &scanner_type, &conn)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    // println!("[INSTANCE-AREA] Returning {} coords\n", instance_data.geometry len(),);
    Ok(HttpResponse::Ok().json(instance_data))
}
