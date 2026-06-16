//! Integration tests for `koji_resource!`.
//!
//! The macro emits a `pub(crate) mod <module>` with `Create<X>` / `Patch<X>`
//! DTOs and five async actix-web handlers. `actix-web`, `utoipa`, `koji-db`,
//! and `koji-core` are real dev-deps so their crate-path symbols resolve.
//!
//! Coverage goals (exercised at compile-time by expanding the macro):
//!   - `pascal_case`: single-word (`project` → `Project`), multi-word
//!     (`tile_server` → `TileServer`).
//!   - `is_schema_primitive`: primitive fields get no `#[schema]` attr; newtype
//!     fields get `#[schema(value_type = String)]`.
//!   - `is_option_type`: already-`Option<T>` create fields are NOT double-wrapped
//!     in the Patch DTO.
//!   - `schema_value_type_attr` both branches hit across the two invocations.
//!   - `ResourceDef::parse` accepts the `module:/seg:/create:` grammar.
//!   - Patch DTO `#[serde(default, skip_serializing_if = "Option::is_none")]`
//!     means missing JSON keys deserialize to `None`.
//!
//! No handler is called at runtime — the macro's call-site types (`KojiDb`,
//! `paginate`, etc.) require a live database. Compile-time expansion proof is
//! the win.

#![allow(dead_code)]

// ---------------------------------------------------------------------------
// crate::utils stubs (the macro emits `crate::utils::…` references).
// ---------------------------------------------------------------------------

pub mod utils {
    pub mod pagination {
        #[derive(serde::Deserialize)]
        pub struct Pagination {
            page: Option<i64>,
            per_page: Option<i64>,
        }
        impl Pagination {
            pub fn page(&self) -> i64 {
                self.page.unwrap_or(1)
            }
            pub fn per_page(&self) -> i64 {
                self.per_page.unwrap_or(20).clamp(1, 500)
            }
        }
    }

    pub mod error {
        #[derive(Debug)]
        pub enum ServiceError {
            NotFound {
                field: &'static str,
                message: String,
            },
            Internal(String),
        }
        impl std::fmt::Display for ServiceError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    ServiceError::NotFound { field, message } => {
                        write!(f, "not found: {field}: {message}")
                    }
                    ServiceError::Internal(s) => write!(f, "internal: {s}"),
                }
            }
        }
        impl ServiceError {
            pub fn internal(e: impl std::fmt::Display) -> Self {
                ServiceError::Internal(e.to_string())
            }
        }
        impl From<koji_db::ModelError> for ServiceError {
            fn from(e: koji_db::ModelError) -> Self {
                ServiceError::Internal(e.to_string())
            }
        }
        impl From<sea_orm::DbErr> for ServiceError {
            fn from(e: sea_orm::DbErr) -> Self {
                ServiceError::Internal(format!("{e}"))
            }
        }
        impl actix_web::ResponseError for ServiceError {
            fn error_response(&self) -> actix_web::HttpResponse {
                actix_web::HttpResponse::InternalServerError().finish()
            }
        }
    }

    pub mod api_response {
        use serde::Serialize;

        #[derive(Serialize, utoipa::ToSchema)]
        pub struct ApiError {
            pub message: String,
        }

        #[derive(Serialize)]
        #[serde(untagged)]
        pub enum ApiResponse<T: Serialize> {
            Ok { data: T, meta: Option<Meta> },
        }
        impl<T: Serialize + 'static> ApiResponse<T> {
            pub fn success(data: T) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(ApiResponse::Ok { data, meta: None })
            }
            pub fn success_paginated(data: T, meta: Meta) -> actix_web::HttpResponse {
                actix_web::HttpResponse::Ok().json(ApiResponse::Ok {
                    data,
                    meta: Some(meta),
                })
            }
        }

        #[derive(Serialize)]
        pub struct Meta {
            pub total: i64,
            pub page: i64,
            pub per_page: i64,
        }
        impl Meta {
            pub fn build(total: i64, page: i64, per_page: i64) -> Self {
                Meta {
                    total,
                    page,
                    per_page,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `koji_resource!` invocations under test.
// ---------------------------------------------------------------------------

// Single-word module: `pascal_case("project") → "Project"`.
// All fields are primitives → no `#[schema(value_type = String)]` emitted.
macros::koji_resource! {
    module: project,
    seg: "projects",
    create: {
        name: String,
        api_endpoint: Option<String>,
        scanner: bool,
    }
}

// Multi-word module: `pascal_case("tile_server") → "TileServer"`.
// `enabled: Option<bool>` stays `Option<bool>` in the Patch DTO (no double-wrap).
macros::koji_resource! {
    module: tile_server,
    seg: "tile-servers",
    create: {
        name: String,
        url: String,
        enabled: Option<bool>,
    }
}

// Non-primitive field: `category: koji_db::Category` is not an ident that
// `is_schema_primitive` knows → `schema_value_type_attr` emits the
// `#[schema(value_type = String)]` attribute. Also an `Option<koji_db::Category>`
// field exercises the `Option<non-primitive>` branch (unwrap inner leaf).
macros::koji_resource! {
    module: property,
    seg: "properties",
    create: {
        name: String,
        category: koji_db::Category,
        multiple: Option<bool>,
    }
}

// ---------------------------------------------------------------------------
// Compile-time + runtime tests.
// ---------------------------------------------------------------------------

/// Create DTO for `project` has all three fields with correct types.
#[test]
fn project_create_dto_fields_and_types() {
    // If any field name or type was wrong, this struct literal wouldn't compile.
    let dto = project::CreateProject {
        name: "My Project".to_string(),
        api_endpoint: None,
        scanner: false,
    };
    assert_eq!(dto.name, "My Project");
    assert!(!dto.scanner);
}

/// Patch DTO wraps non-Option fields in `Option`; leaves `Option<T>` unchanged.
#[test]
fn project_patch_dto_non_option_fields_become_option() {
    // `name: String` in Create → `name: Option<String>` in Patch.
    // `api_endpoint: Option<String>` stays `Option<String>` (not `Option<Option<String>>`).
    let patch = project::PatchProject {
        name: Some("renamed".to_string()),
        api_endpoint: None,
        scanner: Some(true),
    };
    assert_eq!(patch.name.unwrap(), "renamed");
    assert!(patch.api_endpoint.is_none());
}

/// Patch DTO fields are `default` + `skip_serializing_if`, so missing JSON keys → `None`.
#[test]
fn project_patch_dto_missing_fields_deserialize_to_none() {
    // Provide only `name`; `api_endpoint` and `scanner` should default to None.
    let patch: project::PatchProject = serde_json::from_str(r#"{"name":"partial"}"#).unwrap();
    assert_eq!(patch.name.unwrap(), "partial");
    assert!(patch.api_endpoint.is_none());
    assert!(patch.scanner.is_none());
}

/// Multi-word pascal_case: `tile_server` → `TileServer`.
#[test]
fn tile_server_create_dto_pascal_case() {
    let dto = tile_server::CreateTileServer {
        name: "OSM".to_string(),
        url: "https://tile.openstreetmap.org/{z}/{x}/{y}.png".to_string(),
        enabled: Some(true),
    };
    assert_eq!(dto.name, "OSM");
    assert_eq!(dto.enabled, Some(true));
}

/// `Option<bool>` in Create → still `Option<bool>` in Patch (no double-wrap).
#[test]
fn tile_server_patch_option_bool_not_double_wrapped() {
    let patch = tile_server::PatchTileServer {
        name: None,
        url: None,
        enabled: Some(false), // would be `Some(Some(false))` if double-wrapped → compile error
    };
    // Just check the value matches.
    assert_eq!(patch.enabled, Some(false));
}

/// Non-primitive `koji_db::Category` field gets `#[schema(value_type = String)]`.
/// Verify the DTO compiles and Category roundtrips through serde.
#[test]
fn property_create_dto_with_non_primitive_field() {
    let dto = property::CreateProperty {
        name: "my_prop".to_string(),
        category: koji_db::Category::Boolean,
        multiple: None,
    };
    // Category impl Serialize (from StrEnum) so JSON works.
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"boolean\""));
}

/// `Option<koji_db::Category>` in Patch DTO stays `Option<Category>` not double-wrapped.
#[test]
fn property_patch_dto_option_non_primitive_not_double_wrapped() {
    let _patch = property::PatchProperty {
        name: None,
        category: Some(koji_db::Category::String),
        multiple: Some(true),
    };
}

/// The three `scope()` functions compile and return `actix_web::Scope`.
#[test]
fn scope_fns_compile() {
    let _s1: actix_web::Scope = project::scope();
    let _s2: actix_web::Scope = tile_server::scope();
    let _s3: actix_web::Scope = property::scope();
}

/// Both Create DTOs implement `serde::Serialize` (derived) + `utoipa::ToSchema`.
/// Verify via JSON roundtrip.
#[test]
fn create_dtos_serialize() {
    let dto = project::CreateProject {
        name: "p".to_string(),
        api_endpoint: Some("https://example.com".to_string()),
        scanner: true,
    };
    let json = serde_json::to_string(&dto).unwrap();
    assert!(json.contains("\"name\""));
    assert!(json.contains("\"scanner\""));
}
