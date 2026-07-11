//! Dragonite event subscriber + the domain-event payloads it consumes.
//!
//! This is the composition-root half of the events→Dragonite path (architecture
//! §7). The generic outbox/dispatcher/`Subscriber` machinery lives in
//! `koji-events` (which knows nothing of Dragonite); the typed `/v2/areas`
//! client lives in `koji-dragonite`. [`DragoniteSubscriber`] joins them: it
//! claims `area.*` events and PATCHes the linked Dragonite area.
//!
//! ## Payload contract
//! Producers (the `POST /geofences/:id/publish` endpoint and calc persist=push)
//! resolve the Dragonite linkage (`dragonite_area_id` + the target
//! [`AreaMode`]) and emit a self-contained payload. The subscriber is therefore
//! stateless — it never reads the DB — and just maps the payload onto a PATCH:
//!
//! - [`TOPIC_ROUTE_UPDATED`] → [`RouteUpdated`] → `area_route_patch`
//! - [`TOPIC_GEOFENCE_UPDATED`] → [`GeofenceUpdated`] → `area_geofence_patch`

use async_trait::async_trait;
use geojson::Feature;
use koji_core::SingleVec;
use koji_dragonite::{
    AreaMode, DragoniteClient, DragoniteError, area_geofence_patch, area_route_patch,
};
use koji_events::{DeliverError, Event, Subscriber};
use serde::{Deserialize, Serialize};

/// Topic for "a mode's route was (re)calculated" — payload [`RouteUpdated`].
pub(crate) const TOPIC_ROUTE_UPDATED: &str = "area.route_updated";
/// Topic for "a mode's geofence was published/updated" — payload [`GeofenceUpdated`].
pub(crate) const TOPIC_GEOFENCE_UPDATED: &str = "area.geofence_updated";

/// Payload for [`TOPIC_ROUTE_UPDATED`]. `route` is a Koji [`SingleVec`]
/// (`[lat, lon]` points). `Base` has no route slot, so the subscriber drops it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RouteUpdated {
    pub dragonite_area_id: i64,
    pub mode: AreaMode,
    pub route: SingleVec,
}

/// Payload for [`TOPIC_GEOFENCE_UPDATED`]. `geofence` is the GeoJSON `Feature`
/// to set on `mode`'s fence (`Base` → the area-root fence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GeofenceUpdated {
    pub dragonite_area_id: i64,
    pub mode: AreaMode,
    pub geofence: Feature,
}

/// Pushes Koji's calculated routes/geofences to Dragonite's `/v2/areas/{id}`.
pub(crate) struct DragoniteSubscriber {
    client: DragoniteClient,
}

impl DragoniteSubscriber {
    /// Build the subscriber over a configured Dragonite client.
    pub(crate) fn new(client: DragoniteClient) -> Self {
        Self { client }
    }
}

/// Map a [`DragoniteError`] onto the subscriber-side [`DeliverError`] so the
/// dispatcher's backoff/dead-letter logic sees the right category. A non-2xx
/// without a parseable envelope surfaces its typed HTTP status as
/// [`DeliverError::Status`]; everything else is `Other`.
fn to_deliver_error(e: DragoniteError) -> DeliverError {
    match e {
        DragoniteError::Http(err) => DeliverError::Http(err.to_string()),
        DragoniteError::HttpStatus { status, .. } => DeliverError::Status { status },
        DragoniteError::Api { message, .. } => DeliverError::Other(message),
        DragoniteError::Decode(m) => DeliverError::Other(format!("decode: {m}")),
    }
}

#[async_trait]
impl Subscriber for DragoniteSubscriber {
    fn name(&self) -> &'static str {
        "dragonite"
    }

    fn interested_in(&self, topic: &str) -> bool {
        topic == TOPIC_ROUTE_UPDATED || topic == TOPIC_GEOFENCE_UPDATED
    }

    async fn deliver(&self, event: &Event) -> Result<(), DeliverError> {
        match event.topic.as_str() {
            TOPIC_ROUTE_UPDATED => {
                let p: RouteUpdated =
                    serde_json::from_value(event.payload.clone()).map_err(|e| {
                        DeliverError::Other(format!("invalid {TOPIC_ROUTE_UPDATED} payload: {e}"))
                    })?;
                let patch = area_route_patch(p.mode, &p.route);
                self.client
                    .patch_area(p.dragonite_area_id, &patch)
                    .await
                    .map_err(to_deliver_error)?;
                Ok(())
            }
            TOPIC_GEOFENCE_UPDATED => {
                let p: GeofenceUpdated =
                    serde_json::from_value(event.payload.clone()).map_err(|e| {
                        DeliverError::Other(format!(
                            "invalid {TOPIC_GEOFENCE_UPDATED} payload: {e}"
                        ))
                    })?;
                let patch = area_geofence_patch(p.mode, &p.geofence);
                self.client
                    .patch_area(p.dragonite_area_id, &patch)
                    .await
                    .map_err(to_deliver_error)?;
                Ok(())
            }
            // interested_in gates the topics above; anything else is a no-op.
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use koji_dragonite::DragoniteError;

    #[test]
    fn route_payload_round_trips() {
        let p = RouteUpdated {
            dragonite_area_id: 42,
            mode: AreaMode::Quest,
            route: vec![[40.1, -75.2], [40.3, -75.4]],
        };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["dragonite_area_id"], 42);
        assert_eq!(v["mode"], "quest");
        let back: RouteUpdated = serde_json::from_value(v).unwrap();
        assert_eq!(back.route.len(), 2);
        assert_eq!(back.dragonite_area_id, 42);
    }

    #[test]
    fn http_status_error_maps_to_status() {
        let e = DragoniteError::HttpStatus {
            status: 404,
            body: "not found".to_string(),
        };
        assert!(matches!(
            to_deliver_error(e),
            DeliverError::Status { status: 404 }
        ));
    }

    #[test]
    fn non_http_api_error_maps_to_other() {
        let e = DragoniteError::Api {
            code: Some("invalid_json".to_string()),
            message: "bad body".to_string(),
            field: None,
        };
        assert!(matches!(to_deliver_error(e), DeliverError::Other(_)));
    }

    #[test]
    fn interested_only_in_area_topics() {
        let s = DragoniteSubscriber::new(DragoniteClient::new("http://x", "t"));
        assert!(s.interested_in(TOPIC_ROUTE_UPDATED));
        assert!(s.interested_in(TOPIC_GEOFENCE_UPDATED));
        assert!(!s.interested_in("webhook.test"));
    }
}
