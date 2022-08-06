use super::*;
use crate::utils::pixi_marker::pixi_gyms;
use crate::queries::gym::*;

#[get("/api/gym/all")]
async fn all(pool: web::Data<DbPool>) -> Result<HttpResponse, Error> {
    let gyms = web::block(move || {
        let conn = pool.get()?;
        query_all_gyms(&conn)
    })
    .await?
    .map_err(actix_web::error::ErrorInternalServerError)?;
    let gyms = pixi_gyms(&gyms);
    Ok(HttpResponse::Ok().json(gyms))
}
