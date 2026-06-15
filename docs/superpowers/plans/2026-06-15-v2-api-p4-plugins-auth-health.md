# v2 API — Phase 4: Plugins re-path + Auth + Health — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development / executing-plans + test-driven-development. Steps use `- [ ]`.

**Goal:** Finish the v2 surface's smaller corrections: plugin ids as path segments (`/plugins/{kind}/{name}`), a proper `/api/v2/auth/{login,logout,me}`, and `/healthz` + `/readyz` liveness/readiness. (Code-first OpenAPI is **resequenced to a final phase P7** — annotate once the surface is frozen post-teardown.)

**Architecture:** Three small, independent corrections. plugins/auth/health each adopt `ServiceError`. The old `/config/{login,logout,config,nominatim}` stay in place during the transition (removed in P6); P4 *adds* the `/api/v2/auth/*` surface alongside.

**Tech Stack:** Rust, actix-web, actix-session, sea-orm, serde.

## Delegate-mode decisions (recorded)

- **OpenAPI moved out of P4 → P7** (final). The static `crates/koji-service/openapi.yaml` is stale during P4–P6; no external consumer depends on it yet, and utoipa annotation is cleanest once the surface is final.
- **`/config/*` kept** through P4 (the admin UI still logs in via `/config/login`); `/api/v2/auth/*` is added in parallel. P6 removes `/config/*`.
- **`/readyz` = a DB ping** (`db.koji.ping()`), `200`/`503`. `/healthz` stays (process liveness). The bare `/health` alias is dropped now (its v1 sibling dies in P6).
- **DB-test strategy:** no DB here → compile/clippy + pure-logic tests; live auth/health/plugins on a real-DB run.

---

## Task 1: plugins — `{kind}:{name}` → `/{kind}/{name}`

**Files:** Modify `crates/koji-service/src/public/v2/plugins.rs`. Read it first.

- [ ] **Step 1: Re-path the per-plugin handlers**

The `get_one`/`update`/`remove` handlers currently take `web::Path<String>` = `"{kind}:{name}"` and split on `:`. Change to `web::Path<(String, String)>` = `(kind, name)`; drop the colon-split + its malformed-id `400`. Adopt `ServiceError`: unknown plugin ⇒ `ServiceError::NotFound { field: "plugin", message }`; an id with no disk manifest ⇒ `ServiceError::Unprocessable { field: Some("id".into()), … }` (the existing 422 case). `list` is unchanged except `-> Result<_, ServiceError>`.

Update `scope()` to `web::scope("/plugins")` with `web::resource("")` (GET list) + `web::resource("/{kind}/{name}")` (GET/PATCH/DELETE).

- [ ] **Step 2: Compile + commit**

Run: `cargo test -p koji-service --no-run 2>&1 | tail -6`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -6`

```bash
git add crates/koji-service/src/public/v2/plugins.rs
git commit -m "feat(koji-service): plugins keyed by /{kind}/{name} path segments, ServiceError"
```

---

## Task 2: `/api/v2/auth/{login,logout,me}`

**Files:** Create `crates/koji-service/src/public/v2/auth.rs`; modify `mod.rs`. Read `private/misc.rs` (`login`/`logout`) + `utils/auth.rs` (`public_validator`) for the exact session key + `KOJI_SECRET` logic.

- [ ] **Step 1: The three handlers + scope (TDD what's testable)**

```rust
// POST /api/v2/auth/login  { "password": "..." }  → 200 (sets session "logged_in") / 401
// POST /api/v2/auth/logout → 204 (session.clear())
// GET  /api/v2/auth/me     → 200 { "authenticated": bool, "via": "session"|"bearer"|"open"|"none" }
```

- `login`: compare `password` to `KOJI_SECRET` (env). Match ⇒ `session.insert("logged_in", true)` + `ApiResponse::success(json!({"authenticated": true}))`; mismatch ⇒ `ServiceError::NotFound`? No — use a dedicated `401`: return `ApiResponse::error(StatusCode::UNAUTHORIZED, "invalid password", Some("unauthorized".into()), None)` (auth failure isn't a `ServiceError` variant; keep it an explicit 401). Mirror `misc.rs::login`'s secret comparison exactly.
- `logout`: `session.clear()` ⇒ `HttpResponse::NoContent().finish()` (204; was a 302 in `/config/logout`).
- `me`: introspect — `session.get::<bool>("logged_in")` true ⇒ `via:"session"`; else if `KOJI_SECRET` empty ⇒ `via:"open", authenticated:true`; else if the `Authorization: Bearer <KOJI_SECRET>` header matches ⇒ `via:"bearer"`; else `authenticated:false, via:"none"`. (Reuse the `public_validator` logic.)

`pub(crate) fn scope() -> actix_web::Scope` = `web::scope("/auth")` with `/login` (POST), `/logout` (POST), `/me` (GET).

Add a pure-logic test for the `me` "via" decision if it can be factored from the actix `Session`/`HttpRequest` (e.g. a `fn decide_via(logged_in: bool, secret: &str, bearer: Option<&str>) -> &'static str`), tested directly.

- [ ] **Step 2: Compile + commit**

```bash
git add crates/koji-service/src/public/v2/auth.rs crates/koji-service/src/public/v2/mod.rs
git commit -m "feat(koji-service): /api/v2/auth/{login,logout,me}"
```

---

## Task 3: `/readyz` + drop bare `/health`

**Files:** Modify `crates/koji-service/src/lib.rs`. Read the current `/health`/`/healthz` registrations.

- [ ] **Step 1: Add readiness, prune the alias**

- Add `async fn readyz(db: web::Data<KojiDb>) -> HttpResponse` that pings the DB (`db.koji.ping().await`): `Ok` ⇒ `200` empty; `Err` ⇒ `503` (`HttpResponse::ServiceUnavailable().finish()`).
- Register `web::resource("/readyz").route(web::get().to(readyz))` (unauthenticated).
- Keep `/healthz`. Remove the bare `/health` alias resource (the `/api/v1/health` sibling is removed in P6).

- [ ] **Step 2: Wire the auth + plugins scopes**

- In the `/v2` scope, the plugins scope() registration is unchanged (still `public::v2::plugins::scope()`), now serving `/{kind}/{name}`.
- Add `.service(public::v2::auth::scope())` to the `/v2` scope.

- [ ] **Step 3: Build + commit**

Run: `cargo build -p koji 2>&1 | tail -6`, `cargo clippy -p koji-service --all-targets 2>&1 | tail -6`

```bash
git add crates/koji-service/src/lib.rs
git commit -m "feat(koji-service): /readyz DB-readiness, drop bare /health, mount /api/v2/auth"
```

---

## Task 4: Verification gate

- [ ] **Step 1: Build / lint / test**

```bash
cargo clippy -p koji-service --all-targets 2>&1 | tail -10
cargo test -p koji-service --lib 2>&1 | tail -6
cargo build -p koji 2>&1 | tail -3
```
Expected: clean; lib tests pass (P3 = 74 + any new); builds.

- [ ] **Step 2: Confirm the surface**

```bash
rg -n "auth::scope|readyz|\"/\\{kind\\}/\\{name\\}\"" crates/koji-service/src/lib.rs crates/koji-service/src/public/v2/plugins.rs crates/koji-service/src/public/v2/auth.rs
rg -n "\\{kind\\}:\\{name\\}|split.*':'|\"/health\"" crates/koji-service/src/public/v2/plugins.rs crates/koji-service/src/lib.rs || echo "CLEAN: colon-id + bare /health gone"
```
Expected: the new registrations present; `CLEAN` for the removed bits.

- [ ] **Step 3: DB-run note**

Live smoke: `GET /readyz` → 200 (DB up) / 503 (down); `POST /api/v2/auth/login {"password":"…"}` → 200 + cookie / 401; `GET /api/v2/auth/me` → `{authenticated, via}`; `GET /api/v2/plugins/routing/tsp` → plugin view (404 unknown). Recorded in the spec's P4 testing notes.

---

## Self-Review

**Spec coverage:** plugins `/{kind}/{name}` (§4.6, §A13) ✓ · `/api/v2/auth/{login,logout,me}` (§4.7, §A14) ✓ · `/healthz`+`/readyz` (§4.8) ✓ · `ServiceError` adoption ✓. OpenAPI (§6.5/A15) explicitly resequenced to P7 (documented). `/config/*` retained for transition (removed P6).

**Placeholder scan:** none — handlers, status codes, the `via` decision, and the DB ping are specified; "read misc.rs/auth.rs" is a grounding step.

**Type consistency:** `ServiceError::{NotFound,Unprocessable}`, `ApiResponse::{success,error}`, `KojiDb::koji` (`DatabaseConnection::ping`), session key `"logged_in"` — consistent with P0–P3 + the auth scout's map.
