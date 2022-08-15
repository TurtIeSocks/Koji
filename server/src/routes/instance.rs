use super::*;
use crate::models::{api::RouteGeneration, scanner::InstanceData};
use crate::queries::instance::*;

#[get("/api/instance/type/{instance_type}")]
async fn all(
    pool: web::Data<DbPool>,
    instance_type: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    println!("[INSTANCE] Type: {:?}", instance_type.as_str());

    let instances = web::block(move || {
        let conn = pool.get()?;
        query_all_instances(&conn, instance_type.as_str().to_string())
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let filtered: Vec<String> = instances.iter().map(|i| i.name.clone()).collect();
    Ok(HttpResponse::Ok().json(filtered))
}

#[post("/api/instance/area")]
async fn area(
    pool: web::Data<DbPool>,
    payload: web::Json<RouteGeneration>,
) -> Result<HttpResponse, Error> {
    let instance = web::block(move || {
        let conn = pool.get()?;

        query_instance_route(&conn, &payload.name)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let data: InstanceData =
        serde_json::from_str(instance.data.as_str()).expect("JSON was not well-formatted");

    Ok(HttpResponse::Ok().json(data.area))
}
