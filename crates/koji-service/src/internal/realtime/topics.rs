//! Topic constructors + server-event builders. Mirrors shadmin `topics.ts` +
//! `addEventsForMutations` payloads exactly (contract §4).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEvent {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub meta: Option<serde_json::Value>,
}
impl ServerEvent {
    pub fn new(r#type: impl Into<String>, payload: serde_json::Value) -> Self {
        ServerEvent { r#type: r#type.into(), payload: Some(payload), meta: None }
    }
}
pub fn resource_topic(name: &str) -> String { format!("resource/{name}") }
pub fn record_topic(name: &str, id: impl std::fmt::Display) -> String { format!("resource/{name}/{id}") }
#[allow(dead_code)]
pub fn lock_topic(name: &str) -> String { format!("lock/{name}") }
#[allow(dead_code)]
pub fn lock_record_topic(name: &str, id: impl std::fmt::Display) -> String { format!("lock/{name}/{id}") }
pub fn jobs_topic() -> &'static str { "jobs" }
pub fn job_topic(id: impl std::fmt::Display) -> String { format!("jobs/{id}") }

/// created → collection event only (`{type:"created", payload:{ids:[id]}}`).
pub fn created(name: &str, id: impl std::fmt::Display) -> Vec<(String, ServerEvent)> {
    let id_val: serde_json::Value = id.to_string().parse::<i64>()
        .map(serde_json::Value::from)
        .unwrap_or_else(|_| serde_json::Value::String(id.to_string()));
    vec![(resource_topic(name), ServerEvent::new("created", serde_json::json!({"ids":[id_val]})))]
}
/// updated → record event then collection event.
pub fn updated(name: &str, id: i64, data: serde_json::Value) -> Vec<(String, ServerEvent)> {
    vec![
        (record_topic(name, id), ServerEvent::new("updated", serde_json::json!({"id":id, "data":data}))),
        (resource_topic(name), ServerEvent::new("updated", serde_json::json!({"ids":[id]}))),
    ]
}
/// deleted → record event then collection event.
pub fn deleted(name: &str, id: i64) -> Vec<(String, ServerEvent)> {
    vec![
        (record_topic(name, id), ServerEvent::new("deleted", serde_json::json!({"id":id}))),
        (resource_topic(name), ServerEvent::new("deleted", serde_json::json!({"ids":[id]}))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topic_strings_match_contract() {
        assert_eq!(resource_topic("geofence"), "resource/geofence");
        assert_eq!(record_topic("geofence", 7), "resource/geofence/7");
        assert_eq!(lock_topic("route"), "lock/route");
        assert_eq!(lock_record_topic("route", 3), "lock/route/3");
        assert_eq!(jobs_topic(), "jobs");
        assert_eq!(job_topic("01J..."), "jobs/01J...");
    }
    #[test]
    fn created_emits_collection_only() {
        let pairs = created("geofence", 5i64);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "resource/geofence");
        assert_eq!(pairs[0].1.r#type, "created");
        assert_eq!(pairs[0].1.payload.as_ref().unwrap()["ids"], serde_json::json!([5]));
    }
    #[test]
    fn updated_emits_record_then_collection() {
        let pairs = updated("route", 9, serde_json::json!({"id":9,"name":"x"}));
        assert_eq!(pairs[0].0, "resource/route/9");
        assert_eq!(pairs[0].1.r#type, "updated");
        assert_eq!(pairs[0].1.payload.as_ref().unwrap()["id"], 9);
        assert_eq!(pairs[0].1.payload.as_ref().unwrap()["data"]["name"], "x");
        assert_eq!(pairs[1].0, "resource/route");
        assert_eq!(pairs[1].1.payload.as_ref().unwrap()["ids"], serde_json::json!([9]));
    }
    #[test]
    fn deleted_emits_record_then_collection() {
        let pairs = deleted("project", 2);
        assert_eq!(pairs[0].0, "resource/project/2");
        assert_eq!(pairs[0].1.r#type, "deleted");
        assert_eq!(pairs[0].1.payload.as_ref().unwrap()["id"], 2);
        assert_eq!(pairs[1].0, "resource/project");
        assert_eq!(pairs[1].1.payload.as_ref().unwrap()["ids"], serde_json::json!([2]));
    }
}
