# Koji Admin V2 Slice 2 — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship route + project + property + tileserver + plugins resources in the Koji Admin V2 frontend, plus the `GET /internal/routes` backend row-list that powers the route resource's ListLive view.

**Architecture:** Mirror the shipped geofence end-to-end pattern exactly — backend `internal/<resource>.rs` bespoke row-list wired into `internal::scope()` + `v2::<resource>::internal_item_scope()` pattern; frontend resource directories under `apps/web/src/resources/<name>/` with list/edit/create/show files + browser tests that stub the dataProvider. `PropertyValueInput` is a standalone input component that switches on a sibling `category` field via `useWatch`. No new dataProvider logic is needed — `RESOURCE_MAP` already has all six resources; `route` just needs a `geo: true` entry added.

**Tech Stack:** Rust / actix-web / SeaORM (backend); React 19 / ra-core 5.14 (alias `shadmin-core`) / react-hook-form `useWatch` / vitest-browser-react / bun (frontend).

## Global Constraints

- Package manager: **bun** only. No npm/yarn/pnpm.
- Frontend root: `apps/web/`. All TS paths use `@/` alias (= `apps/web/src/`).
- Tailwind v4; shadmin (ra-core 5.14) vendored under `src/components/admin/`.
- `ColorInput` lives at `@/components/admin/inputs/` — does NOT exist yet; must be vendored from `shadcn-admin-kit` in Task 2 as a prerequisite for `PropertyValueInput`.
- `MonacoJsonInput` lives in the shadcn-admin-kit repo. It requires `monaco-editor` which is NOT in Koji web's `package.json` — Task 2 must add the dep and vendor the component before any task uses it.
- dataProvider hits `/internal` only. Writes forward to public CRUD. All six resources are already in `RESOURCE_MAP`; `route` entry is missing `geo: true` (add in Task 3).
- Realtime: `ListLive` / `EditLive` / `ShowLive` from `@/components/realtime`. Browser tests stub `subscribe: () => () => undefined` on the dataProvider.
- Browser tests use `vitest-browser-react` / Chromium (~100s cold boot). Run with `bun run test:browser` in `apps/web/`. Use `run_in_background: true` for these commands.
- Unit tests use `bun run test` in `apps/web/`.
- DB-gated Rust tests gate on `KOJI_DB_URL`; skip gracefully when unset (see `serial_guard()` pattern in `internal_geofences_rows.rs`).
- TDD, bite-sized steps, conventional commits (`feat:` / `test:` / `chore:`), branch `claude/v2`.
- Every commit includes `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.

---

## File Structure

### Created

| File | Responsibility |
|------|---------------|
| `crates/koji-service/src/internal/routes.rs` | `GET /internal/routes` bespoke row-list handler |
| `crates/koji-service/tests/internal_routes_rows.rs` | DB-gated integration test for route row-list |
| `apps/web/src/components/admin/inputs/color-input.tsx` | Vendored `ColorInput` from shadcn-admin-kit |
| `apps/web/src/components/inputs/property-value-input.tsx` | New `PropertyValueInput` component (category-switched) |
| `apps/web/src/components/inputs/property-value-input.browser.test.tsx` | Browser test for `PropertyValueInput` |
| `apps/web/src/resources/route/route-list.tsx` | Route `ListLive` + DataTable |
| `apps/web/src/resources/route/route-edit.tsx` | Route `EditLive` + `SimpleForm` |
| `apps/web/src/resources/route/route-create.tsx` | Route `Create` + form fields |
| `apps/web/src/resources/route/route-show.tsx` | Route `ShowLive` |
| `apps/web/src/resources/route/route-list.browser.test.tsx` | Browser test for `RouteList` |
| `apps/web/src/resources/route/route-form.browser.test.tsx` | Browser test for route edit/create (MultiPointInput) |
| `apps/web/src/resources/route/index.ts` | Route resource descriptor |
| `apps/web/src/resources/project/project-list.tsx` | Project `ListLive` + DataTable |
| `apps/web/src/resources/project/project-edit.tsx` | Project `EditLive` + `SimpleForm` |
| `apps/web/src/resources/project/project-create.tsx` | Project `Create` + form fields |
| `apps/web/src/resources/project/project-show.tsx` | Project `ShowLive` |
| `apps/web/src/resources/project/project-list.browser.test.tsx` | Browser test for `ProjectList` |
| `apps/web/src/resources/project/index.ts` | Project resource descriptor |
| `apps/web/src/resources/property/property-list.tsx` | Property `ListLive` + DataTable |
| `apps/web/src/resources/property/property-edit.tsx` | Property `EditLive` + `SimpleForm` |
| `apps/web/src/resources/property/property-create.tsx` | Property `Create` + form fields |
| `apps/web/src/resources/property/property-show.tsx` | Property `ShowLive` |
| `apps/web/src/resources/property/property-list.browser.test.tsx` | Browser test for `PropertyList` |
| `apps/web/src/resources/tileserver/tileserver-list.tsx` | Tileserver `ListLive` + DataTable |
| `apps/web/src/resources/tileserver/tileserver-edit.tsx` | Tileserver `EditLive` + form |
| `apps/web/src/resources/tileserver/tileserver-create.tsx` | Tileserver `Create` + form |
| `apps/web/src/resources/tileserver/tileserver-show.tsx` | Tileserver `ShowLive` |
| `apps/web/src/resources/tileserver/tileserver-list.browser.test.tsx` | Browser test for `TileserverList` |
| `apps/web/src/resources/tileserver/index.ts` | Tileserver resource descriptor |
| `apps/web/src/resources/plugins/plugins-list.tsx` | Plugins `ListLive` + DataTable |
| `apps/web/src/resources/plugins/plugins-edit.tsx` | Plugins `EditLive` + `SimpleForm` (edit-only) |
| `apps/web/src/resources/plugins/plugins-list.browser.test.tsx` | Browser test for `PluginsList` |
| `apps/web/src/resources/plugins/index.ts` | Plugins resource descriptor |
| `apps/web/src/resources/property/index.ts` | Property resource descriptor |

### Modified

| File | Change |
|------|--------|
| `crates/koji-service/src/internal/mod.rs` | Replace `v2::routes::scope()` with bespoke `GET` + `routes::internal_item_scope()`; add `pub(crate) mod routes;` |
| `crates/koji-service/src/public/v2/routes.rs` | Add `internal_item_scope()` function (mirror `geofences::internal_item_scope()`) |
| `crates/koji-service/src/lib.rs` | Add `test_internal_routes_app()` pub test surface |
| `apps/web/src/data-provider.ts` | Add `route: { seg: "routes", geo: true }` to `RESOURCE_MAP` |
| `apps/web/src/lib/constants.ts` | Add `PROPERTY_CATEGORIES`, `ROUTE_MODES` |
| `apps/web/src/App.tsx` | Register all six resources with groups |

---

## Tasks

### Task 1: Backend — `GET /internal/routes` row-list + DB test

**Files:**
- Create: `crates/koji-service/src/internal/routes.rs`
- Modify: `crates/koji-service/src/internal/mod.rs`
- Modify: `crates/koji-service/src/public/v2/routes.rs`
- Modify: `crates/koji-service/src/lib.rs`
- Create (Test): `crates/koji-service/tests/internal_routes_rows.rs`

**Interfaces:**
- Consumes: `koji_db::db::route::Query::paginate(db, AdminReqParsed)` → `PaginateResults<Vec<Json>>`. Each JSON row has fields: `id`, `geofence_id`, `name`, `description`, `points`, `mode`.
- Produces: `GET /internal/routes` → `{ status: "ok", data: RouteRow[], meta: Meta }`. `RouteRow = { id: i64, name: String, description: Option<String>, mode: String, geofence_id: i64, points: usize }`. Also produces `v2::routes::internal_item_scope()` (collection `POST` + `/{id}` GET/PATCH/DELETE/publish) for the internal scope to consume.

- [ ] **Step 1: Write the failing DB test**

```rust
// crates/koji-service/tests/internal_routes_rows.rs
#![allow(clippy::await_holding_lock)]
use actix_web::test;
use koji_db::KojiDb;
use koji_jobs::JobId;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement, Value};
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());
fn serial_guard() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|p| p.into_inner())
}
async fn test_db() -> Option<DatabaseConnection> {
    let Ok(url) = std::env::var("KOJI_DB_URL") else {
        eprintln!("skip: KOJI_DB_URL unset");
        return None;
    };
    Database::connect(&url).await.ok().or_else(|| {
        eprintln!("skip: connect failed");
        None
    })
}
fn unique_name(tag: &str) -> String {
    format!("test-{tag}-{}", JobId::new().as_string())
}
async fn body_json(resp: actix_web::dev::ServiceResponse) -> serde_json::Value {
    serde_json::from_slice(&test::read_body(resp).await).unwrap()
}
async fn build_test_koji_db(koji_db: DatabaseConnection) -> KojiDb {
    let url = std::env::var("KOJI_DB_URL").unwrap();
    let golbat = Database::connect(&url).await.expect("golbat re-connect");
    KojiDb { koji: koji_db, golbat }
}

/// Insert a geofence (needed as FK) and then a route under it.
async fn insert_geofence_and_route(
    db: &DatabaseConnection,
    route_name: &str,
    mode: &str,
) -> (u64, u64) {
    let fence_id = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO geofence (name, mode, geometry, created_at, updated_at) \
             VALUES (?, ?, '{\"type\":\"Polygon\",\"coordinates\":[[[0,0],[1,0],[1,1],[0,0]]]}', NOW(), NOW())",
            [Value::from(format!("fence-for-{route_name}")), Value::from("unset")],
        ))
        .await
        .unwrap()
        .last_insert_id();

    let route_id = db
        .execute(Statement::from_sql_and_values(
            DbBackend::MySql,
            "INSERT INTO route (geofence_id, name, mode, geometry, created_at, updated_at) \
             VALUES (?, ?, ?, '{\"type\":\"MultiPoint\",\"coordinates\":[[1,2],[3,4]]}', NOW(), NOW())",
            [Value::from(fence_id), Value::from(route_name.to_owned()), Value::from(mode)],
        ))
        .await
        .unwrap()
        .last_insert_id();

    (fence_id, route_id)
}

async fn cleanup_route(db: &DatabaseConnection, route_id: u64, fence_id: u64) {
    let _ = db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "DELETE FROM route WHERE id = ?",
        [Value::from(route_id)],
    )).await;
    let _ = db.execute(Statement::from_sql_and_values(
        DbBackend::MySql,
        "DELETE FROM geofence WHERE id = ?",
        [Value::from(fence_id)],
    )).await;
}

#[actix_web::test]
async fn route_row_list_returns_row_shape_and_meta() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else { return; };
    let db = build_test_koji_db(conn.clone()).await;
    let name = unique_name("route-rows");
    let (fence_id, route_id) = insert_geofence_and_route(&conn, &name, "pokemon").await;

    let app = test::init_service(koji_service::test_internal_routes_app(db)).await;
    let req = test::TestRequest::get()
        .uri(&format!("/internal/routes?per_page=500&q={name}"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    let v = body_json(resp).await;
    assert_eq!(v["status"], "ok");
    let row = v["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    assert_eq!(row["id"], route_id);
    assert_eq!(row["mode"], "pokemon");
    assert!(row["geofence_id"].as_i64().unwrap() > 0);
    assert!(row["points"].as_u64().unwrap() >= 2);
    assert_eq!(v["meta"]["page"], 1);
    assert!(v["meta"]["total"].as_i64().unwrap() >= 1);
    cleanup_route(&conn, route_id, fence_id).await;
}

#[actix_web::test]
async fn route_row_list_middle_page_has_prev_and_has_next() {
    let _g = serial_guard();
    let Some(conn) = test_db().await else { return; };
    let db = build_test_koji_db(conn.clone()).await;
    let tag = format!("mid-{}", JobId::new().as_string());
    let mut pairs = Vec::new();
    for suffix in ["a", "b", "c"] {
        pairs.push(insert_geofence_and_route(&conn, &format!("test-{tag}-{suffix}"), "unset").await);
    }

    let app = test::init_service(koji_service::test_internal_routes_app(db)).await;
    let req = test::TestRequest::get()
        .uri(&format!("/internal/routes?per_page=1&page=2&q={tag}&sortBy=name&order=ASC"))
        .to_request();
    let v = body_json(test::call_service(&app, req).await).await;
    assert_eq!(v["meta"]["page"], 2);
    assert_eq!(v["meta"]["total"], 3);
    assert_eq!(v["meta"]["has_prev"], true, "middle page must report has_prev=true");
    assert_eq!(v["meta"]["has_next"], true, "middle page must report has_next=true");
    for (fid, rid) in pairs {
        cleanup_route(&conn, rid, fid).await;
    }
}
```

- [ ] **Step 2: Run test to verify it fails (skip if no KOJI_DB_URL, else fails with "function not found")**

```bash
cd /Users/rin/GitHub/Koji
cargo test --package koji-service --test internal_routes_rows 2>&1 | tail -20
```

Expected: FAIL — `koji_service::test_internal_routes_app` does not exist.

- [ ] **Step 3: Create `crates/koji-service/src/internal/routes.rs`**

```rust
//! Bespoke `GET /internal/routes` — paginated flat rows for the shadmin
//! DataTable. Reuses `route::Query::paginate` (koji-db) — serialization only.
use actix_web::{web, HttpResponse};
use koji_db::{KojiDb, query_args::AdminReqParsed};
use serde::{Deserialize, Serialize};

use crate::utils::api_response::{ApiResponse, Meta};
use crate::utils::error::ServiceError;

/// `?page&per_page&sortBy|sort_by&order&q&mode&geofenceid&pointsmin&pointsmax`
#[derive(Debug, Deserialize)]
pub(crate) struct RowQuery {
    page: Option<i64>,
    per_page: Option<i64>,
    #[serde(alias = "sortBy")]
    sort_by: Option<String>,
    order: Option<String>,
    q: Option<String>,
    mode: Option<String>,
    geofenceid: Option<u32>,
    pointsmin: Option<u32>,
    pointsmax: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RouteRow {
    id: i64,
    name: String,
    description: Option<String>,
    mode: String,
    geofence_id: i64,
    points: usize,
}

pub(crate) async fn list_rows(
    conn: web::Data<KojiDb>,
    query: web::Query<RowQuery>,
) -> Result<HttpResponse, ServiceError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 500);
    let args = AdminReqParsed {
        page: (page - 1) as u64,
        per_page: per_page as u64,
        sort_by: query.sort_by.clone().unwrap_or_else(|| "id".to_string()),
        order: query.order.clone().unwrap_or_else(|| "ASC".to_string()),
        q: query.q.clone().unwrap_or_default(),
        geofenceid: query.geofenceid,
        mode: query.mode.clone(),
        pointsmin: query.pointsmin,
        pointsmax: query.pointsmax,
        // Unused by route::Query::paginate
        geotype: None,
        project: None,
        parent: None,
    };

    let (rows, total, has_next, has_prev) =
        koji_db::db::route::Query::paginate(&conn.koji, args)
            .await?
            .into_parts();

    let data: Vec<RouteRow> = rows
        .into_iter()
        .map(|r| RouteRow {
            id: r.get("id").and_then(serde_json::Value::as_i64).unwrap_or(0),
            name: r.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            description: r
                .get("description")
                .and_then(|v| if v.is_null() { None } else { v.as_str().map(str::to_string) }),
            mode: r.get("mode").and_then(|v| v.as_str()).unwrap_or("unset").to_string(),
            geofence_id: r.get("geofence_id").and_then(serde_json::Value::as_i64).unwrap_or(0),
            points: r.get("points").and_then(serde_json::Value::as_u64).unwrap_or(0) as usize,
        })
        .collect();

    let total_pages = if per_page > 0 { ((total as i64) + per_page - 1) / per_page } else { 0 };

    Ok(ApiResponse::success_paginated(
        data,
        Meta { total: total as i64, page, per_page, total_pages, has_next, has_prev },
    ))
}
```

- [ ] **Step 4: Add `pub(crate) mod routes;` to `crates/koji-service/src/internal/mod.rs` and replace the forwarded `v2::routes::scope()` with the bespoke resource + item scope**

Open `crates/koji-service/src/internal/mod.rs`. Change:

```rust
pub(crate) mod geofences;
```

to:

```rust
pub(crate) mod geofences;
pub(crate) mod routes;
```

Change the comment block to add the routes row-list documentation, and change:

```rust
        // Plain CRUD resources — macro-generated scopes forward unchanged.
        .service(v2::routes::scope())
```

to:

```rust
        // Route row-list (bespoke) + POST collection + /{id} CRUD forwarded.
        .service(
            web::resource("/routes")
                .route(web::get().to(routes::list_rows))
                .route(web::post().to(v2::routes::create)),
        )
        .service(v2::routes::internal_item_scope())
```

- [ ] **Step 5: Add `internal_item_scope()` to `crates/koji-service/src/public/v2/routes.rs`**

At the end of the file, before the closing `#[cfg(test)]` block, add:

```rust
/// Like [`scope`] but omits the collection `GET` (list). Used by the `/internal`
/// scope, which overrides `GET /internal/routes` with the bespoke row-list
/// handler while forwarding all other route operations unchanged.
pub(crate) fn internal_item_scope() -> actix_web::Scope {
    web::scope("/routes")
        .service(web::resource("/{id}/publish").route(web::post().to(publish)))
        .service(
            web::resource("/{id}")
                .route(web::get().to(get_one))
                .route(web::patch().to(update))
                .route(web::delete().to(remove)),
        )
}
```

Also make `create` pub(crate) if it isn't already (it is — confirmed in source).

- [ ] **Step 6: Add `test_internal_routes_app` to `crates/koji-service/src/lib.rs`**

After the `test_projects_app` function, add:

```rust
/// Test surface: a minimal App mounting `GET /internal/routes` (row list).
#[doc(hidden)]
pub fn test_internal_routes_app(
    db: koji_db::KojiDb,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(web::Data::new(db))
        .service(web::scope("/internal").service(
            web::resource("/routes")
                .route(web::get().to(internal::routes::list_rows)),
        ))
}
```

- [ ] **Step 7: Run tests to verify they pass (or skip when no KOJI_DB_URL)**

```bash
cd /Users/rin/GitHub/Koji
cargo test --package koji-service --test internal_routes_rows 2>&1 | tail -30
```

Expected: PASS (or "skip: KOJI_DB_URL unset" for each test).

Also check existing tests still pass:

```bash
cargo test --package koji-service 2>&1 | tail -20
```

- [ ] **Step 8: Commit**

```bash
git add crates/koji-service/src/internal/routes.rs \
        crates/koji-service/src/internal/mod.rs \
        crates/koji-service/src/public/v2/routes.rs \
        crates/koji-service/src/lib.rs \
        crates/koji-service/tests/internal_routes_rows.rs
git commit -m "$(cat <<'EOF'
feat(internal): add GET /internal/routes row-list + internal_item_scope

Mirrors geofences pattern: bespoke list_rows handler at the collection GET,
internal_item_scope() for /{id} CRUD forwarding, DB-gated test.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Vendor `ColorInput` + `MonacoJsonInput`; add `PROPERTY_CATEGORIES`/`ROUTE_MODES` to constants

**Files:**
- Create: `apps/web/src/components/admin/inputs/color-input.tsx`
- Create: `apps/web/src/components/monaco/monaco-json-input.tsx`
- Create: `apps/web/src/components/monaco/index.ts`
- Modify: `apps/web/src/components/admin/index.ts`
- Modify: `apps/web/src/lib/constants.ts`

**Interfaces:**
- Consumes: shadcn-admin-kit source at `/Users/rin/GitHub/shadcn-admin-kit/packages/shadmin/src/components/extras/color-input.tsx` and `monaco/monaco-json-input.tsx`. ColorInput's internal deps (`@/components/ui/color-picker`, `@/components/ui/field`, `@/components/admin/common/input-helper-text`) — verify these exist in Koji web before assuming they do; add stubs if missing.
- Produces: `ColorInput` exported from `@/components/admin` (via `index.ts`); `MonacoJsonInput` exported from `@/components/monaco`; `PROPERTY_CATEGORIES`, `ROUTE_MODES` from `@/lib/constants`.

- [ ] **Step 1: Check that ColorInput's deps exist in Koji web**

```bash
ls /Users/rin/GitHub/Koji/apps/web/src/components/ui/color-picker.tsx 2>/dev/null && echo "exists" || echo "MISSING"
ls /Users/rin/GitHub/Koji/apps/web/src/components/admin/common/input-helper-text.tsx 2>/dev/null && echo "exists" || echo "MISSING"
```

If either is MISSING, read the shadcn-admin-kit source for those files and vendor them (same pattern as ColorInput). The `FieldLabelText` component is used in shadcn-admin-kit's ColorInput but Koji web uses a different component — confirm against `apps/web/src/components/admin/inputs/number-input.tsx` (which imports `FieldTitle` from `shadmin-core`). Adapt ColorInput to match Koji web's established pattern (use `FieldTitle` from `shadmin-core` instead of `FieldLabelText` if needed).

- [ ] **Step 2: Create `apps/web/src/components/admin/inputs/color-input.tsx`**

Adapt from shadcn-admin-kit source, replacing internal kit paths with Koji web's actual paths. The minimal working version:

```tsx
import * as React from "react";
import type { InputProps } from "shadmin-core";
import { useInput, FieldTitle, ValidationError, useResourceContext } from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import { cn } from "@/lib/utils";

interface ColorInputProps
  extends InputProps,
    Omit<React.ComponentProps<"input">, "defaultValue" | "onBlur" | "onChange" | "type"> {
  swatches?: readonly string[];
}

function ColorInput(props: ColorInputProps) {
  const { label, source, className, resource: resourceProp, helperText, swatches, disabled } =
    props;
  const resource = useResourceContext({ resource: resourceProp });
  const { onChange: _sc, onBlur: _sb, ...sansHandlers } = props;
  void _sc; void _sb;
  const { id, field, fieldState, isRequired } = useInput(sansHandlers);
  const invalid = fieldState.invalid;
  const errorMessage = fieldState.error?.root?.message ?? fieldState.error?.message;
  const value = (field.value as string | undefined) ?? "#000000";

  return (
    <Field className={className} data-invalid={invalid || undefined}>
      {label !== false && (
        <FieldLabel htmlFor={id}>
          <FieldTitle label={label} source={source} resource={resource} isRequired={isRequired} />
        </FieldLabel>
      )}
      <div className="flex items-center gap-2">
        <input
          id={id}
          type="color"
          value={value}
          disabled={disabled}
          aria-invalid={invalid || undefined}
          onChange={(e) => field.onChange(e.target.value)}
          onBlur={field.onBlur}
          className={cn(
            "h-9 w-16 cursor-pointer rounded border border-input bg-transparent p-1",
            disabled && "cursor-not-allowed opacity-50",
          )}
        />
        <span className="text-sm text-muted-foreground">{value}</span>
        {swatches?.map((s) => (
          <button
            key={s}
            type="button"
            aria-label={`Select color ${s}`}
            onClick={() => field.onChange(s)}
            disabled={disabled}
            className="h-6 w-6 rounded border border-border"
            style={{ backgroundColor: s }}
          />
        ))}
      </div>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? <ValidationError error={errorMessage} /> : null}
      </FieldError>
    </Field>
  );
}

export { ColorInput, type ColorInputProps };
```

> Note: This uses a native `<input type="color">` rather than the full oklch popover picker from shadcn-admin-kit (which requires the `color-picker` UI primitive not yet in Koji web). Functional for the property resource's `color` category. The full picker can be vendored in a follow-on task.

- [ ] **Step 3: Add `export * from "@/components/admin/inputs/color-input";` to `apps/web/src/components/admin/index.ts`**

Add the line immediately after the `boolean-input` export:

```ts
export * from "@/components/admin/inputs/color-input";
```

- [ ] **Step 4: Check if `monaco-editor` is installed**

```bash
ls /Users/rin/GitHub/Koji/apps/web/node_modules/monaco-editor 2>/dev/null && echo "installed" || echo "NOT installed"
```

If NOT installed:

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun add monaco-editor @monaco-editor/react
```

- [ ] **Step 5: Create `apps/web/src/components/monaco/monaco-json-input.tsx`**

A simple wrapper around `@monaco-editor/react` (lighter to vendor than the full shadcn-admin-kit lazy-load chain):

```tsx
import { useInput, FieldTitle, ValidationError, useResourceContext } from "shadmin-core";
import { Field, FieldError, FieldLabel } from "@/components/ui/field";
import { InputHelperText } from "@/components/admin/common/input-helper-text";
import Editor from "@monaco-editor/react";
import type { InputProps } from "shadmin-core";

interface MonacoJsonInputProps extends InputProps {
  height?: number | string;
  schema?: object;
  readOnly?: boolean;
  className?: string;
}

function MonacoJsonInput(props: MonacoJsonInputProps) {
  const { label, source, resource: resourceProp, helperText, height = 300, schema, readOnly, className } = props;
  const resource = useResourceContext({ resource: resourceProp });
  const { id, field, fieldState, isRequired } = useInput(props);
  const invalid = fieldState.invalid;
  const errorMessage = fieldState.error?.root?.message ?? fieldState.error?.message;

  const stringValue =
    typeof field.value === "string"
      ? field.value
      : field.value != null
        ? JSON.stringify(field.value, null, 2)
        : "";

  const handleChange = (val: string | undefined) => {
    const s = val ?? "";
    try {
      field.onChange(JSON.parse(s));
    } catch {
      field.onChange(s);
    }
  };

  return (
    <Field className={className} data-invalid={invalid || undefined}>
      {label !== false && (
        <FieldLabel htmlFor={id}>
          <FieldTitle label={label} source={source} resource={resource} isRequired={isRequired} />
        </FieldLabel>
      )}
      <div className="overflow-hidden rounded-md border" style={{ height }}>
        <Editor
          height={height}
          defaultLanguage="json"
          value={stringValue}
          onChange={handleChange}
          options={{
            readOnly: readOnly ?? false,
            minimap: { enabled: false },
            scrollBeyondLastLine: false,
            ...(schema
              ? {}
              : {}),
          }}
          beforeMount={(monaco) => {
            if (schema) {
              monaco.languages.json.jsonDefaults.setDiagnosticsOptions({
                validate: true,
                schemas: [{ uri: "https://koji/schema", fileMatch: ["*"], schema }],
              });
            }
          }}
        />
      </div>
      <InputHelperText helperText={helperText} />
      <FieldError>
        {invalid && errorMessage ? <ValidationError error={errorMessage} /> : null}
      </FieldError>
    </Field>
  );
}

export { MonacoJsonInput, type MonacoJsonInputProps };
```

- [ ] **Step 6: Create `apps/web/src/components/monaco/index.ts`**

```ts
export * from "./monaco-json-input";
```

- [ ] **Step 7: Add `PROPERTY_CATEGORIES` and `ROUTE_MODES` to `apps/web/src/lib/constants.ts`**

Append to the file:

```ts
export const PROPERTY_CATEGORIES = [
  { id: "boolean", name: "Boolean" },
  { id: "string", name: "String" },
  { id: "number", name: "Number" },
  { id: "object", name: "Object (JSON)" },
  { id: "array", name: "Array (JSON)" },
  { id: "database", name: "Database (runtime)" },
  { id: "color", name: "Color" },
] as const;

// Route modes mirror geofence modes (same DB enum).
export const ROUTE_MODES = GEOFENCE_MODES;
```

- [ ] **Step 8: Run typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck 2>&1 | tail -30
```

Expected: no errors.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/components/admin/inputs/color-input.tsx \
        apps/web/src/components/monaco/monaco-json-input.tsx \
        apps/web/src/components/monaco/index.ts \
        apps/web/src/components/admin/index.ts \
        apps/web/src/lib/constants.ts
git commit -m "$(cat <<'EOF'
feat(web): vendor ColorInput + MonacoJsonInput; add PROPERTY_CATEGORIES/ROUTE_MODES

Adds ColorInput (native color picker, functional for property category=color),
MonacoJsonInput (monaco-editor wrapper for JSON fields), and the two new
constant arrays needed by property and route resources.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: `PropertyValueInput` component + browser test

**Files:**
- Create: `apps/web/src/components/inputs/property-value-input.tsx`
- Create: `apps/web/src/components/inputs/property-value-input.browser.test.tsx`

**Interfaces:**
- Consumes: `BooleanInput`, `TextInput`, `NumberInput`, `SelectInput` from `@/components/admin`; `ColorInput` from `@/components/admin`; `MonacoJsonInput` from `@/components/monaco`; `useWatch` from `react-hook-form`; `PROPERTY_CATEGORIES` from `@/lib/constants`.
- Produces: `PropertyValueInput` — props: `{ source: string; categorySource?: string }`. Used by property edit/create forms (Task 5).

- [ ] **Step 1: Write the failing browser test**

```tsx
// apps/web/src/components/inputs/property-value-input.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { Create, SimpleForm } from "@/components/admin";
import { SelectInput } from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="property">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("PropertyValueInput", () => {
  it("renders a TextInput by default (string category)", async () => {
    const screen = render(
      wrap(
        <Create>
          <SimpleForm defaultValues={{ category: "string" }}>
            <SelectInput
              source="category"
              choices={[{ id: "string", name: "String" }, { id: "boolean", name: "Boolean" }]}
            />
            <PropertyValueInput source="default_value" />
          </SimpleForm>
        </Create>,
      ),
    );
    // Default category=string → TextInput
    await expect.element(screen.getByLabelText(/default.value/i)).toBeVisible();
  });

  it("renders a BooleanInput when category is boolean", async () => {
    const screen = render(
      wrap(
        <Create>
          <SimpleForm defaultValues={{ category: "boolean" }}>
            <SelectInput
              source="category"
              choices={[{ id: "boolean", name: "Boolean" }]}
            />
            <PropertyValueInput source="default_value" />
          </SimpleForm>
        </Create>,
      ),
    );
    // category=boolean → BooleanInput (a switch)
    await expect.element(screen.getByRole("switch")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails (background — ~100s Chromium boot)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser -- --reporter=verbose 2>&1 | grep -E "FAIL|PASS|Error" | tail -20
```

Expected: FAIL — `PropertyValueInput` not found.

- [ ] **Step 3: Implement `apps/web/src/components/inputs/property-value-input.tsx`**

```tsx
import { useWatch } from "react-hook-form";
import {
  TextInput,
  NumberInput,
  BooleanInput,
  SelectInput,
  ColorInput,
} from "@/components/admin";
import { MonacoJsonInput } from "@/components/monaco";

interface PropertyValueInputProps {
  source: string;
  /**
   * The RHF field name to watch for the category value.
   * Defaults to "category".
   */
  categorySource?: string;
}

const JSON_CATEGORIES = new Set(["object", "array"]);

/**
 * A dynamic input that renders the appropriate input type for a property's
 * `default_value` field, switching on the sibling `category` field via
 * `useWatch`. Non-destructive on category change — the current value is kept
 * with a warning note.
 */
function PropertyValueInput({
  source,
  categorySource = "category",
}: PropertyValueInputProps) {
  const category = useWatch({ name: categorySource }) as string | undefined;

  if (category === "boolean") {
    return <BooleanInput source={source} label="Default Value" />;
  }

  if (category === "number") {
    return <NumberInput source={source} label="Default Value" />;
  }

  if (category === "color") {
    return <ColorInput source={source} label="Default Value" />;
  }

  if (category != null && JSON_CATEGORIES.has(category)) {
    return (
      <MonacoJsonInput
        source={source}
        label="Default Value"
        height={200}
        helperText={`Must be a JSON ${category}.`}
      />
    );
  }

  if (category === "database") {
    return (
      <TextInput
        source={source}
        label="Default Value"
        disabled
        helperText="Resolved from the database at runtime — cannot be set here."
      />
    );
  }

  // Fallback: string / unknown / undefined → plain TextInput
  return <TextInput source={source} label="Default Value" />;
}

export { PropertyValueInput, type PropertyValueInputProps };
```

- [ ] **Step 4: Run browser tests to verify they pass (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: PASS for property-value-input tests; geofence tests still pass.

- [ ] **Step 5: Commit**

```bash
git add apps/web/src/components/inputs/property-value-input.tsx \
        apps/web/src/components/inputs/property-value-input.browser.test.tsx
git commit -m "$(cat <<'EOF'
feat(web): add PropertyValueInput component (category-switched default_value input)

Switches on the sibling `category` field via useWatch: boolean→BooleanInput,
number→NumberInput, color→ColorInput, object/array→MonacoJsonInput,
database→disabled TextInput, string/fallback→TextInput.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Route resource (list/edit/create/show) + register in dataProvider + App.tsx

**Files:**
- Create: `apps/web/src/resources/route/route-list.tsx`
- Create: `apps/web/src/resources/route/route-edit.tsx`
- Create: `apps/web/src/resources/route/route-create.tsx`
- Create: `apps/web/src/resources/route/route-show.tsx`
- Create: `apps/web/src/resources/route/route-list.browser.test.tsx`
- Create: `apps/web/src/resources/route/route-form.browser.test.tsx`
- Create: `apps/web/src/resources/route/index.ts`
- Modify: `apps/web/src/data-provider.ts`

**Interfaces:**
- Consumes: `MultiPointInput`, `MultiPointField` from `@/components/leaflet`; `GeoJsonField` from `@/components/leaflet`; `ListLive`, `EditLive`, `ShowLive` from `@/components/realtime`; `ROUTE_MODES`, `DEFAULT_TILE_URL` from `@/lib/constants`.
- Produces: `route` resource descriptor (name/list/edit/create/show/recordRepresentation/icon); `RESOURCE_MAP.route = { seg: "routes", geo: true }`.

- [ ] **Step 1: Write failing browser tests**

```tsx
// apps/web/src/resources/route/route-list.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { RouteList } from "@/resources/route/route-list";

const fakeRows = [
  { id: 1, name: "North Loop", mode: "pokemon", geofence_id: 10, points: 42, description: null },
  { id: 2, name: "South Run", mode: "fort", geofence_id: 11, points: 7, description: "daily" },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="route">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("RouteList", () => {
  it("renders route rows from the row list", async () => {
    const screen = render(wrap(<RouteList />));
    await expect.element(screen.getByText("North Loop")).toBeVisible();
    await expect.element(screen.getByText("South Run")).toBeVisible();
  });

  it("renders the live-search filter input", async () => {
    const screen = render(wrap(<RouteList />));
    await expect.element(screen.getByPlaceholder(/search/i)).toBeVisible();
  });
});
```

```tsx
// apps/web/src/resources/route/route-form.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { RouteCreate } from "@/resources/route/route-create";
import { RouteEdit } from "@/resources/route/route-edit";

const MULTI_POINT_RECORD = {
  id: 1,
  name: "Test Route",
  mode: "pokemon",
  geofence_id: 5,
  description: null,
  geometry: { type: "MultiPoint", coordinates: [[1, 2], [3, 4], [5, 6]] },
};

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: [] as any, total: 0 }),
    getMany: async () => ({ data: [] as any }),
    getOne: async () => ({ data: MULTI_POINT_RECORD as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

const wrap = (node: React.ReactNode) => (
  <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
    <ResourceContextProvider value="route">{node}</ResourceContextProvider>
  </AdminContext>
);

describe("Route form", () => {
  it("renders name and mode inputs in create view", async () => {
    const screen = render(wrap(<RouteCreate />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect.element(screen.getByLabelText(/mode/i)).toBeVisible();
  });

  it("renders a Leaflet map in create view", async () => {
    const screen = render(wrap(<RouteCreate />));
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
  });

  it("renders the Leaflet map when a MultiPoint record is loaded in edit view", async () => {
    const screen = render(wrap(<RouteEdit id={1} />));
    await expect.element(screen.getByLabelText(/name/i)).toBeVisible();
    await expect
      .element(screen.container.querySelector(".leaflet-container"))
      .toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run tests to verify they fail (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|cannot find module" | tail -10
```

Expected: FAIL — route modules not found.

- [ ] **Step 3: Add `route` to `RESOURCE_MAP` in `apps/web/src/data-provider.ts`**

```ts
const RESOURCE_MAP: Record<string, ResourceDef> = {
  geofence: { seg: "geofences", geo: true },
  route: { seg: "routes", geo: true },
  project: { seg: "projects", geo: false },
  property: { seg: "properties", geo: false },
  tileserver: { seg: "tile-servers", geo: false },
  plugins: { seg: "plugins", geo: false },
};
```

- [ ] **Step 4: Create `apps/web/src/resources/route/route-list.tsx`**

```tsx
import {
  DataTable,
  ReferenceField,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { ROUTE_MODES } from "@/lib/constants";

const RouteFilters = () => (
  <div className="flex w-56 flex-col gap-4">
    <FilterLiveSearch source="q" />
    <FilterList label="Mode">
      {ROUTE_MODES.map((m) => (
        <FilterListItem key={m.id} label={m.name} value={{ mode: m.id }} />
      ))}
    </FilterList>
  </div>
);

export const RouteList = () => (
  <ListLive aside={<RouteFilters />}>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="description" />
      <DataTable.Col source="mode" />
      <DataTable.Col source="geofence_id" label="Geofence">
        <ReferenceField source="geofence_id" reference="geofence" />
      </DataTable.Col>
      <DataTable.Col source="points" label="Points" />
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 5: Create `apps/web/src/resources/route/route-create.tsx`**

```tsx
import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
  ReferenceInput,
} from "@/components/admin";
import { MultiPointInput } from "@/components/leaflet";
import { ROUTE_MODES, DEFAULT_TILE_URL } from "@/lib/constants";
import { required } from "ra-core";

export const RouteFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="description" />
    <SelectInput source="mode" choices={[...ROUTE_MODES]} defaultValue="unset" />
    <ReferenceInput source="geofence_id" reference="geofence" validate={required()} />
    <MultiPointInput source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
  </>
);

export const RouteCreate = () => (
  <Create>
    <SimpleForm>
      <RouteFormFields />
    </SimpleForm>
  </Create>
);
```

- [ ] **Step 6: Create `apps/web/src/resources/route/route-edit.tsx`**

```tsx
import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { RouteFormFields } from "./route-create";

export const RouteEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <RouteFormFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 7: Create `apps/web/src/resources/route/route-show.tsx`**

```tsx
import { TextField, ReferenceField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { MultiPointField } from "@/components/leaflet";
import { DEFAULT_TILE_URL } from "@/lib/constants";
import type { ShowProps } from "@/components/admin/views/show";

export const RouteShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <div className="flex flex-col gap-2">
        <TextField source="name" />
        <TextField source="mode" />
        <TextField source="description" />
        <ReferenceField source="geofence_id" reference="geofence" empty="—" />
      </div>
      <MultiPointField source="geometry" tileUrl={DEFAULT_TILE_URL} height={400} />
    </div>
  </ShowLive>
);
```

- [ ] **Step 8: Create `apps/web/src/resources/route/index.ts`**

```ts
import { Route } from "lucide-react";
import { RouteList } from "./route-list";
import { RouteEdit } from "./route-edit";
import { RouteCreate } from "./route-create";
import { RouteShow } from "./route-show";

export const route = {
  name: "route",
  list: RouteList,
  edit: RouteEdit,
  create: RouteCreate,
  show: RouteShow,
  recordRepresentation: "name",
  icon: Route,
};
```

- [ ] **Step 9: Run browser tests (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: route-list and route-form tests PASS.

- [ ] **Step 10: Commit**

```bash
git add apps/web/src/resources/route/ apps/web/src/data-provider.ts
git commit -m "$(cat <<'EOF'
feat(web): add route resource (list/edit/create/show) + route in RESOURCE_MAP

List shows name/description/mode/geofence_id ref/points. Edit/Create use
MultiPointInput for geometry. Show uses MultiPointField.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Project resource (list/edit/create/show)

**Files:**
- Create: `apps/web/src/resources/project/project-list.tsx`
- Create: `apps/web/src/resources/project/project-edit.tsx`
- Create: `apps/web/src/resources/project/project-create.tsx`
- Create: `apps/web/src/resources/project/project-show.tsx`
- Create: `apps/web/src/resources/project/project-list.browser.test.tsx`
- Create: `apps/web/src/resources/project/index.ts`

**Interfaces:**
- Consumes: `ReferenceArrayInput`, `AutocompleteArrayInput`, `BooleanInput`, `BooleanField`, `NumberField`, `ReferenceArrayField` from `@/components/admin`; `ListLive`, `EditLive`, `ShowLive` from `@/components/realtime`.
- Produces: `project` resource descriptor.

- [ ] **Step 1: Write the failing browser test**

```tsx
// apps/web/src/resources/project/project-list.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { ProjectList } from "@/resources/project/project-list";

const fakeRows = [
  { id: 1, name: "Alpha Project", golbat: true, geofences: [1, 2] },
  { id: 2, name: "Beta Project", golbat: false, geofences: [] },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("ProjectList", () => {
  it("renders project rows", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="project">
          <ProjectList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("Alpha Project")).toBeVisible();
    await expect.element(screen.getByText("Beta Project")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "project|FAIL" | tail -10
```

Expected: FAIL — module not found.

- [ ] **Step 3: Create `apps/web/src/resources/project/project-list.tsx`**

```tsx
import {
  DataTable,
  BooleanField,
  FilterLiveSearch,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { FunctionField } from "@/components/admin";

export const ProjectList = () => (
  <ListLive aside={
    <div className="flex w-56 flex-col gap-4">
      <FilterLiveSearch source="q" />
    </div>
  }>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="golbat" label="Golbat Sync">
        <BooleanField source="golbat" />
      </DataTable.Col>
      <DataTable.Col source="geofences" label="Geofences">
        <FunctionField
          source="geofences"
          render={(record: any) =>
            Array.isArray(record?.geofences) ? record.geofences.length : 0
          }
        />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 4: Create `apps/web/src/resources/project/project-create.tsx`**

```tsx
import {
  Create,
  SimpleForm,
  TextInput,
  BooleanInput,
  ReferenceArrayInput,
  AutocompleteArrayInput,
} from "@/components/admin";
import { required } from "ra-core";

export const ProjectFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <BooleanInput source="golbat" label="Golbat Sync" defaultValue={false} />
    <ReferenceArrayInput source="geofences" reference="geofence">
      <AutocompleteArrayInput />
    </ReferenceArrayInput>
  </>
);

export const ProjectCreate = () => (
  <Create>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </Create>
);
```

- [ ] **Step 5: Create `apps/web/src/resources/project/project-edit.tsx`**

```tsx
import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { ProjectFormFields } from "./project-create";

export const ProjectEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <ProjectFormFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 6: Create `apps/web/src/resources/project/project-show.tsx`**

```tsx
import { TextField, BooleanField, ReferenceArrayField, SingleFieldList, ChipField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";

export const ProjectShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <BooleanField source="golbat" label="Golbat Sync" />
      <ReferenceArrayField source="geofences" reference="geofence">
        <SingleFieldList>
          <ChipField source="name" />
        </SingleFieldList>
      </ReferenceArrayField>
    </div>
  </ShowLive>
);
```

- [ ] **Step 7: Create `apps/web/src/resources/project/index.ts`**

```ts
import { FolderOpen } from "lucide-react";
import { ProjectList } from "./project-list";
import { ProjectEdit } from "./project-edit";
import { ProjectCreate } from "./project-create";
import { ProjectShow } from "./project-show";

export const project = {
  name: "project",
  list: ProjectList,
  edit: ProjectEdit,
  create: ProjectCreate,
  show: ProjectShow,
  recordRepresentation: "name",
  icon: FolderOpen,
};
```

- [ ] **Step 8: Run browser tests (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: project-list test PASS.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/resources/project/
git commit -m "$(cat <<'EOF'
feat(web): add project resource (list/edit/create/show)

List shows name/golbat-sync/geofences-count. Edit/Create use
ReferenceArrayInput+AutocompleteArrayInput for geofences M2M.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: Property resource (list/edit/create/show) using `PropertyValueInput`

**Files:**
- Create: `apps/web/src/resources/property/property-list.tsx`
- Create: `apps/web/src/resources/property/property-edit.tsx`
- Create: `apps/web/src/resources/property/property-create.tsx`
- Create: `apps/web/src/resources/property/property-show.tsx`
- Create: `apps/web/src/resources/property/property-list.browser.test.tsx`
- Create: `apps/web/src/resources/property/index.ts`

**Interfaces:**
- Consumes: `PropertyValueInput` from `@/components/inputs/property-value-input`; `SelectInput`, `TextInput`, `FunctionField` from `@/components/admin`; `PROPERTY_CATEGORIES` from `@/lib/constants`.
- Produces: `property` resource descriptor.

- [ ] **Step 1: Write the failing browser test**

```tsx
// apps/web/src/resources/property/property-list.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PropertyList } from "@/resources/property/property-list";

const fakeRows = [
  { id: 1, name: "spawn_color", category: "color", default_value: "#ff0000", geofences: [1] },
  { id: 2, name: "iv_min", category: "number", default_value: 80, geofences: [] },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("PropertyList", () => {
  it("renders property rows with name and category", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="property">
          <PropertyList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("spawn_color")).toBeVisible();
    await expect.element(screen.getByText("iv_min")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "property|FAIL" | tail -10
```

Expected: FAIL.

- [ ] **Step 3: Create `apps/web/src/resources/property/property-list.tsx`**

```tsx
import {
  DataTable,
  FilterLiveSearch,
  FilterList,
  FilterListItem,
  FunctionField,
} from "@/components/admin";
import { ListLive } from "@/components/realtime";
import { PROPERTY_CATEGORIES } from "@/lib/constants";

export const PropertyList = () => (
  <ListLive aside={
    <div className="flex w-56 flex-col gap-4">
      <FilterLiveSearch source="q" />
      <FilterList label="Category">
        {PROPERTY_CATEGORIES.map((c) => (
          <FilterListItem key={c.id} label={c.name} value={{ category: c.id }} />
        ))}
      </FilterList>
    </div>
  }>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="category" />
      <DataTable.Col source="default_value" label="Default" />
      <DataTable.Col source="geofences" label="Geofences">
        <FunctionField
          source="geofences"
          render={(record: any) =>
            Array.isArray(record?.geofences) ? record.geofences.length : 0
          }
        />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 4: Create `apps/web/src/resources/property/property-create.tsx`**

```tsx
import {
  Create,
  SimpleForm,
  TextInput,
  SelectInput,
} from "@/components/admin";
import { PropertyValueInput } from "@/components/inputs/property-value-input";
import { PROPERTY_CATEGORIES } from "@/lib/constants";
import { required } from "ra-core";

export const PropertyFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <SelectInput
      source="category"
      choices={[...PROPERTY_CATEGORIES]}
      defaultValue="string"
      validate={required()}
    />
    <PropertyValueInput source="default_value" categorySource="category" />
  </>
);

export const PropertyCreate = () => (
  <Create>
    <SimpleForm>
      <PropertyFormFields />
    </SimpleForm>
  </Create>
);
```

- [ ] **Step 5: Create `apps/web/src/resources/property/property-edit.tsx`**

```tsx
import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { PropertyFormFields } from "./property-create";

export const PropertyEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <PropertyFormFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 6: Create `apps/web/src/resources/property/property-show.tsx`**

```tsx
import { TextField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import type { ShowProps } from "@/components/admin/views/show";

export const PropertyShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <TextField source="category" />
      <TextField source="default_value" label="Default Value" />
    </div>
  </ShowLive>
);
```

- [ ] **Step 7: Create `apps/web/src/resources/property/index.ts`**

```ts
import { Settings } from "lucide-react";
import { PropertyList } from "./property-list";
import { PropertyEdit } from "./property-edit";
import { PropertyCreate } from "./property-create";
import { PropertyShow } from "./property-show";

export const property = {
  name: "property",
  list: PropertyList,
  edit: PropertyEdit,
  create: PropertyCreate,
  show: PropertyShow,
  recordRepresentation: "name",
  icon: Settings,
};
```

- [ ] **Step 8: Run browser tests (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: property-list test PASS.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/resources/property/
git commit -m "$(cat <<'EOF'
feat(web): add property resource (list/edit/create/show) with PropertyValueInput

Edit/Create switch the default_value input on the category field via
PropertyValueInput. List filters by category.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: Tileserver resource (list/edit/create/show) with live `BaseMap` preview

**Files:**
- Create: `apps/web/src/resources/tileserver/tileserver-list.tsx`
- Create: `apps/web/src/resources/tileserver/tileserver-edit.tsx`
- Create: `apps/web/src/resources/tileserver/tileserver-create.tsx`
- Create: `apps/web/src/resources/tileserver/tileserver-show.tsx`
- Create: `apps/web/src/resources/tileserver/tileserver-list.browser.test.tsx`
- Create: `apps/web/src/resources/tileserver/index.ts`

**Interfaces:**
- Consumes: `BaseMap` from `@/components/leaflet`; `useWatch` from `react-hook-form`; `TextInput`, `UrlField` from `@/components/admin`; `ListLive`, `EditLive`, `ShowLive` from `@/components/realtime`.
- Produces: `tileserver` resource descriptor; `TileserverPreview` sub-component (inline, renders `BaseMap` with live-watched `url`).

- [ ] **Step 1: Write the failing browser test**

```tsx
// apps/web/src/resources/tileserver/tileserver-list.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { TileserverList } from "@/resources/tileserver/tileserver-list";

const fakeRows = [
  { id: 1, name: "CartoDB Voyager", url: "https://{s}.basemaps.cartocdn.com/..." },
  { id: 2, name: "OSM Standard", url: "https://{s}.tile.openstreetmap.org/..." },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("TileserverList", () => {
  it("renders tileserver rows with name and url", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="tileserver">
          <TileserverList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("CartoDB Voyager")).toBeVisible();
    await expect.element(screen.getByText("OSM Standard")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "tileserver|FAIL" | tail -10
```

- [ ] **Step 3: Create `apps/web/src/resources/tileserver/tileserver-list.tsx`**

```tsx
import { DataTable, FilterLiveSearch, UrlField } from "@/components/admin";
import { ListLive } from "@/components/realtime";

export const TileserverList = () => (
  <ListLive aside={
    <div className="flex w-56 flex-col gap-4">
      <FilterLiveSearch source="q" />
    </div>
  }>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="url" label="URL">
        <UrlField source="url" />
      </DataTable.Col>
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 4: Create `apps/web/src/resources/tileserver/tileserver-create.tsx`**

The `TileserverPreview` sub-component watches the `url` field and shows a live `BaseMap` when a URL is present.

```tsx
import { Create, SimpleForm, TextInput } from "@/components/admin";
import { BaseMap } from "@/components/leaflet";
import { useWatch } from "react-hook-form";
import { required } from "ra-core";
import { DEFAULT_TILE_URL } from "@/lib/constants";

function TileserverPreview() {
  const url = useWatch({ name: "url" }) as string | undefined;
  const tileUrl = url && url.trim().length > 0 ? url : null;
  if (!tileUrl) {
    return (
      <div className="flex h-40 items-center justify-center rounded-md border bg-muted/30 text-sm text-muted-foreground">
        Enter a tile URL above to preview the map.
      </div>
    );
  }
  return <BaseMap tileUrl={tileUrl} height={300} />;
}

export const TileserverFormFields = () => (
  <>
    <TextInput source="name" validate={required()} />
    <TextInput source="url" label="Tile URL" validate={required()} />
    <TileserverPreview />
  </>
);

export const TileserverCreate = () => (
  <Create>
    <SimpleForm>
      <TileserverFormFields />
    </SimpleForm>
  </Create>
);
```

- [ ] **Step 5: Create `apps/web/src/resources/tileserver/tileserver-edit.tsx`**

```tsx
import { SimpleForm } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { TileserverFormFields } from "./tileserver-create";

export const TileserverEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      <TileserverFormFields />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 6: Create `apps/web/src/resources/tileserver/tileserver-show.tsx`**

```tsx
import { TextField, UrlField } from "@/components/admin";
import { ShowLive } from "@/components/realtime";
import { BaseMap } from "@/components/leaflet";
import type { ShowProps } from "@/components/admin/views/show";
import { useRecordContext } from "shadmin-core";
import { DEFAULT_TILE_URL } from "@/lib/constants";

function TileserverMapPreview() {
  const record = useRecordContext<{ url?: string }>();
  const tileUrl = record?.url ?? DEFAULT_TILE_URL;
  return <BaseMap tileUrl={tileUrl} height={300} />;
}

export const TileserverShow = (props: Pick<ShowProps, "id">) => (
  <ShowLive {...props}>
    <div className="flex flex-col gap-4 p-4">
      <TextField source="name" />
      <UrlField source="url" label="Tile URL" />
      <TileserverMapPreview />
    </div>
  </ShowLive>
);
```

- [ ] **Step 7: Create `apps/web/src/resources/tileserver/index.ts`**

```ts
import { Map } from "lucide-react";
import { TileserverList } from "./tileserver-list";
import { TileserverEdit } from "./tileserver-edit";
import { TileserverCreate } from "./tileserver-create";
import { TileserverShow } from "./tileserver-show";

export const tileserver = {
  name: "tileserver",
  list: TileserverList,
  edit: TileserverEdit,
  create: TileserverCreate,
  show: TileserverShow,
  recordRepresentation: "name",
  icon: Map,
};
```

- [ ] **Step 8: Run browser tests (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: tileserver-list test PASS.

- [ ] **Step 9: Commit**

```bash
git add apps/web/src/resources/tileserver/
git commit -m "$(cat <<'EOF'
feat(web): add tileserver resource with live BaseMap preview in edit/create

TileserverPreview watches the url field via useWatch and renders a BaseMap
tile preview. Show uses useRecordContext to render the stored url.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: Plugins resource (list + edit-only; composite id)

**Files:**
- Create: `apps/web/src/resources/plugins/plugins-list.tsx`
- Create: `apps/web/src/resources/plugins/plugins-edit.tsx`
- Create: `apps/web/src/resources/plugins/plugins-list.browser.test.tsx`
- Create: `apps/web/src/resources/plugins/index.ts`

**Interfaces:**
- Consumes: `MonacoJsonInput` from `@/components/monaco`; `BooleanInput`, `BooleanField`, `TextInput` from `@/components/admin`; `ListLive`, `EditLive` from `@/components/realtime`. No `create` or `show` (edit-only per contract). Composite id `kind:name` is already handled in `data-provider.ts` `itemPath` function.
- Produces: `plugins` resource descriptor (list + edit only — no `create`, no `show`).

- [ ] **Step 1: Write the failing browser test**

```tsx
// apps/web/src/resources/plugins/plugins-list.browser.test.tsx
import { describe, expect, it } from "vitest";
import { render } from "vitest-browser-react";
import { AdminContext } from "@/components/admin";
import { ResourceContextProvider, testDataProvider } from "shadmin-core";
import type { AuthProvider } from "shadmin-core";
import { PluginsList } from "@/resources/plugins/plugins-list";

const fakeRows = [
  { id: "scanner:rdm-bridge", name: "rdm-bridge", kind: "scanner", enabled: true, version: "1.0.0" },
  { id: "export:geojson", name: "geojson", kind: "export", enabled: false, version: "0.9.0" },
];

const stubDataProvider = {
  ...testDataProvider({
    getList: async () => ({ data: fakeRows as any, total: fakeRows.length }),
    getMany: async () => ({ data: [] as any }),
  }),
  subscribe: () => () => undefined,
};

const stubAuthProvider: AuthProvider = {
  login: async () => undefined,
  logout: async () => undefined,
  checkAuth: async () => undefined,
  checkError: async () => undefined,
  getPermissions: async () => "admin",
  canAccess: async () => true,
};

describe("PluginsList", () => {
  it("renders plugin rows with name, kind, and enabled status", async () => {
    const screen = render(
      <AdminContext dataProvider={stubDataProvider} authProvider={stubAuthProvider}>
        <ResourceContextProvider value="plugins">
          <PluginsList />
        </ResourceContextProvider>
      </AdminContext>,
    );
    await expect.element(screen.getByText("rdm-bridge")).toBeVisible();
    await expect.element(screen.getByText("geojson")).toBeVisible();
  });
});
```

- [ ] **Step 2: Run test to verify it fails (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "plugins|FAIL" | tail -10
```

- [ ] **Step 3: Create `apps/web/src/resources/plugins/plugins-list.tsx`**

```tsx
import { DataTable, BooleanField, FilterLiveSearch } from "@/components/admin";
import { ListLive } from "@/components/realtime";

export const PluginsList = () => (
  <ListLive aside={
    <div className="flex w-56 flex-col gap-4">
      <FilterLiveSearch source="q" />
    </div>
  }>
    <DataTable>
      <DataTable.Col source="name" />
      <DataTable.Col source="kind" />
      <DataTable.Col source="enabled" label="Enabled">
        <BooleanField source="enabled" />
      </DataTable.Col>
      <DataTable.Col source="version" />
    </DataTable>
  </ListLive>
);
```

- [ ] **Step 4: Create `apps/web/src/resources/plugins/plugins-edit.tsx`**

Disabled inputs for metadata fields; only `enabled`, `description`, and `args_default` are editable:

```tsx
import { SimpleForm, TextInput, BooleanInput } from "@/components/admin";
import type { EditProps } from "@/components/admin/views/edit";
import { EditLive } from "@/components/realtime";
import { MonacoJsonInput } from "@/components/monaco";

export const PluginsEdit = (props: Pick<EditProps, "id">) => (
  <EditLive {...props}>
    <SimpleForm>
      {/* Read-only metadata — disabled to signal non-editable */}
      <TextInput source="name" disabled />
      <TextInput source="kind" disabled />
      <TextInput source="version" disabled />
      <TextInput source="entrypoint" disabled />
      <TextInput source="interpreter" disabled />
      <TextInput source="protocol" disabled />
      {/* Editable fields */}
      <BooleanInput source="enabled" />
      <TextInput source="description" />
      <MonacoJsonInput
        source="args_default"
        label="Default Arguments (JSON)"
        height={300}
      />
    </SimpleForm>
  </EditLive>
);
```

- [ ] **Step 5: Create `apps/web/src/resources/plugins/index.ts`**

```ts
import { Puzzle } from "lucide-react";
import { PluginsList } from "./plugins-list";
import { PluginsEdit } from "./plugins-edit";

export const plugins = {
  name: "plugins",
  list: PluginsList,
  edit: PluginsEdit,
  // No create/show — plugins are managed by the server, not user-created.
  recordRepresentation: "name",
  icon: Puzzle,
};
```

- [ ] **Step 6: Run browser tests (background)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | grep -E "FAIL|PASS|✓|×" | tail -20
```

Expected: plugins-list test PASS.

- [ ] **Step 7: Commit**

```bash
git add apps/web/src/resources/plugins/
git commit -m "$(cat <<'EOF'
feat(web): add plugins resource (list + edit-only, composite id)

Edit exposes only enabled/description/args_default; entrypoint/interpreter/
protocol/version disabled. Composite id kind:name handled by dataProvider.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 9: Register all resources in `App.tsx` (Geo + Config groups)

**Files:**
- Modify: `apps/web/src/App.tsx`

**Interfaces:**
- Consumes: `route` from `@/resources/route`; `project` from `@/resources/project`; `property` from `@/resources/property`; `tileserver` from `@/resources/tileserver`; `plugins` from `@/resources/plugins`.
- Produces: A fully wired `Admin` with two resource groups: `Geo` (geofence + route) and `Config` (project + property + tileserver + plugins).

- [ ] **Step 1: Update `apps/web/src/App.tsx`**

```tsx
import { Admin, Resource, Layout } from "@/components/admin";
import { dataProvider } from "@/data-provider";
import { authProvider } from "@/auth-provider";
import { geofence } from "@/resources/geofence";
import { route } from "@/resources/route";
import { project } from "@/resources/project";
import { property } from "@/resources/property";
import { tileserver } from "@/resources/tileserver";
import { plugins } from "@/resources/plugins";
import { Dashboard } from "@/dashboard/dashboard";
import { KojiAppBar } from "@/components/app-bar";
import { PasswordLoginPage } from "@/components/login/password-login-page";

const KojiLayout = (props: React.ComponentProps<typeof Layout>) => (
  <Layout {...props} appBar={KojiAppBar} />
);

function App() {
  return (
    <Admin
      dataProvider={dataProvider}
      authProvider={authProvider}
      layout={KojiLayout}
      dashboard={Dashboard}
      title="Kōji Admin"
      loginPage={PasswordLoginPage}
      disableTelemetry
    >
      <Resource {...geofence} group="Geo" />
      <Resource {...route} group="Geo" />
      <Resource {...project} group="Config" />
      <Resource {...property} group="Config" />
      <Resource {...tileserver} group="Config" />
      <Resource {...plugins} group="Config" />
    </Admin>
  );
}

export default App;
```

- [ ] **Step 2: Run typecheck**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck 2>&1 | tail -20
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add apps/web/src/App.tsx
git commit -m "$(cat <<'EOF'
feat(web): register all slice-2 resources in App (Geo: geofence+route; Config: project+property+tileserver+plugins)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: Full-suite verify (typecheck + unit + browser + Rust build)

**Files:** None created. Verification only.

**Interfaces:**
- Consumes: all prior tasks' outputs.
- Produces: clean check gates or a prioritized list of fixes.

- [ ] **Step 1: Run frontend typecheck + unit tests in parallel**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run typecheck 2>&1 | tail -20
```

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test 2>&1 | tail -20
```

Expected: both pass.

- [ ] **Step 2: Run full browser test suite (background — ~100s)**

```bash
cd /Users/rin/GitHub/Koji/apps/web && bun run test:browser 2>&1 | tail -30
```

Expected: all tests PASS (geofence + route + project + property + tileserver + plugins + property-value-input).

- [ ] **Step 3: Run Rust build + existing tests**

```bash
cd /Users/rin/GitHub/Koji && cargo build --workspace 2>&1 | tail -20
```

```bash
cd /Users/rin/GitHub/Koji && cargo test --workspace --lib 2>&1 | tail -20
```

Expected: no errors, all lib tests pass.

- [ ] **Step 4: Fix any failures**

If typecheck errors: trace the error to the task that introduced it and fix in place, then re-run typecheck.

If browser tests fail: check the error message. Common issues — missing import, wrong prop name, wrong export. Fix the specific file; re-run only the failing test file first: `bun run test:browser -- <filename>`.

If Rust errors: `cargo check --package koji-service` to isolate, fix, re-run.

- [ ] **Step 5: Commit any fixes**

```bash
git add -p   # stage only fix files
git commit -m "$(cat <<'EOF'
fix(web): resolve typecheck/test failures from slice-2 integration

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>
EOF
)"
```

---

## Self-Review

### Spec Coverage

| Contract section | Task(s) |
|-----------------|---------|
| §1 `GET /internal/routes` row-list | Task 1 |
| §1 `internal::scope()` swap | Task 1 |
| §1 `internal_item_scope()` for routes | Task 1 |
| §1 DB-gated test | Task 1 |
| §2 App.tsx resource registration groups | Task 9 |
| §2 `RESOURCE_MAP` route geo:true | Task 4 |
| §3 route resource (list/edit/create/show, MultiPointInput) | Task 4 |
| §3 project resource (list/edit/create/show, ReferenceArrayInput) | Task 5 |
| §3 property resource (list/edit/create/show, PropertyValueInput) | Task 6 |
| §3 tileserver (list/edit/create/show, BaseMap preview) | Task 7 |
| §3 plugins (list + edit only, composite id, MonacoJsonInput) | Task 8 |
| §4 `PropertyValueInput` component | Task 3 |
| §4 `PROPERTY_CATEGORIES` constant | Task 2 |
| §4 `categorySource` prop | Task 3 |
| §4 Non-destructive category change (keep value) | Task 3 — `useWatch` watches the field; the value field is managed by its own `useInput`; switching category re-renders the input but RHF retains the existing value for `source` until the user changes it. The helper note is included via `helperText`. |
| §5 bun / @/ / TDD / browser tests stub provider | All tasks |

### Placeholder Check

No "TBD", "similar to", or "add the rest" patterns present. Every step shows actual code.

### Type Consistency

- `RouteRow` (Rust) fields used consistently across test + handler: `id: i64`, `name: String`, `description: Option<String>`, `mode: String`, `geofence_id: i64`, `points: usize`.
- `MultiPointInput` props: `source`, `tileUrl`, `height` — matches `ShapeInputShellProps` which extends `BaseInputProps`.
- `MultiPointField` props: `source`, `tileUrl`, `height` — matches `ShapeFieldShellProps`.
- `BaseMap` props: `tileUrl`, `height`, `zoom`, `defaultCenter` — matches `BaseMapWrapperProps`.
- `ReferenceArrayInput` props: `source`, `reference` (children default to `AutocompleteArrayInput`) — confirmed in source.
- `GeoJsonField` in geofence-show uses `source`, `tileUrl`, `height` — route-show correctly uses `MultiPointField` instead (points, not polygons).
- `ROUTE_MODES` = `GEOFENCE_MODES` (same DB enum, per contract §3 note).
- `PropertyValueInput` exports `{ source, categorySource? }` — used with those exact props in Task 6.
- Browser test stub pattern: `testDataProvider({...}) + subscribe: () => () => undefined` — matches all geofence test files exactly.

### Notes for Implementer

0. **VERIFY-FIRST (two assumptions that silently break if wrong):**
   - **Route `mode` enum** — Task 2 sets `ROUTE_MODES = GEOFENCE_MODES` assuming the route table's `mode` column is the SAME DB enum as geofence (`pokemon|fort|quest|unset`). This is NOT confirmed — the v1 route modes were `circle_*`. **Before Task 2/4, inspect the route entity (`crates/koji-db/src/db/route.rs` / the route `mode` column / migrations) and use the ACTUAL enum values.** If route modes differ, define a real `ROUTE_MODES` list instead of aliasing `GEOFENCE_MODES`.
   - **Route `paginate` `points` field** — Task 1's `RouteRow` reshape reads `r.get("points")`. Confirm `koji_db::db::route::Query::paginate` actually emits a `points` (coordinate count) field per row; if it's named differently or absent, adapt the reshape (the geofence row handler did the same verification against its paginate output). The DB test asserts `points >= 2`, so a wrong field name fails the test (good — it's caught), but verify up front to save a cycle.

1. **`ColorInput` vendoring in Task 2**: The shadcn-admin-kit version depends on `@/components/ui/color-picker` which does not yet exist in Koji web. The plan uses a native `<input type="color">` fallback to avoid blocking the slice. If the full oklch picker is needed, it can be vendored as a separate follow-on. The plan's Task 2 Step 1 includes a verification check for missing deps.

2. **`MonacoJsonInput` dependency**: `monaco-editor` is NOT in Koji web's `package.json`. Task 2 Step 4 adds it via `bun add monaco-editor @monaco-editor/react`. The plan uses `@monaco-editor/react` (the lighter React wrapper) rather than the full shadcn-admin-kit lazy-load chain, to avoid vendoring additional internal modules.

3. **`ChipField`**: Used in `ProjectShow` for `ReferenceArrayField` children. Verify it is exported from `@/components/admin` (it appears in `admin/index.ts` as `chip-field`). If missing, replace with `<TextField source="name" />` inside `SingleFieldList`.

4. **`UrlField`**: Used in `TileserverList` and `TileserverShow`. Verify export from `@/components/admin` (present as `url-field` in `admin/index.ts`).

5. **Realtime note on plugins**: The contract says "already emit events" — the macro-generated handlers already emit realtime events, so `ListLive` should work for plugins as-is.
