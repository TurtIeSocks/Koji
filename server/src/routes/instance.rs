use super::*;
use crate::queries::instance::*;

#[get("/api/instance/all")]
async fn all(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let instances = web::block(move || {
        let conn = pool.get()?;
        query_all_instances(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(instances))
}
