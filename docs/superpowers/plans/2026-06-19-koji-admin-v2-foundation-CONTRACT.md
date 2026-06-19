# Koji Admin V2 Foundation — Shared Interface Contract

Locked interfaces between **Plan A (backend)** and **Plan B (frontend)**. Ground-truthed from shadmin
source + the Koji backend trace. Both plans MUST conform; do not redefine these.

## 1. Envelope (existing v2, reused by `/internal`)

`{ "status":"ok", "data": <T>, "meta"?: <Meta> }` | `{ "status":"error", "error": {...} }`
`Meta = { total, page, per_page, total_pages, has_next, has_prev }` (1-based page; per_page clamped [1,500]).

## 2. Private API `/internal` (all behind the same session/bearer auth as `/api/v2`; same-origin; NOT in OpenAPI)

**Forward-aliases** — register the *same actix handler* under `/internal/X` as `/api/v2/X` (no HTTP
redirect):
- `GET /internal/geofences/{id}`, `POST /internal/geofences`, `PATCH /internal/geofences/{id}`,
  `DELETE /internal/geofences/{id}` → public geofence handlers (getOne returns a GeoJSON `Feature`).
- `GET|POST|PATCH|DELETE /internal/projects[/{id}]`, `/internal/properties[/{id}]`,
  `/internal/tile-servers[/{id}]`, `/internal/plugins/{kind}/{name}` (+ `GET /internal/plugins`).
- `GET /internal/config`; `POST /internal/auth/login`; `POST /internal/auth/logout`;
  `GET /internal/auth/me`.

**Bespoke** (own handler/shape):
- `GET /internal/geofences` — **row list** (overrides only the GET-list; GET-one still forwards):
  query `page, per_page, sortBy, order, q` + filters `project, parent, geotype, mode`. Reuses the same
  DB query path (`AdminReqParsed`/`paginate`) the public geofence handler uses.
  ```
  { status:"ok",
    data: GeofenceRow[],
    meta: { total, page, per_page, total_pages, has_next, has_prev } }
  GeofenceRow = {
    id: number, name: string, mode: string,
    parent: number | null, geo_type: string,
    projects: number[], property_count: number
  }
  ```
- `GET /internal/realtime` — WebSocket upgrade (Section 3 below).

**One public-API change** (general correctness, not a client shape): the `koji_resource!` macro `list`
must honor `sortBy/order/q` (today hardcodes `sort_by:"id"`, `order:"ASC"`, `q:""`) by threading
`Pagination → AdminReqParsed`. Public GeoJSON FC reads (`?format=feature`) stay untouched.

## 3. WebSocket protocol (`GET /internal/realtime`)

Auth: validate the session cookie on the upgrade handshake; reject unauthenticated. JSON text frames.

**Client → server** (`ClientFrame`):
```
{ "op":"subscribe",   "topic": <string> }
{ "op":"unsubscribe", "topic": <string> }
{ "op":"publish",     "topic": <string>, "event": { "type": <string>, "payload"?: <any>, "meta"?: {} } }
{ "op":"ping" }
```

**Server → client** (`ServerFrame`):
```
{ "op":"pong" }                                        // reply to ping
{ "topic": <string>, "type": <string>, "payload"?: <any>, "meta"?: {} }   // event (topic & type REQUIRED)
```

Hub behavior: per-connection subscribed-topic set; `subscribe`/`unsubscribe` mutate it; `ping`→`pong`;
`publish` is accepted and re-broadcast to subscribers (protocol completeness — client doesn't use it in
this phase); server-originated events (§4) broadcast to all connections subscribed to the topic.

## 4. Topics + server-emitted events (mirror shadmin's `addEventsForMutations` payloads exactly)

Topics (from shadmin `topics.ts`): `resource/{name}`, `resource/{name}/{id}`, `lock/{name}`,
`lock/{name}/{id}`. Koji-specific: `jobs`, `jobs/{id}`.

`{name}` = the react-admin Resource name: `geofence | route | project | property | tileserver | plugins`.

On a successful mutation the server publishes (emitted inside the `koji_resource!` macro + the
geofence/route handlers):
- **created** → `resource/{name}` : `{ type:"created", payload:{ ids:[id] } }`
- **updated** → `resource/{name}/{id}` : `{ type:"updated", payload:{ id, data } }`
            **and** `resource/{name}` : `{ type:"updated", payload:{ ids:[id] } }`
- **deleted** → `resource/{name}/{id}` : `{ type:"deleted", payload:{ id } }`
            **and** `resource/{name}` : `{ type:"deleted", payload:{ ids:[id] } }`

Job lifecycle (worker status/progress writes):
- `jobs/{id}` : `{ type:"progress"|"status", payload:{ id, status, progress, phase? } }`
- `jobs`      : `{ type:"updated", payload:{ id, status } }`

`useGetListLive` subscribes `resource/{name}` and invalidates on any event → the minimum the server
MUST publish is the `resource/{name}` event with `payload.ids`. The record-topic events feed
`useGetOneLive`.

## 5. Frontend wiring

- `dataProvider = realtimeDataProvider(base, webSocketTransport({ url:'/internal/realtime' }), { locks: inMemoryLockProvider() })`.
  (Option key is **`locks`** — shadmin's real `RealtimeDataProviderOptions` field, `realtime/types.ts:120`.)
  **No `addEventsForMutations`** (server is the single event source — would double-publish).
- `base` = thin ra-data adapter, base URL `/internal`, envelope-unwrapped:
  - `getList`/`getManyReference` (geofence) → `GET /internal/geofences` (row list, `{data, total}` from meta).
  - `getList` (project/property/tileserver/plugins) → forwarded macro CRUD (now server-sorted/filtered).
  - `getOne` (geofence) → `GET /internal/geofences/{id}` (Feature) → map single Feature → record (lossless).
  - `create`/`update`/`delete` → forwarded public CRUD (form posts GeoJSON for geofence).
  - `getMany` → N parallel `getOne` (batch endpoint deferred).
- Resources use `<ListLive>`/`<EditLive>`/`<ShowLive>`; dashboard subscribes `jobs` + live `<Count>`.
