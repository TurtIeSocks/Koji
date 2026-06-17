# v2 API — Phase 0: Response & Error Machinery — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the shared response/error primitives the rest of the v2 redesign compiles against — a `ServiceError` that maps to correct HTTP status codes, offset pagination, `?format=` negotiation (enveloped GeoJSON vs raw exports), and a code-first OpenAPI scaffold — with **zero endpoint behavior change** in this phase.

**Architecture:** Pure helper modules under `crates/koji-service/src/utils/`, each independently unit-tested. They wrap and extend the existing `ApiResponse<T>` envelope (`utils/api_response.rs`) and the existing `response_body` export-format serializer (`utils/response.rs`) rather than reinventing them. No handlers are rewired here; later phases (P1–P4) consume these primitives.

**Tech Stack:** Rust, actix-web 4, sea-orm, thiserror, utoipa (new), serde. Workspace crate `koji-service`.

**Scope note — the `#[koji_resource]` proc-macro is NOT in this plan.** The spec (§6.1) places it in P0, but it generates per-resource Create/Patch DTOs from each sea-orm `Model`, so it needs every resource Model read first. It gets its own plan written immediately before P2 (CRUD regeneration), where those Models are in scope. This phase delivers the primitives that macro (and the hand-written P1/P3/P4 handlers) will use.

**Verification rules (from project memory):**
- `koji-wasm` is **not** built with `--target wasm32` (TLS-link false-failure trap). `koji-service` is host-only anyway.
- Run lint + tests in parallel where possible. Background any command > 5s.
- These are `--lib` unit tests for `koji-service` — **no database required**.

---

## File Structure

| File | Responsibility | Action |
|---|---|---|
| `crates/koji-service/src/utils/api_response.rs` | v2 envelope (`ApiResponse`, `ApiError`, `Meta`, `code_for_status`) | **Modify** — expose `code_for_status`; add `success_paginated`; add `Meta::build` |
| `crates/koji-service/src/utils/response.rs` | legacy `Response` + `response_body` export serializer | **Modify** — expose `response_body` as `pub(crate)` |
| `crates/koji-service/src/utils/error.rs` | `ServiceError` + `ResponseError` → status mapping | **Create** |
| `crates/koji-service/src/utils/pagination.rs` | `Pagination` query (`?limit=&offset=`) + `Meta::build` | **Create** |
| `crates/koji-service/src/utils/format.rs` | `respond_geo` — `?format=` enveloped-vs-raw negotiation | **Create** |
| `crates/koji-service/src/utils/openapi.rs` | code-first OpenAPI scaffold (`ApiDoc`) | **Create** |
| `crates/koji-service/src/utils/mod.rs` | utils module index | **Modify** — declare the 4 new submodules |
| `crates/koji-service/Cargo.toml` | deps | **Modify** — ensure `thiserror`; add `utoipa` |

---

## Task 1: `ServiceError` + `ResponseError`

Replaces the scattered `.map_err(actix_web::error::ErrorInternalServerError)` with a typed error that renders the v2 error envelope at the right status. The mapping is a **pure function** (`to_api_error`) so it unit-tests without spinning up actix; `ResponseError` is a thin wrapper.

**Files:**
- Modify: `crates/koji-service/src/utils/api_response.rs` (expose `code_for_status`)
- Modify: `crates/koji-service/src/Cargo.toml`-adjacent `crates/koji-service/Cargo.toml` (ensure `thiserror`)
- Create: `crates/koji-service/src/utils/error.rs`
- Modify: `crates/koji-service/src/utils/mod.rs`

- [ ] **Step 1: Expose `code_for_status`**

In `crates/koji-service/src/utils/api_response.rs`, change the helper's visibility:

```rust
/// Derive a stable error `code` from the HTTP status.
pub(crate) fn code_for_status(status: StatusCode) -> String {
```

(Only the `fn` keyword line changes: `fn code_for_status` → `pub(crate) fn code_for_status`.)

- [ ] **Step 2: Ensure `thiserror` is a dependency**

In `crates/koji-service/Cargo.toml`, confirm `[dependencies]` contains `thiserror`. If absent, add:

```toml
thiserror = { workspace = true }
```

Run: `cargo metadata --format-version 1 -q | rg -q '"name":"thiserror"' && echo OK`
Expected: `OK` (after adding if needed).

- [ ] **Step 3: Write the failing test**

Create `crates/koji-service/src/utils/error.rs`:

```rust
//! `ServiceError` — the koji-service error type that maps cleanly to the v2
//! response envelope. Handlers return `Result<HttpResponse, ServiceError>`;
//! actix renders the error via `ResponseError` into
//! `{ "status": "error", "error": { code, message, field? } }` at the right HTTP
//! status. Replaces the scattered
//! `.map_err(actix_web::error::ErrorInternalServerError)` calls.

use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use koji_db::ModelError;
use sea_orm::DbErr;
use thiserror::Error;

use crate::utils::api_response::{ApiError, ApiResponse, code_for_status};

#[derive(Debug, Error)]
pub(crate) enum ServiceError {
    #[error("{message}")]
    NotFound {
        field: &'static str,
        message: String,
    },
    #[error("{message}")]
    Invalid {
        field: Option<String>,
        message: String,
    },
    #[error("{message}")]
    Unprocessable {
        field: Option<String>,
        message: String,
    },
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Db(#[from] DbErr),
    #[error(transparent)]
    Model(#[from] ModelError),
}

impl ServiceError {
    /// Pure mapping to `(status, error-object)`. Unit-tested directly; the
    /// `ResponseError` impl is a thin wrapper over this. Internal failures
    /// (`Db`/`Model`) are logged and surface a generic message — never leaking
    /// their detail to the client.
    pub(crate) fn to_api_error(&self) -> (StatusCode, ApiError) {
        let (status, field, message) = match self {
            ServiceError::NotFound { field, message } => {
                (StatusCode::NOT_FOUND, Some((*field).to_string()), message.clone())
            }
            ServiceError::Invalid { field, message } => {
                (StatusCode::BAD_REQUEST, field.clone(), message.clone())
            }
            ServiceError::Unprocessable { field, message } => {
                (StatusCode::UNPROCESSABLE_ENTITY, field.clone(), message.clone())
            }
            ServiceError::Conflict(message) => (StatusCode::CONFLICT, None, message.clone()),
            ServiceError::Db(e) => {
                log::error!("service db error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, None, "internal error".to_string())
            }
            ServiceError::Model(e) => {
                log::error!("service model error: {e}");
                (StatusCode::INTERNAL_SERVER_ERROR, None, "internal error".to_string())
            }
        };
        (status, ApiError { code: code_for_status(status), message, field })
    }
}

impl ResponseError for ServiceError {
    fn status_code(&self) -> StatusCode {
        self.to_api_error().0
    }
    fn error_response(&self) -> HttpResponse {
        let (status, error) = self.to_api_error();
        HttpResponse::build(status).json(ApiResponse::<()>::Error { error })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_maps_to_404_with_field() {
        let (status, err) = ServiceError::NotFound {
            field: "geofence",
            message: "no geofence 7".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(err.code, "not_found");
        assert_eq!(err.field.as_deref(), Some("geofence"));
        assert_eq!(err.message, "no geofence 7");
    }

    #[test]
    fn unprocessable_maps_to_422() {
        let (status, err) = ServiceError::Unprocessable {
            field: Some("dragonite_area_id".into()),
            message: "geofence is not linked".into(),
        }
        .to_api_error();
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(err.code, "unprocessable");
        assert_eq!(err.field.as_deref(), Some("dragonite_area_id"));
    }

    #[test]
    fn conflict_maps_to_409() {
        let (status, err) = ServiceError::Conflict("already canceled".into()).to_api_error();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(err.code, "conflict");
        assert!(err.field.is_none());
    }

    #[test]
    fn db_error_is_500_generic_and_does_not_leak() {
        let (status, err) = ServiceError::Db(DbErr::Custom("secret internal detail".into()))
            .to_api_error();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.code, "internal_error");
        assert_eq!(err.message, "internal error");
        assert!(!err.message.contains("secret"), "internal detail must not leak");
    }

    #[test]
    fn from_dberr_via_question_mark() {
        fn boom() -> Result<(), ServiceError> {
            Err(DbErr::Custom("x".into()))?;
            Ok(())
        }
        assert!(matches!(boom(), Err(ServiceError::Db(_))));
    }
}
```

Then declare the module — in `crates/koji-service/src/utils/mod.rs` add:

```rust
pub(crate) mod error;
```

- [ ] **Step 4: Run test to verify it fails**

Run: `cargo test -p koji-service --lib utils::error::tests 2>&1 | tail -20`
Expected: compile error or FAIL — `error.rs` references `ApiResponse::<()>::Error` / `code_for_status` which must resolve (if Step 1 was skipped it won't compile).

- [ ] **Step 5: Make it pass**

The implementation IS the code in Step 3 (test + impl land together for a new module). Ensure Steps 1–2 (expose `code_for_status`, `thiserror` dep) are done.

Run: `cargo test -p koji-service --lib utils::error::tests 2>&1 | tail -20`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 6: Commit**

```bash
git add crates/koji-service/src/utils/error.rs crates/koji-service/src/utils/mod.rs crates/koji-service/src/utils/api_response.rs crates/koji-service/Cargo.toml
git commit -m "feat(koji-service): ServiceError + ResponseError → v2 error envelope"
```

---

## Task 2: Offset pagination

`Pagination` parses `?limit=&offset=` (clamped: default 50, max 500, offset ≥ 0). `Meta::build` computes the `meta` block. `ApiResponse::success_paginated` emits an `ok` envelope carrying that `meta`.

**Files:**
- Modify: `crates/koji-service/src/utils/api_response.rs` (add `success_paginated`)
- Create: `crates/koji-service/src/utils/pagination.rs`
- Modify: `crates/koji-service/src/utils/mod.rs`

- [ ] **Step 1: Add `success_paginated` to the envelope**

In `crates/koji-service/src/utils/api_response.rs`, inside `impl<T: Serialize> ApiResponse<T>` (next to `success`):

```rust
    /// `200 OK` success carrying `data` plus a pagination `meta` block.
    pub(crate) fn success_paginated(data: T, meta: Meta) -> HttpResponse {
        HttpResponse::build(StatusCode::OK).json(ApiResponse::Ok { data, meta: Some(meta) })
    }
```

- [ ] **Step 2: Write the failing test**

Create `crates/koji-service/src/utils/pagination.rs`:

```rust
//! Offset pagination for v2 list endpoints. `Pagination` parses `?limit=&offset=`
//! (clamped); `Meta::build` computes the `meta` block the envelope carries.

use serde::Deserialize;

use crate::utils::api_response::Meta;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 500;

/// `?limit=&offset=` query for list endpoints. Both optional; access the
/// effective (clamped) values via [`Pagination::limit`] / [`Pagination::offset`].
#[derive(Debug, Default, Deserialize)]
pub(crate) struct Pagination {
    limit: Option<i64>,
    offset: Option<i64>,
}

impl Pagination {
    /// Effective limit: defaults to 50, clamped to `[1, 500]`.
    pub(crate) fn limit(&self) -> i64 {
        self.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)
    }

    /// Effective offset: defaults to 0, never negative.
    pub(crate) fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

impl Meta {
    /// Build the pagination `meta` from the total row count and the effective
    /// `limit`/`offset` (both already clamped via [`Pagination`]). `page` is a
    /// 0-based page index.
    pub(crate) fn build(total: i64, limit: i64, offset: i64) -> Meta {
        let per_page = limit.max(1);
        let page = offset / per_page;
        let total_pages = if total == 0 {
            0
        } else {
            (total + per_page - 1) / per_page
        };
        Meta {
            total,
            page,
            per_page,
            total_pages,
            has_next: offset + per_page < total,
            has_prev: offset > 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_defaults_and_clamps() {
        assert_eq!(Pagination::default().limit(), 50);
        assert_eq!(Pagination { limit: Some(9999), offset: None }.limit(), 500);
        assert_eq!(Pagination { limit: Some(0), offset: None }.limit(), 1);
    }

    #[test]
    fn offset_defaults_and_floors_at_zero() {
        assert_eq!(Pagination::default().offset(), 0);
        assert_eq!(Pagination { limit: None, offset: Some(-5) }.offset(), 0);
        assert_eq!(Pagination { limit: None, offset: Some(40) }.offset(), 40);
    }

    #[test]
    fn meta_first_page_small_total() {
        let m = Meta::build(2, 50, 0);
        assert_eq!(m.total, 2);
        assert_eq!(m.page, 0);
        assert_eq!(m.per_page, 50);
        assert_eq!(m.total_pages, 1);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }

    #[test]
    fn meta_middle_page() {
        let m = Meta::build(120, 50, 50);
        assert_eq!(m.page, 1);
        assert_eq!(m.total_pages, 3);
        assert!(m.has_next);
        assert!(m.has_prev);
    }

    #[test]
    fn meta_empty() {
        let m = Meta::build(0, 50, 0);
        assert_eq!(m.total_pages, 0);
        assert!(!m.has_next);
        assert!(!m.has_prev);
    }
}
```

Declare the module — in `crates/koji-service/src/utils/mod.rs` add:

```rust
pub(crate) mod pagination;
```

- [ ] **Step 3: Run test to verify it fails / passes**

Run: `cargo test -p koji-service --lib utils::pagination::tests 2>&1 | tail -20`
Expected: `test result: ok. 5 passed` (test + impl land together; the failure mode here is a compile error if `Meta`'s fields aren't reachable — they are `pub` within the crate).

- [ ] **Step 4: Commit**

```bash
git add crates/koji-service/src/utils/pagination.rs crates/koji-service/src/utils/api_response.rs crates/koji-service/src/utils/mod.rs
git commit -m "feat(koji-service): offset pagination (?limit/&offset) + Meta::build"
```

---

## Task 3: `?format=` negotiation (`respond_geo`)

Geometry-bearing reads return enveloped GeoJSON by default; explicit export formats (`sql`, `poracle`, `text`, `array`, …) return a **raw** body for drop-in golbat-tool compatibility. Reuses the existing `response_body` serializer.

**Files:**
- Modify: `crates/koji-service/src/utils/response.rs` (expose `response_body`)
- Create: `crates/koji-service/src/utils/format.rs`
- Modify: `crates/koji-service/src/utils/mod.rs`

- [ ] **Step 1: Expose `response_body`**

In `crates/koji-service/src/utils/response.rs`, change:

```rust
pub(crate) fn response_body(coll: &KojiGeometryCollection, return_type: ReturnTypeArg) -> JsonValue {
```

(Only the `fn` line: `fn response_body` → `pub(crate) fn response_body`.)

- [ ] **Step 2: Write the failing test**

Create `crates/koji-service/src/utils/format.rs`:

```rust
//! `?format=` negotiation for geometry-bearing reads. The default (geojson)
//! shapes ride inside the v2 envelope; the explicit export formats are returned
//! raw (no envelope) so they stay drop-in compatible with golbat tooling.
//! Serialization itself is delegated to [`response_body`].

use actix_web::{HttpResponse, http::StatusCode};
use koji_core::KojiGeometryCollection;

use crate::requests::ReturnTypeArg;
use crate::utils::api_response::ApiResponse;
use crate::utils::response::response_body;

/// `true` for the GeoJSON shapes that ride inside the v2 envelope; `false` for
/// the raw export formats returned bare for golbat-tool compatibility.
pub(crate) fn is_enveloped(rt: &ReturnTypeArg) -> bool {
    matches!(
        rt,
        ReturnTypeArg::Feature | ReturnTypeArg::FeatureCollection | ReturnTypeArg::Geometry
    )
}

/// Render a geometry collection per the negotiated return type: enveloped
/// GeoJSON for the default shapes, a raw body for the export formats (`sql` as
/// `text/plain`; the rest as bare JSON).
pub(crate) fn respond_geo(coll: KojiGeometryCollection, rt: ReturnTypeArg) -> HttpResponse {
    let body = response_body(&coll, rt.clone());
    if is_enveloped(&rt) {
        ApiResponse::success(body)
    } else if matches!(rt, ReturnTypeArg::Sql) {
        let sql = body.as_str().unwrap_or_default().to_string();
        HttpResponse::build(StatusCode::OK)
            .content_type("text/plain; charset=utf-8")
            .body(sql)
    } else {
        HttpResponse::build(StatusCode::OK).json(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{LineString, coord};
    use koji_core::{KojiGeometry, KojiGeometryCollection};

    fn coll() -> KojiGeometryCollection {
        let line = LineString::from(vec![coord! {x:1.0,y:2.0}, coord! {x:3.0,y:4.0}]);
        KojiGeometryCollection::new(vec![KojiGeometry::new(line)])
    }

    #[test]
    fn geojson_shapes_are_enveloped_exports_are_not() {
        assert!(is_enveloped(&ReturnTypeArg::FeatureCollection));
        assert!(is_enveloped(&ReturnTypeArg::Feature));
        assert!(is_enveloped(&ReturnTypeArg::Geometry));
        assert!(!is_enveloped(&ReturnTypeArg::Sql));
        assert!(!is_enveloped(&ReturnTypeArg::Poracle));
        assert!(!is_enveloped(&ReturnTypeArg::SingleArray));
    }

    #[test]
    fn default_geojson_serves_json() {
        let resp = respond_geo(coll(), ReturnTypeArg::FeatureCollection);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"), "got {ct}");
    }

    #[test]
    fn sql_is_raw_text_plain() {
        let resp = respond_geo(coll(), ReturnTypeArg::Sql);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.starts_with("text/plain"), "got {ct}");
    }

    #[test]
    fn poracle_is_raw_json() {
        let resp = respond_geo(coll(), ReturnTypeArg::Poracle);
        assert_eq!(resp.status(), StatusCode::OK);
        let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
        assert!(ct.contains("application/json"), "got {ct}");
    }
}
```

Declare the module — in `crates/koji-service/src/utils/mod.rs` add:

```rust
pub(crate) mod format;
```

- [ ] **Step 3: Run test**

Run: `cargo test -p koji-service --lib utils::format::tests 2>&1 | tail -20`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-service/src/utils/format.rs crates/koji-service/src/utils/response.rs crates/koji-service/src/utils/mod.rs
git commit -m "feat(koji-service): respond_geo — ?format= enveloped-vs-raw negotiation"
```

---

## Task 4: Code-first OpenAPI scaffold

Add `utoipa` and a minimal `ApiDoc` that compiles. Per-phase work later annotates handlers/DTOs and registers them in `paths(...)` / `components(...)`; this is just the scaffold so those phases have a home to extend.

**Files:**
- Modify: `crates/koji-service/Cargo.toml` (add `utoipa`)
- Create: `crates/koji-service/src/utils/openapi.rs`
- Modify: `crates/koji-service/src/utils/mod.rs`

- [ ] **Step 1: Add the dependency**

In `crates/koji-service/Cargo.toml` `[dependencies]`:

```toml
utoipa = { version = "5", features = ["actix_extras"] }
```

Run: `cargo build -p koji-service 2>&1 | tail -5`
Expected: builds (downloads `utoipa`).

- [ ] **Step 2: Write the failing test**

Create `crates/koji-service/src/utils/openapi.rs`:

```rust
//! Code-first OpenAPI document. Per-phase work annotates handlers with
//! `#[utoipa::path(...)]` and DTOs with `#[derive(ToSchema)]`, then registers
//! them in the `paths(...)` / `components(...)` lists below. This is the scaffold
//! (title + version) those phases extend; it is served at
//! `GET /api/v2/openapi.yaml`, replacing the hand-maintained `openapi.yaml`.

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(info(
    title = "Koji v2 API",
    version = "2.0.0",
    description = "The /api/v2 surface of Koji — job-queue calc, typed CRUD, geometry utilities."
))]
pub(crate) struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_builds_with_title_and_version() {
        let doc = ApiDoc::openapi();
        assert_eq!(doc.info.title, "Koji v2 API");
        assert_eq!(doc.info.version, "2.0.0");
    }
}
```

Declare the module — in `crates/koji-service/src/utils/mod.rs` add:

```rust
pub(crate) mod openapi;
```

- [ ] **Step 3: Run test**

Run: `cargo test -p koji-service --lib utils::openapi::tests 2>&1 | tail -20`
Expected: `test result: ok. 1 passed`.

If the `#[openapi(info(...))]` shorthand is rejected by the installed utoipa version, use the nested form `info(...)` exactly as written above (utoipa 5 accepts the `info(title=…, version=…, description=…)` list). Do not add `paths`/`components` yet — empty registries are the point of the scaffold.

- [ ] **Step 4: Commit**

```bash
git add crates/koji-service/src/utils/openapi.rs crates/koji-service/src/utils/mod.rs crates/koji-service/Cargo.toml Cargo.lock
git commit -m "feat(koji-service): code-first OpenAPI scaffold (utoipa ApiDoc)"
```

---

## Task 5: Phase verification gate

Confirm the whole crate is green and the new primitives are wired, with no behavior change to existing endpoints.

**Files:** none (verification only).

- [ ] **Step 1: Confirm the utils module index**

`crates/koji-service/src/utils/mod.rs` should now declare all four new submodules (added across Tasks 1–4):

```rust
pub(crate) mod error;
pub(crate) mod format;
pub(crate) mod openapi;
pub(crate) mod pagination;
```

(Plus the pre-existing `api_response`, `response`, and any others — leave those as-is.)

- [ ] **Step 2: Lint + full crate test in parallel (background if > 5s)**

Run (single batch):
```bash
cargo clippy -p koji-service --all-targets 2>&1 | tail -15
cargo test -p koji-service --lib 2>&1 | tail -15
```
Expected: clippy clean (no new warnings); all `koji-service` lib tests pass, including the pre-existing `utils::api_response` and `utils::response` golden tests (proving no regression) plus the 15 new tests from Tasks 1–4.

- [ ] **Step 3: Confirm no endpoint behavior changed**

Run: `git diff --stat HEAD~4 -- crates/koji-service/src/public crates/koji-service/src/lib.rs`
Expected: **empty** — Phase 0 touches only `utils/*` and `Cargo.toml`, never handlers or routing.

- [ ] **Step 4: Commit (if Step 1 required an edit; else skip)**

```bash
git add crates/koji-service/src/utils/mod.rs
git commit -m "chore(koji-service): finalize utils module index for v2 machinery"
```

---

## Self-Review

**Spec coverage (spec §6):**
- §6.2 `ServiceError` + `ResponseError` → Task 1. ✓
- §6.3 Pagination (`?limit/&offset` + `meta`) → Task 2. ✓
- §6.4 `?format=` negotiation (enveloped GeoJSON vs raw exports) → Task 3. ✓
- §6.5 Code-first OpenAPI → Task 4 (scaffold; per-endpoint annotation lands P1–P4). ✓
- §6.1 `#[koji_resource]` macro → **deferred to its own plan before P2** (needs Models read). Documented in the header. ✓
- §2 envelope, §6 helpers do not alter handlers → Task 5 Step 3 asserts no `public/` diff. ✓

**Placeholder scan:** none — every step carries complete code or an exact command + expected output. The one conditional (Task 4 Step 3 utoipa shorthand) names the exact fallback, not a TODO.

**Type consistency:** `ApiError { code, message, field }`, `Meta { total, page, per_page, total_pages, has_next, has_prev }`, `ApiResponse::<()>::Error { error }`, `code_for_status(StatusCode) -> String`, `response_body(&KojiGeometryCollection, ReturnTypeArg) -> JsonValue`, `ReturnTypeArg::{Feature, FeatureCollection, Geometry, Sql, Poracle, SingleArray}` — all match the names read from `api_response.rs`, `response.rs`, and `requests/config.rs`. `Pagination` fields are private and only touched by same-module tests.

**Downstream contract for later phases:** handlers will return `Result<HttpResponse, ServiceError>`; lists call `ApiResponse::success_paginated(rows, Meta::build(total, p.limit(), p.offset()))`; geometry reads call `respond_geo(coll, rt)`. The db-layer `NotFound` signal that maps to `ServiceError::NotFound` (→404) is wired in P2 when CRUD is regenerated.
