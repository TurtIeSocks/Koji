use crate::utils::request::send_api_req;

use super::*;

use serde_json::json;

use model::{
    api::{
        args::{ApiQueryArgs, Response},
        collection::Default,
        GeoFormats,
    },
    db::{area, geofence, instance, project},
    KojiDb,
};

#[get("/push/{id}")]
async fn push_to_prod(
    conn: web::Data<KojiDb>,
    scanner_type: web::Data<String>,
    id: actix_web::web::Path<String>,
) -> Result<HttpResponse, Error> {
    let id = id.into_inner();
    let scanner_type = scanner_type.as_ref();

    let features = geofence::Query::project_as_feature(
        &conn.koji_db,
        id.clone(),
        &ApiQueryArgs {
            name: Some(true),
            mode: Some(true),
            ..Default::default()
        },
    )
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    let project = project::Query::get_one(&conn.koji_db, id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;

    let (inserts, updates) = if project.scanner {
        if scanner_type == "rdm" {
            instance::Query::upsert_from_geometry(
                &conn.data_db,
                GeoFormats::FeatureVec(features),
                false,
            )
            .await
        } else {
            area::Query::upsert_from_geometry(
                &conn.unown_db.as_ref().unwrap(),
                GeoFormats::FeatureVec(features),
            )
            .await
        }
        .map_err(actix_web::error::ErrorInternalServerError)?
    } else {
        (0, 0)
    };
    let scanner_type = if project.scanner {
        scanner_type.to_string()
    } else {
        String::from("other")
    };
    send_api_req(project, Some(&scanner_type))
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
