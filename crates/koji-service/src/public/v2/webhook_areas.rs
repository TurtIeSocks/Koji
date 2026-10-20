//! `GET /api/v2/webhooks/{id}/areas` — a Dragonite target's Areas, for the
//! admin's Area picker.

use actix_web::{HttpResponse, web};
use koji_db::KojiDb;
use koji_db::db::dragonite_link;
use koji_db::db::sea_orm_active_enums::Mode;
use sea_orm::ActiveEnum;
use serde::Serialize;

use super::geofences::dragonite_target;
use crate::dragonite::client_for;
use crate::utils::api_response::{ApiError, ApiResponse};
use crate::utils::error::ServiceError;

/// A geofence holding an Area on the target.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct Holder {
    geofence: u32,
    name: String,
    mode: String,
}

/// One of the target's Areas and the geofences holding it.
#[derive(Debug, Serialize, PartialEq, Eq)]
struct AreaOption {
    id: u32,
    name: String,
    /// The geofences linked to this Area on the target, whatever their mode. An
    /// Area takes one geofence per mode, so the picker reads the mode it needs.
    held_by: Vec<Holder>,
}

/// `GET /api/v2/webhooks/{id}/areas` — every Area the Dragonite behind a
/// `dragonite` webhook has, in the order it lists them (all pages), as `id` and
/// `name`, each with the geofences linked to it on this webhook (`held_by`).
/// An Area without a name or with an id Koji cannot store is left out. `404`
/// on an unknown webhook, `422` when it is another mode, `502` when Dragonite
/// cannot be read.
#[utoipa::path(
    get,
    path = "/api/v2/webhooks/{id}/areas",
    tag = "webhooks",
    params(("id" = u64, Path, description = "Dragonite webhook id")),
    responses(
        (status = 200, description = "The Areas: `[{ id, name, held_by: [{ geofence, name, mode }] }]`", body = Object),
        (status = 404, description = "No such webhook", body = ApiError),
        (status = 422, description = "The webhook is not a Dragonite", body = ApiError),
        (status = 502, description = "Dragonite could not be read", body = ApiError),
    ),
)]
pub(crate) async fn list_areas(
    conn: web::Data<KojiDb>,
    path: web::Path<u64>,
) -> Result<HttpResponse, ServiceError> {
    let target = dragonite_target(&conn.koji, path.into_inner()).await?;
    let areas = client_for(&target)
        .list_areas()
        .await
        .map_err(|e| ServiceError::Upstream {
            field: "dragonite",
            message: format!("could not read Dragonite areas: {e}"),
        })?;
    let holders = dragonite_link::Query::holders(&conn.koji, target.id).await?;

    let options: Vec<AreaOption> = areas
        .into_iter()
        .filter_map(|area| {
            let id = u32::try_from(area.id?).ok()?;
            Some(AreaOption {
                id,
                name: area.name?,
                held_by: holders
                    .iter()
                    .filter(|h| h.dragonite_area_id == id)
                    .map(|h| Holder {
                        geofence: h.geofence_id,
                        name: h.name.clone(),
                        mode: mode_name(h.mode),
                    })
                    .collect(),
            })
        })
        .collect();
    Ok(ApiResponse::success(options))
}

fn mode_name(mode: Mode) -> String {
    mode.to_value()
}
