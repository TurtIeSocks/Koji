//! Macro-driven typed CRUD for the v2 plain-JSON resources (`projects`,
//! `properties`, `tile-servers`).
//!
//! These three resources share the exact same koji-db `Query` surface —
//! `paginate` / `get_one` / `get_one_json` / `upsert_json_return` / `delete` —
//! and are not geometry-bearing, so they don't need the `?format=` return-type
//! handling that geofences/routes use. The [`koji_resource!`](macros::koji_resource)
//! macro stamps out, per resource, the typed `Create<Name>` / `Patch<Name>`
//! DTOs, the five REST handlers (list / create / get-one / update / delete),
//! and a [`Scope`](actix_web::Scope) builder, all wired to the v2
//! [`ApiResponse`](crate::utils::api_response::ApiResponse) envelope,
//! `ServiceError` (404 on misses), `201`+`Location`, `204 No Content`, and
//! `?page=&per_page=` pagination.
//!
//! Each resource is emitted into a `<module>`-named submodule (`project` /
//! `property` / `tile_server`) so handler names don't collide; `crate::start`
//! mounts each via `resources::project::scope()` etc.
//!
//! **Wire shape:** the DTOs use **snake_case** field names (the established Koji
//! wire — the frontend types and koji-db's `to_project`/`to_property` readers all
//! use snake; see the field round-trip below). A camelCase rename would diverge
//! from that and silently drop multi-word fields (`api_endpoint`, `api_key`,
//! `default_value`) on write.
//!
//! Geofences and routes are hand-written (see [`super::geofences`] /
//! [`super::routes`]) because their koji-db signatures genuinely differ (they
//! carry geometry and take `ApiQueryArgs` / an `internal` flag).

use macros::koji_resource;

koji_resource! {
    module: project,
    seg: "projects",
    topic: "project",
    create: {
        name: String,
        api_endpoint: Option<String>,
        api_key: Option<String>,
        golbat: bool,
        description: Option<String>,
    }
}

koji_resource! {
    module: property,
    seg: "properties",
    topic: "property",
    create: {
        name: String,
        category: koji_db::Category,
        default_value: Option<String>,
    }
}

koji_resource! {
    module: tile_server,
    seg: "tile-servers",
    topic: "tileserver",
    create: {
        name: String,
        url: String,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---- project ----------------------------------------------------------

    #[test]
    fn create_project_deserializes_full_body() {
        let dto: project::CreateProject = serde_json::from_value(json!({
            "name": "Test",
            "api_endpoint": "http://x",
            "api_key": "k",
            "golbat": true,
            "description": "d"
        }))
        .unwrap();
        assert_eq!(dto.name, "Test");
        assert_eq!(dto.api_endpoint.as_deref(), Some("http://x"));
        assert!(dto.golbat);
    }

    #[test]
    fn create_project_requires_required_fields() {
        // `name` + `golbat` are required (not Option) — a body missing them fails.
        let err = serde_json::from_value::<project::CreateProject>(json!({ "name": "x" }));
        assert!(
            err.is_err(),
            "missing required `golbat` must fail to deserialize"
        );
    }

    #[test]
    fn patch_project_accepts_partial_body_and_omits_none() {
        let dto: project::PatchProject =
            serde_json::from_value(json!({ "description": "only this" })).unwrap();
        assert_eq!(dto.description.as_deref(), Some("only this"));
        assert!(dto.name.is_none());
        assert!(dto.golbat.is_none());
        // Omitted fields are dropped from the serialized upsert value.
        let v = serde_json::to_value(&dto).unwrap();
        assert!(v.get("name").is_none(), "None name omitted");
        assert!(v.get("golbat").is_none(), "None golbat omitted");
        assert_eq!(v["description"], "only this");
    }

    #[test]
    fn create_project_serializes_snake_case_keys() {
        // The serialized value feeds koji-db `to_project`, which reads snake keys.
        let dto = project::CreateProject {
            name: "n".into(),
            api_endpoint: Some("e".into()),
            api_key: Some("k".into()),
            golbat: false,
            description: None,
        };
        let v = serde_json::to_value(&dto).unwrap();
        assert!(
            v.get("api_endpoint").is_some(),
            "snake `api_endpoint` on the wire"
        );
        assert!(v.get("apiEndpoint").is_none(), "not camelCase");
    }

    // ---- property ---------------------------------------------------------

    #[test]
    fn create_property_deserializes_category_string() {
        let dto: property::CreateProperty = serde_json::from_value(json!({
            "name": "p",
            "category": "string",
            "default_value": "hello"
        }))
        .unwrap();
        assert_eq!(dto.name, "p");
        assert_eq!(dto.category, koji_db::Category::String);
        assert_eq!(dto.default_value.as_deref(), Some("hello"));
    }

    #[test]
    fn patch_property_partial_omits_none() {
        let dto: property::PatchProperty =
            serde_json::from_value(json!({ "category": "boolean" })).unwrap();
        assert_eq!(dto.category, Some(koji_db::Category::Boolean));
        assert!(dto.name.is_none());
        let v = serde_json::to_value(&dto).unwrap();
        assert!(v.get("name").is_none());
        assert!(v.get("default_value").is_none());
        assert_eq!(v["category"], "boolean");
    }

    // ---- tile_server ------------------------------------------------------

    #[test]
    fn create_tile_server_deserializes() {
        let dto: tile_server::CreateTileServer = serde_json::from_value(json!({
            "name": "osm",
            "url": "http://tiles"
        }))
        .unwrap();
        assert_eq!(dto.name, "osm");
        assert_eq!(dto.url, "http://tiles");
    }

    #[test]
    fn patch_tile_server_partial() {
        let dto: tile_server::PatchTileServer =
            serde_json::from_value(json!({ "url": "http://new" })).unwrap();
        assert_eq!(dto.url.as_deref(), Some("http://new"));
        assert!(dto.name.is_none());
        let v = serde_json::to_value(&dto).unwrap();
        assert!(v.get("name").is_none());
        assert_eq!(v["url"], "http://new");
    }
}
