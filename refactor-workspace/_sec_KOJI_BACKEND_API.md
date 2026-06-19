I now have the complete, ground-truthed surface. Producing the dense trace doc.

---

# KOJI BACKEND HTTP API — V2 SURFACE TRACE

Framework: **actix-web 4.13** + **utoipa 5.5** (code-first OpenAPI). Single API version: `/api/v2` (the `mod.rs` "/api/v1 left as-is" comment is **stale** — no v1 scope exists). All v2 handlers return `Result<HttpResponse, ServiceError>` in the `ApiResponse` envelope (`{ status:"ok", data, meta? }` / `{ status:"error", ... }`). Wire DTOs split: geometry CRUD = snake_case; calc + golbat-data bodies = camelCase.

## Wiring (apps/koji-server/src/main.rs → koji_service::start)
- Entry: `apps/koji-server/src/main.rs:17` calls `koji_service::start()`.
- `crates/koji-service/src/lib.rs:302-413` `HttpServer::new` factory. Bind `HOST`/`PORT` (default `0.0.0.0:8080`), `lib.rs:404-410`.
- Middleware: `Logger` `lib.rs:325`, `Compress` `:326`, `SessionMiddleware` (cookie store, `cookie_http_only`, `SameSite::Lax`, `cookie_secure` unless `KOJI_INSECURE_COOKIES` set) `:327-333`; session key from `KOJI_SESSION_KEY` (`:200-218`).
- **Auth**: `HttpAuthentication::with_fn(auth::public_validator)` wraps the **entire `/v2` scope** `lib.rs:344-345`. Validator `crates/koji-service/src/utils/auth.rs:30-58`: pass if session `logged_in` ⇒ OR `KOJI_SECRET` empty (open) ⇒ OR `Bearer == KOJI_SECRET` (constant-time `ct_eq`). So **every `/api/v2/*` route is bearer/session-gated** — including `auth/login` itself (works because empty-secret or a fresh login round-trips through the open/session paths). JSON body limit 50 MB (`:324`).
- **Unauthenticated** (outside the auth scope): `GET /api/v2/openapi.yaml` `lib.rs:337`, `GET /healthz` `:388`, `GET /readyz` `:389`.
- **SPA**: `actix_files::Files::new("/", path())` `lib.rs:390-402`; `path()` = `./dist` in docker else `../client/dist` (`:293-300`); `index_file("index.html")` + `default_handler` falls back to `index.html` for react-router/react-admin client routing.
- **CORS**: none (no `actix_cors`, grep clean). Same-origin only; dev uses Vite proxy.
- **Base path**: hard `/api/v2`. Web-client (`apps/web-client`, Vite) proxies `/api`, `/internal`, `/config` → `http://0.0.0.0:8080` (`vite.config.ts:54-72`) — note `/internal` + `/config` proxies are stale (no such backend routes anymore).
- **SPA data layer**: `apps/web-client/src/pages/admin/dataProvider.ts` — ra-data-simple-rest over `/api/v2`. `RESOURCE_MAP` (`:35-42`): geofence→geofences, route→routes, project→projects, property→properties, tileserver→tile-servers, plugins→plugins. Plugins use composite `kind:name` id (`itemPath` `:140-147`).

## ENDPOINT TABLE

| Method | Path | Area | Req type | Resp type (in envelope) | Status | Auth | File:line |
|---|---|---|---|---|---|---|---|
| GET | `/healthz` | misc | — | empty | 200 | none | lib.rs:388 |
| GET | `/readyz` | misc | — | empty | 200/503 (DB ping) | none | lib.rs:185,389 |
| GET | `/api/v2/openapi.yaml` | misc | — | OpenAPI 3.1 JSON (served as json despite `.yaml`) | 200 | none | lib.rs:169,337 |
| POST | `/api/v2/jobs` | calc | `CalcJobRequest` | `{job_id}` + `Location` hdr | 202/400/500 | bearer/session | jobs.rs:52 |
| GET | `/api/v2/jobs` | calc | query `JobListQuery`(status,page,per_page) | paginated `JobRecord[]`+meta | 200 | b/s | jobs.rs:223 |
| GET | `/api/v2/jobs/{id}` | calc | query `?wait=N` (long-poll ≤290s) | `JobRecord` | 200/404 | b/s | jobs.rs:158 |
| DELETE | `/api/v2/jobs/{id}` | calc | — | `{canceled}` | 202/404 | b/s | jobs.rs:252 |
| GET | `/api/v2/algorithms` | calc | — | `{clustering,routing,bootstrap}` opt lists | 200 | b/s | jobs.rs:304 |
| GET | `/api/v2/geofences` | admin CRUD | query `ReadQuery`(format/rt,depth,level) | GeoJSON FC or raw export | 200/400 | b/s | geofences.rs:143,396 |
| POST | `/api/v2/geofences` | admin CRUD | `CreateGeofence` | record + `Location` | 201/500 | b/s | geofences.rs:168,398 |
| GET | `/api/v2/geofences/{id}` | admin CRUD | query `ReadQuery` (id or name) | GeoJSON Feature or export | 200/400/404 | b/s | geofences.rs:210,409 |
| PATCH | `/api/v2/geofences/{id}` | admin CRUD | `PatchGeofence` | record | 200/404 | b/s | geofences.rs:253,410 |
| DELETE | `/api/v2/geofences/{id}` | admin CRUD | — | empty | 204/404 | b/s | geofences.rs:291,411 |
| POST | `/api/v2/geofences/{id}/publish` | admin/integration | — | `{geofence,event_id,topic,dragonite_area_id}` | 202/404/422 | b/s | geofences.rs:328,400 |
| GET | `/api/v2/geofences/{id}/golbat-data` | data | query `GolbatDataQuery`(category,lastSeen) | golbat points (GenericData) | 200/400/404 | b/s | golbat_data.rs:78; geofences.rs:404-407 |
| GET | `/api/v2/routes` | admin CRUD | query `ReadQuery`(format/rt) | GeoJSON FC or export | 200 | b/s | routes.rs:104,331 |
| POST | `/api/v2/routes` | admin CRUD | `CreateRoute` | record + `Location` | 201/500 | b/s | routes.rs:126,333 |
| GET | `/api/v2/routes/{id}` | admin CRUD | query `ReadQuery` (id or name) | GeoJSON Feature or export | 200/404 | b/s | routes.rs:159,337 |
| PATCH | `/api/v2/routes/{id}` | admin CRUD | `PatchRoute` | record | 200/404 | b/s | routes.rs:193,338 |
| DELETE | `/api/v2/routes/{id}` | admin CRUD | — | empty | 204/404 | b/s | routes.rs:231,339 |
| POST | `/api/v2/routes/{id}/publish` | admin/integration | — | `{route,event_id,topic,dragonite_area_id,mode}` | 202/404/422 | b/s | routes.rs:274,335 |
| GET | `/api/v2/projects` | admin CRUD | query `Pagination`(page,per_page) | paginated rows+meta | 200 | b/s | resources.rs:31→macro lib.rs:818,976 |
| POST | `/api/v2/projects` | admin CRUD | `CreateProject`{name,api_endpoint?,api_key?,golbat,description?} | record+`Location` | 201/500 | b/s | macro lib.rs:858,977 |
| GET | `/api/v2/projects/{id}` | admin CRUD | — (id or name) | record | 200/404 | b/s | macro lib.rs:887,981 |
| PATCH | `/api/v2/projects/{id}` | admin CRUD | `PatchProject` | record | 200/404 | b/s | macro lib.rs:910,982 |
| DELETE | `/api/v2/projects/{id}` | admin CRUD | — | empty | 204/404 | b/s | macro lib.rs:940,983 |
| GET | `/api/v2/properties` | admin CRUD | query `Pagination` | paginated rows+meta | 200 | b/s | resources.rs:43; macro |
| POST | `/api/v2/properties` | admin CRUD | `CreateProperty`{name,category:`koji_db::Category`,default_value?} | record+`Location` | 201/500 | b/s | macro |
| GET | `/api/v2/properties/{id}` | admin CRUD | — | record | 200/404 | b/s | macro |
| PATCH | `/api/v2/properties/{id}` | admin CRUD | `PatchProperty` | record | 200/404 | b/s | macro |
| DELETE | `/api/v2/properties/{id}` | admin CRUD | — | empty | 204/404 | b/s | macro |
| GET | `/api/v2/tile-servers` | admin CRUD | query `Pagination` | paginated rows+meta | 200 | b/s | resources.rs:53; macro |
| POST | `/api/v2/tile-servers` | admin CRUD | `CreateTileServer`{name,url} | record+`Location` | 201/500 | b/s | macro |
| GET | `/api/v2/tile-servers/{id}` | admin CRUD | — | record | 200/404 | b/s | macro |
| PATCH | `/api/v2/tile-servers/{id}` | admin CRUD | `PatchTileServer` | record | 200/404 | b/s | macro |
| DELETE | `/api/v2/tile-servers/{id}` | admin CRUD | — | empty | 204/404 | b/s | macro |
| POST | `/api/v2/geometry/convert` | calc/convert | `ConvertReq` + query `?format=` | GeoJSON or export | 200/500 | b/s | geometry.rs:57,192 |
| POST | `/api/v2/geometry/simplify` | calc/convert | `SimplifyReq` + `?format=` | GeoJSON or export | 200/500 | b/s | geometry.rs:92,193 |
| POST | `/api/v2/geometry/merge-points` | calc/convert | `MergePointsReq` + `?format=` | GeoJSON or export | 200/500 | b/s | geometry.rs:121,194 |
| POST | `/api/v2/geometry/area` | calc/convert | `SimplifyReq` | `{area:f64 m²}` | 200 | b/s | geometry.rs:181,195 |
| POST | `/api/v2/s2/circle-coverage` | calc/s2 | `CoverageArgs`{lat,lon,radius?,size?,level} | cell-id result | 200 | b/s | s2.rs:70,166 |
| POST | `/api/v2/s2/cell-coverage` | calc/s2 | `CoverageArgs` | `Vec<String>` cell ids | 200 | b/s | s2.rs:91,167 |
| POST | `/api/v2/s2/polygons` | calc/s2 | `Vec<String>` cell ids | polygons | 200 | b/s | s2.rs:116,168 |
| POST | `/api/v2/s2/{cell_level}` | calc/s2 | `koji_core::BoundsArg` (doc twin `S2CellsBody`) | cells in bbox | 200 | b/s | s2.rs:132,169 |
| POST | `/api/v2/golbat-data/{category}` | data | `AreaReq`(area\|bbox,lastSeen,tth) | `{points:[[lat,lon]]}` | 200/400 | b/s | golbat_data.rs:271,366 |
| POST | `/api/v2/golbat-data/{category}/stats` | data | `AreaReq` | `{total:usize}` | 200/400 | b/s | golbat_data.rs:306,367 |
| GET | `/api/v2/config` | misc | — | `ConfigResponse` | 200 | b/s | config.rs:38; lib.rs:374 |
| GET | `/api/v2/nominatim` | import | query `?query=` | FC (Polygon/MultiPolygon only) | 200/500 | b/s | nominatim.rs:39; lib.rs:377 |
| GET | `/api/v2/plugins` | admin/config | — | merged plugin views[] | 200 | b/s | plugins.rs:95,234 |
| GET | `/api/v2/plugins/{kind}/{name}` | admin/config | — | plugin view | 200/404 | b/s | plugins.rs:120,237 |
| PATCH | `/api/v2/plugins/{kind}/{name}` | admin/config | `PluginPatch`{enabled?,args_default?,description?} | plugin view | 200/404/422 | b/s | plugins.rs:156,238 |
| DELETE | `/api/v2/plugins/{kind}/{name}` | admin/config | — | `{rows_affected}` | 200/404 | b/s | plugins.rs:210,239 |
| POST | `/api/v2/auth/login` | misc/login | `Auth`{password} | `{authenticated:true}` (sets session) | 200/401 | b/s (open) | auth.rs:63,127 |
| POST | `/api/v2/auth/logout` | misc/login | — | empty (clears session) | 204 | b/s | auth.rs:90,128 |
| GET | `/api/v2/auth/me` | misc/login | — | `{authenticated,via:session\|bearer\|open\|none}` | 200 | b/s | auth.rs:103,129 |

**Total: 47 routed endpoints** (44 under `/api/v2` + healthz/readyz/openapi).

## Key shared types
- **`CalcJobRequest`** (`requests/ops.rs:122`): `#[serde(flatten)] request: CalcRequest` + `category` (default `"pokestop"`). **`CalcRequest`** (`ops.rs:42-49`) is internally-tagged `#[serde(tag="mode", rename_all="camelCase")]` → mode strings `cluster`/`route`/`reroute`/`bootstrap`/`routeStats`; variants `Cluster(ClusterReq)`, `Route(ClusterReq)`, `Reroute(RerouteReq)`, `Bootstrap(BootstrapReq)`, `RouteStats(StatsReq)`. Arg-groups (`ClusteringArgs`/`RoutingArgs`/`BootstrapArgs`/`DataFilterArgs`/`OutputArgs`/`DevArgs`) are camelCase `#[serde(default)]` wire DTOs that `.resolve()` to koji-core configs (`requests/groups.rs`, `requests/resolve.rs`).
- **`CalcPayload`** (`calc.rs:57`): the queue-crossing payload (mode,category,request,area,data_points,clusters); job kind `CALC_KIND="calculate"`, HIGH priority 100, content-dedup via `dedup_key`.
- **Envelope** (`utils/api_response.rs`): `ApiResponse::Ok{data,meta}` / error; `Meta{total,page,per_page,total_pages,has_next,has_prev}`; `ApiError`. **`Pagination`** (`utils/pagination.rs`): `{page?,per_page?}`, 1-based wire, `per_page` clamped [1,500].
- **`ConfigResponse`** (`utils/response.rs:10`): `{start_lat,start_lon,tile_server,logged_in,dangerous,route_plugins[],clustering_plugins[],bootstrap_plugins[]}`.
- **`?format=` / `?rt=`** return-type negotiation on geometry reads → `respond_geo` (`utils/format.rs`); `ReturnTypeArg` covers FeatureCollection/Feature/SingleArray/Sql/Poracle/etc.

## INTERNAL ENDPOINTS WE COULD ADD (V2, purpose-built for a shadmin client)

Gaps grounded in `dataProvider.ts` TODO (`:18-28`) — current friction is real and documented in-repo:

1. **Row-shaped list endpoints for geofences/routes** — biggest gap. The geometry list handlers (`geofences.rs:143`, `routes.rs:104`) return a whole GeoJSON FeatureCollection, **ignore `?page/?sortBy/?q`**, and force the SPA to project features→rows client-side (`featureToRecord` `:74`, lossy per the TODO) and count `total` client-side. Add `GET /api/v2/geofences?view=rows` (or a sibling `/geofences/list`) returning paginated flat records `{id,name,mode,parent,area_m2,point_count,geofence_id,projects[],properties[],has_geometry}` + a real `meta` block, honoring `page/per_page/sortBy/order/q`. Same for routes. Eliminates the lossy 12→4-mode/link-field client projection entirely.

2. **`getMany` batch-by-ids** — `dataProvider.getMany` (`:153`) does N parallel single-GETs because no batch endpoint exists. Add `GET /api/v2/{resource}?ids=1,2,3` (or `POST /api/v2/{resource}/batch {ids:[]}`) returning the matched rows. ra calls this constantly for reference fields (route→geofence_id, geofence→projects/properties).

3. **Server-side sort/filter on the macro CRUD** — `koji_resource!` `list` (`macros lib.rs:818-845`) hardcodes `sort_by:"id"`, `order:"ASC"`, `q:""` and discards the `sortBy/order/q` the SPA sends. Thread `Pagination` → `AdminReqParsed` properly (the koji-db `paginate`/`AdminReqParsed` already supports `sort_by`/`order`/`q`/`geotype`/`project`/`mode`/`parent`/`geofenceid`/`pointsmin`/`pointsmax`). Pure wiring; no new DB surface.

4. **Aggregate dashboard stats** — `GET /api/v2/stats` (or `/api/v2/dashboard`) one-shot: counts of geofences/routes/projects/properties/tile-servers, job-queue rollup (pending/running/completed/failed by status), plugin enabled-counts by kind, golbat data totals per category. Today a dashboard must fan out to `list_jobs` + every CRUD list + `/algorithms`. Cheap COUNT(*) aggregate; saves ~6 round-trips per dashboard paint.

5. **Job queue summary / live feed** — `GET /api/v2/jobs/summary` returning `{by_status:{pending,running,...}, oldest_pending_age, worker_count}` for an admin "queue health" panel without paging the full job list. (Optionally SSE `GET /api/v2/jobs/stream` for live job-status push instead of the `?wait=` long-poll.)

6. **Reference choices for forms** — `GET /api/v2/geofences/choices` and `/routes/choices` returning slim `{id,name}[]` (no geometry) for `<ReferenceInput>`/select dropdowns. Currently a select must pull the entire FeatureCollection to populate options.

7. **Bulk delete** — `DELETE /api/v2/{resource}?ids=…` for ra's `BulkDeleteButton` (currently N single DELETEs).

8. **OpenAPI content-type fix (DX, not new endpoint)** — `GET /api/v2/openapi.yaml` serves JSON under a `.yaml` URL (`lib.rs:161-180`, utoipa 5.x has no `to_yaml`). A shadmin/codegen client wiring against the spec should hit it expecting JSON, or add a real `/api/v2/openapi.json` alias.

Note for the refactor trace: all 8 are additive and align with the project's own "we can add internal endpoints for V2" mandate; #1–#3 are the load-bearing ones (they delete the lossy client-side GeoJSON→row projection and the discarded sort/filter), #4–#7 are convenience surfaces that match ra/shadmin data-provider call shapes.