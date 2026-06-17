# v2 API — Phase 8 (final): Code-first OpenAPI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans. Verified by the crate build + `ApiDoc::openapi()` producing the live routes. Steps use `- [ ]`.

**Goal:** Replace the stale hand-maintained `crates/koji-service/openapi.yaml` with a **utoipa code-first** document generated from the v2 handlers, so the served `/api/v2/openapi.yaml` is accurate and can't drift. The P0 `ApiDoc` scaffold (`utils/openapi.rs`) already exists — this populates it.

**Architecture:** Add `#[utoipa::path(...)]` to each v2 handler and `#[derive(ToSchema)]` to the request/response DTOs + the envelope types, register them in `ApiDoc` (`paths(...)` + `components(schemas(...))`), and serve `ApiDoc::openapi().to_yaml()` from `openapi_spec`. Delete the static `openapi.yaml` + its `include_str!`.

**Tech Stack:** Rust, utoipa 5, actix-web.

## Delegate-mode decisions (recorded)

- **Code-first (utoipa), not a hand-rewrite** — drift-proof; the scaffold exists.
- **Scope:** annotate the whole v2 surface. If the sweep proves too large to finish cleanly, annotate the **core** (envelope/error schemas + jobs + the 5 CRUD resources + geometry/s2/golbat-data) and flag the remainder (plugins/auth/health/nominatim) as `// TODO(openapi)` — a partial-but-accurate generated doc still beats the static lie. Report exactly what's covered.
- **Documentation-only phase** — no runtime behavior changes. Verified by the doc building + containing the routes.

---

## Task 1: Schemas — `ToSchema` on the types

**Files:** `utils/api_response.rs` (envelope), `requests/ops.rs` + `requests/groups.rs` (calc), the per-resource DTOs (`public/v2/resources.rs` generated DTOs, `geofences.rs`/`routes.rs` DTOs), `s2`/`geometry`/`golbat_data` request types, `plugins.rs` `Plugin`/`PluginPatch`.

- [ ] **Step 1** — Derive `utoipa::ToSchema` on: `ApiError`, `Meta`, and the per-endpoint payload/DTO types (`CalcJobRequest`, the arg-groups, `CreateGeofence`/`PatchGeofence`/`CreateRoute`/`PatchRoute`, the generated `Create*`/`Patch*` for plain resources, `CoverageArgs`/`BoundsArg`, `GeometryArgs`/`AreaReq`, the golbat-data `AreaReq`/`BboxInput`, `Plugin`/`PluginPatch`, `ConfigResponse`, `JobRecord`). For the macro-generated DTOs (`koji_resource!`), add `ToSchema` to the macro's emitted `#[derive(...)]` list.
- [ ] **Step 2** — `cargo build -p koji-service` green. Commit.

```bash
git commit -am "feat(koji-service): derive utoipa::ToSchema on v2 DTOs + envelope"
```

---

## Task 2: Paths — `#[utoipa::path]` on the handlers

**Files:** all `public/v2/*.rs` handler modules.

- [ ] **Step 1** — Add `#[utoipa::path(...)]` above each v2 handler: method, path (full `/api/v2/...`), `params`, `request_body` (the DTO), `responses` (the status codes + the envelope/`ApiError` schema, `tag`). Work module-by-module (jobs, resources, geofences, routes, geometry, s2, golbat_data, plugins, auth, config, nominatim), building after each so the compiler guides the annotation.
- [ ] **Step 2** — `cargo build -p koji-service` green. Commit (can be one commit or per-module).

```bash
git commit -am "feat(koji-service): #[utoipa::path] on v2 handlers"
```

---

## Task 3: Register + serve; delete the static yaml

**Files:** `utils/openapi.rs` (`ApiDoc`), `lib.rs` (`openapi_spec`), delete `crates/koji-service/openapi.yaml`.

- [ ] **Step 1** — Populate `#[openapi(paths(...), components(schemas(...)), tags(...))]` on `ApiDoc` with every annotated handler + schema. (Use `nest`/`OpenApi::merge` if it keeps the list manageable.)
- [ ] **Step 2** — Rewrite `openapi_spec` to serve the generated doc: `ApiDoc::openapi().to_yaml()` (or `to_pretty_json` if yaml isn't available in the installed utoipa) with the `text/yaml` content type. Remove the `const OPENAPI_YAML: &str = include_str!("../openapi.yaml")`.
- [ ] **Step 3** — Delete `crates/koji-service/openapi.yaml`.
- [ ] **Step 4** — `cargo build -p koji` green. Commit.

```bash
git rm crates/koji-service/openapi.yaml
git commit -am "feat(koji-service): serve generated OpenAPI; delete static yaml"
```

---

## Task 4: Verification gate

- [ ] **Step 1** — Add a test (`utils/openapi.rs`) asserting `ApiDoc::openapi()` builds and contains representative paths (`/api/v2/jobs`, `/api/v2/geofences`, `/api/v2/geometry/convert`, `/api/v2/s2/{level}`, `/api/v2/auth/login`) and the `ApiError` schema.

```rust
#[test]
fn openapi_covers_core_surface() {
    let doc = ApiDoc::openapi();
    let paths = doc.paths.paths.keys().cloned().collect::<Vec<_>>();
    for p in ["/api/v2/jobs", "/api/v2/geofences", "/api/v2/geometry/convert"] {
        assert!(paths.iter().any(|k| k == p), "missing {p}");
    }
}
```

- [ ] **Step 2** — Build/lint/test:

```bash
cargo clippy -p koji-service --all-targets 2>&1 | rg "error|warning: unused" | rg -v "num-bigint-dig|proc-macro-error2" | head
cargo test -p koji-service --lib 2>&1 | rg "test result"
cargo build -p koji 2>&1 | rg "Finished|error"
```
Expected: clean; tests pass (P7 = 84 + the openapi test + any new); builds.

- [ ] **Step 3** — Confirm the static yaml is gone + generation is wired:

```bash
test -f crates/koji-service/openapi.yaml && echo "STILL THERE" || echo "CLEAN: static yaml deleted"
rg -n "ApiDoc::openapi|include_str!.*openapi" crates/koji-service/src/lib.rs crates/koji-service/src/utils/openapi.rs
```
Expected: `CLEAN`; `openapi_spec` serves `ApiDoc::openapi()`, no `include_str!`.

- [ ] **Step 4** — Commit any final tidy.

---

## Self-Review

**Spec coverage:** code-first OpenAPI (§6.5, §A15) — generated from the handlers, served, static lie deleted. Drift-proof.

**Placeholder scan:** the partial-fallback names the exact remainder to flag; otherwise every type/handler to annotate is listed.

**Consistency:** schemas/paths reflect the surface as built in P1–P7 (single `/api/v2`, the v2 envelope, page/per_page, `?format=`). If `to_yaml` isn't in the installed utoipa, `to_pretty_json` is the named fallback.
