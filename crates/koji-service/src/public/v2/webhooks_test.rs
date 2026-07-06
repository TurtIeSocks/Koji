//! `POST /api/v2/webhooks/{id}/test` — synchronously fire one subscription with
//! a sample event and report the upstream result inline. Replaces v1's
//! "sync-to-test" workflow; deliberately bypasses the outbox (no retries — the
//! admin is watching the response).

use actix_web::{HttpResponse, web};
use koji_events::WebhookSubscriber;
use koji_events::entity::webhook_subscription;
use koji_events::types::{Event, EventId};
use sea_orm::EntityTrait;

use crate::utils::api_response::{ApiError, ApiResponse};
use crate::utils::error::ServiceError;

/// `POST /api/v2/webhooks/{id}/test` — fire a sample `webhook.test` event at
/// exactly this subscription (mode-aware: signed event POST or legacy ping),
/// synchronously, and report the upstream result. `404` on an unknown id;
/// otherwise always `200` — delivery failure is reported in the body
/// (`delivered: false`, `error: <message>`), not as an HTTP error, since the
/// admin is watching the response inline.
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
