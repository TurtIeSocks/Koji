use crate::utils::{request::send_api_req, response::Response};

use super::*;

use serde_json::json;

use model::{
    api::{args::ApiQueryArgs, collection::Default, GeoFormats},
    db::{area, geofence, instance, project},
    KojiDb, ScannerType,
};

#[get("/push/{id}")]
async fn push_to_prod(
    conn: web::Data<KojiDb>,
    id: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();

    let features = geofence::Query::project_as_feature(
        &conn.koji,
        id.clone(),
        &ApiQueryArgs {
            name: Some(true),
            mode: Some(true),
            ..Default::default()
        },
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let project = project::Query::get_one(&conn.koji, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let (inserts, updates) = if project.scanner {
        if conn.scanner_type == ScannerType::Unown {
            area::Query::upsert_from_geometry(&conn.controller, GeoFormats::FeatureVec(features))
                .await
        } else {
            instance::Query::upsert_from_geometry(
                &conn.controller,
                GeoFormats::FeatureVec(features),
                false,
            )
            .await
        }
        .map_err(actix_web::error::ErrorInternalServerError)?
    } else {
        (0, 0)
    };
    send_api_req(project, Some(&conn.scanner_type))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    log::info!("Rows Updated: {}, Rows Inserted: {}", updates, inserts);

    Ok(HttpResponse::Ok().json(Response {
        data: Some(json!({ "updates": updates, "inserts": inserts })),
        message: "Success".to_string(),
        status: "ok".to_string(),
        stats: None,
        status_code: 200,
    }))
}
