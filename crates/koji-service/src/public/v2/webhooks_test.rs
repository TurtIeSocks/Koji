//! `POST /api/v2/webhooks/{id}/test` — synchronously fire one subscription with
//! a sample event and report the upstream result inline. Replaces v1's
//! "sync-to-test" workflow; deliberately bypasses the outbox (no retries — the
//! admin is watching the response).

use actix_web::{HttpResponse, web};
use koji_dragonite::{ApiArea, DragoniteError};
use koji_events::WebhookSubscriber;
use koji_events::entity::webhook_subscription;
use koji_events::types::{Event, EventId};
use sea_orm::EntityTrait;

use crate::dragonite::client_from;
use crate::utils::api_response::{ApiError, ApiResponse};
use crate::utils::error::ServiceError;

/// `POST /api/v2/webhooks/{id}/test` — fire a sample `webhook.test` event at
/// exactly this subscription (mode-aware: signed event POST or legacy ping),
/// synchronously, and report the upstream result. `404` on an unknown id;
/// otherwise always `200` — delivery failure is reported in the body
/// (`delivered: false`, `error: <message>`), not as an HTTP error, since the
/// admin is watching the response inline.
///
/// A `dragonite` row is not sent an event: its Area list is read instead, with
/// the row's URL and bearer, and the count answers in `area_count`, so a wrong
/// URL or bearer shows up here. A failed read answers `delivered: false` with
/// Dragonite's own error.
#[utoipa::path(
    post,
    path = "/api/v2/webhooks/{id}/test",
    tag = "webhooks",
    params(("id" = u32, Path, description = "Webhook subscription id")),
    responses(
        (status = 200, description = "Delivery attempted; result in body", body = Object),
        (status = 404, description = "No such webhook", body = ApiError),
    ),
)]
pub(crate) async fn test_fire(
    db: web::Data<koji_db::KojiDb>,
    path: web::Path<u32>,
) -> Result<HttpResponse, ServiceError> {
    let id = path.into_inner();
    let sub = webhook_subscription::Entity::find_by_id(id as u64)
        .one(&db.koji)
        .await
        .map_err(ServiceError::internal)?
        .ok_or(ServiceError::NotFound {
            field: "webhook",
            message: "does not exist".to_string(),
        })?;

    if sub.mode == "dragonite" {
        let areas = client_from(&sub.url, sub.secret.as_deref())
            .list_areas()
            .await;
        return Ok(ApiResponse::success(dragonite_result(areas)));
    }

    let event = Event {
        id: EventId::new().as_string(),
        topic: "webhook.test".to_string(),
        payload: serde_json::json!({
            "test": true,
            "webhookId": sub.id,
            "projectId": sub.project_id,
        }),
    };

    let subscriber = WebhookSubscriber::with_default_client(db.koji.clone());
    let data = match subscriber.deliver_to(&sub, &event).await {
        Ok(status) => serde_json::json!({
            "delivered": true,
            "upstream_status": status,
            "error": null,
        }),
        Err(e) => serde_json::json!({
            "delivered": false,
            "upstream_status": null,
            "error": e.to_string(),
        }),
    };
    Ok(ApiResponse::success(data))
}

/// The `/test` answer for a Dragonite row, from its Area list read.
fn dragonite_result(areas: Result<Vec<ApiArea>, DragoniteError>) -> serde_json::Value {
    match areas {
        Ok(areas) => serde_json::json!({
            "delivered": true,
            "upstream_status": 200,
            "area_count": areas.len(),
            "error": null,
        }),
        Err(e) => serde_json::json!({
            "delivered": false,
            "upstream_status": match &e {
                DragoniteError::HttpStatus { status, .. } => serde_json::json!(status),
                _ => serde_json::Value::Null,
            },
            "area_count": null,
            "error": e.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_read_answers_the_area_count() {
        let areas = vec![ApiArea::default(), ApiArea::default()];
        assert_eq!(
            dragonite_result(Ok(areas)),
            serde_json::json!({
                "delivered": true,
                "upstream_status": 200,
                "area_count": 2,
                "error": null,
            })
        );
    }

    #[test]
    fn an_upstream_error_answers_its_status_and_message() {
        let answer = dragonite_result(Err(DragoniteError::HttpStatus {
            status: 401,
            detail: Some("bad bearer".to_string()),
            errors: Vec::new(),
            body: String::new(),
        }));
        assert_eq!(answer["delivered"], false);
        assert_eq!(answer["upstream_status"], 401);
        assert_eq!(answer["area_count"], serde_json::Value::Null);
        assert!(answer["error"].as_str().unwrap().contains("bad bearer"));
    }

    #[test]
    fn a_failed_connection_has_no_status() {
        let answer = dragonite_result(Err(DragoniteError::Decode("bad body".to_string())));
        assert_eq!(answer["delivered"], false);
        assert_eq!(answer["upstream_status"], serde_json::Value::Null);
    }
}
