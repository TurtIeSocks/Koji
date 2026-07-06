# Projects v2 — Webhook-Backed Push Design

**Date:** 2026-07-06
**Status:** Approved (brainstormed with user, all decisions signed off)
**Supersedes:** the v1 project push mechanism (`api_endpoint` / `api_key` / `golbat` columns)
**Related specs:** `2026-05-29-koji-v2-p3b-events-design.md` (webhook/event infra this builds on), `2026-06-15-v2-api-redesign-design.md` (resource conventions A2/A5/A9/A10)

## 1. Problem

The v1 `project` table conflates two jobs:

1. **Grouping** — a named collection of geofences, used as an export scope
   (`/geofence/{format}/{project}`).
2. **Push target** — `api_endpoint` + `api_key` describe an external system to
   ping after a sync, and `golbat` (né `scanner`) marks "the one project whose
   fences sync to the scanner DB".

The push half is ghetto:

- `api_key` is a magic `HeaderName:SecretValue` string parsed at send time;
  every integration has its own header format (`x-golbat-secret:…`,
  `X-Poracle-Secret:…`, `react-map-secret:…`).
- Delivery is a fire-and-forget GET with no retry, no backoff, no delivery log.
- `golbat` is a boolean whose "only one row may be true" invariant is enforced
  by convention only — and in v2 it is **vestigial**: `koji-golbat` is
  read-only by design, the v1 save-scanner write path was never ported, and
  `get_golbat_project()` (`crates/koji-db/src/db/project.rs:218`) has zero
  callers.

Meanwhile v2 already ships a proper delivery engine (P3b): `event_outbox` +
`webhook_subscription` tables, a dispatcher with `FOR UPDATE SKIP LOCKED`
claiming, exponential backoff, dead-lettering after 8 attempts, HMAC-SHA256
signing, and ULID event IDs — but it is unwired: no CRUD surface, no admin UI,
and only two dragonite-internal topics ever emit.

## 2. Decisions (locked)

| # | Decision |
|---|----------|
| D1 | Project becomes pure grouping; `webhook_subscription` gains an optional `project_id` FK. Push config lives on subscriptions. |
| D2 | Two delivery modes on a subscription: `event` (HMAC-signed JSON POST, the P3b contract) and `ping` (legacy reload: configurable method, custom headers, empty body, no HMAC). One dispatcher, one retry/dead-letter path for both. |
| D3 | The `golbat` column is dropped entirely, along with dead `get_golbat_project()`. A future golbat reload ping is just a webhook row. Scanner-DB writes remain out of scope (Epic 19). |
| D4 | Topic vocabulary: five topics (§4). Matching = topic filter AND project filter. |
| D5 | Existing v1 push config is auto-migrated into webhook rows; the three project columns are then dropped. |
| D6 | Webhooks are a top-level resource: `/api/v2/webhooks` as the 6th `koji_resource!`, with a `?projectId=` list filter. No nested routes. |

## 3. Schema

### `project` — after

```
id, name, description, created_at, updated_at
```

Dropped: `api_endpoint`, `api_key`, `golbat`. The `geofence_project` join table
is unchanged.

### `webhook_subscription` — five new columns

| Column | Type | Notes |
|---|---|---|
| `name` | `VARCHAR(255) NOT NULL` | Human label for the admin UI ("ReactMap reload"). |
| `project_id` | `INT UNSIGNED NULL`, FK → `project.id` `ON DELETE CASCADE` | `NULL` = global subscription. Cascade: a project-scoped reload ping is meaningless without its project. |
| `mode` | `ENUM('event','ping') NOT NULL DEFAULT 'event'` | `event` = signed JSON POST; `ping` = legacy reload. |
| `method` | `ENUM('GET','POST') NOT NULL DEFAULT 'GET'` | Ping mode only; event mode always POSTs. |
| `headers` | `JSON NULL` | Custom header map (`{"X-Poracle-Secret": "…"}`), applied in **both** modes. Replaces the `HeaderName:SecretValue` string hack. |

Existing columns unchanged: `url`, `secret`, `topics`, `active`, timestamps.

## 4. Events

### Topics and emit sites

| Topic | Emitted when | Payload |
|---|---|---|
| `project.updated` | project PATCH (metadata) | project id, name |
| `project.deleted` | project DELETE | project id, name |
| `project.geofences_changed` | membership changes: bulk assign, import wizard, `upsert_related_by_*` | `{projectId, addedIds[], removedIds[]}` |
| `geofence.updated` | geofence create/save (geometry or properties) | geofence id, name, `projectIds[]` |
| `route.updated` | route save | route id, geofence id, `projectIds[]` |

- **One event per project per operation** for membership changes: a bulk
  import of 500 fences produces 1 ping per affected project, not 500.
- `projectIds[]` is resolved from `geofence_project` at emit time and stamped
  into the payload. The `event_outbox` schema is untouched; the matcher reads
  the payload.
- Existing `area.route_updated` / `area.geofence_updated` dragonite topics are
  untouched.

### Matching rule

A subscription receives an event when **both** hold:

1. Topic filter passes: `topics` is empty (subscribe-all) or contains the
   event's topic.
2. Project filter passes: `project_id` is `NULL` (global) or
   `project_id ∈ payload.projectIds` (for `project.*` topics, the payload's
   `projectId`).

Legacy-consumer recipe: `mode=ping` + `project_id` set + empty `topics` =
"ping me when anything in project X changes" — exact v1 semantics.

### Delivery per mode

- `event`: current P3b path — `X-Koji-Signature` (HMAC-SHA256 when `secret`
  set), `X-Koji-Event-Id`, JSON body — plus custom `headers` merged in.
- `ping`: `method` request to `url` with custom `headers`, empty body, no
  HMAC. Still sends `X-Koji-Event-Id` (free idempotency/debugging aid).

Retry, backoff, and dead-lettering come from the existing dispatcher
unchanged. Known ceiling, accepted: retry is all-or-nothing per event across
subscribers, so one flaky consumer re-pings the others. Reload pings are
idempotent, so this is harmless; a `ponytail:` comment at the delivery site
names the upgrade path (per-subscription delivery rows) if it ever matters.

## 5. API surface

- **`/api/v2/webhooks`** — 6th `koji_resource!`: full CRUD, pagination,
  `201 + Location` / `204` / `404` per locked v2 conventions (A9/A10).
  - DTO: `name` (required), `url` (required), `secret?`, `topics?`, `active?`,
    `projectId?`, `mode?`, `method?`, `headers?`.
  - List filter: `?projectId=`.
- **`POST /api/v2/webhooks/{id}/test`** — manual fire. Ping mode sends the
  ping; event mode sends a sample signed payload. Returns the delivery result
  inline (upstream status code or error). Replaces the v1 "sync to test"
  workflow and backs the admin-UI test button.
- `secret` is returned in GET responses and stored plaintext in the DB —
  matches v1 `api_key` behavior; the admin surface is bearer-protected and
  Koji is self-hosted/single-admin. Accepted trade-off, recorded here.

## 6. Migration (one sea-orm migration)

1. Add the five columns to `webhook_subscription`.
2. Data move — for each project with a non-empty `api_endpoint`, insert a
   subscription row:
   - `name = "{project.name} reload"`, `url = api_endpoint`, `mode = 'ping'`,
     `method = 'GET'`, `project_id = project.id`, `topics = []`, `active = 1`.
   - `headers` from `api_key`: split on the **first** `:` →
     `{"<left>": "<right>"}`; no `:` → `{"Authorization": "<value>"}`
     (never silently drop config); `NULL`/empty → `NULL`.
3. Drop `api_endpoint`, `api_key`, `golbat` from `project`.

Every documented v1 integration (ReactMap, PoracleJS, rdmGruber, Golbat
reload) survives the upgrade with no re-entry.

Code deletions in the same unit of work: `get_golbat_project()`, the three
entity fields, and the corresponding `CreateProject`/`PatchProject` DTO
fields (macro-regenerated).

## 7. Admin UI (`apps/web`)

- **Webhooks resource**: list (name, url, mode, project reference, active,
  topics), create/edit forms where `mode` toggles the visible fields
  (ping → method + headers; event → secret), and a test button wired to
  `POST /webhooks/{id}/test`.
- **Project show page**: webhooks section filtered by `projectId`, with an
  "add webhook" shortcut that pre-fills the project.
- **Project forms**: `api_endpoint` / `api_key` / `golbat` inputs removed.

## 8. Docs (`apps/web-docs`)

- Integrations page rewritten per consumer: webhook-subscription setup
  (mode/headers recipes) replaces the `api_endpoint`/`api_key` recipes.
- Admin-panel and getting-started pages updated to the new project form and
  webhooks resource.

## 9. Testing

- **koji-events unit**: project-filter matching (global vs scoped, project
  topics vs payload `projectIds[]`), ping delivery (no HMAC, empty body,
  method honored), custom-header merge in both modes.
- **Migration**: `api_key` parse cases — `X:Y`, bare token, empty/NULL.
- **koji-service**: webhook CRUD + `?projectId=` filter, `/test` endpoint,
  event emission on project PATCH / DELETE / membership change, bulk
  membership op emits a single event per project.
- DB-touching tests follow the `*_db.rs` / `KOJI_DB_URL` gating convention.

## 10. Out of scope (separate epics)

- Epic 19: scanner DB writes and dragonite area sync.
- E3.1: project `get_one` hydration fix (geofences[] in the response).
- Per-subscription delivery log / per-subscription retry.
- Payload templates per subscription (rejected as YAGNI in brainstorming).
