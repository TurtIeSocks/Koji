# Koji V2 — Refactor Map (old → new)

Primary deliverable. New workspace at **repo root**. Decisions: persistent DB job queue · v2 API + v1 shim · layered ~9 crates · durable outbox+webhooks (Dragonite=first subscriber) · formalized subprocess plugins · drop RDM · JSend envelope (matches Dragonite v2).

## New structure

```
Koji/
  Cargo.toml                         # [workspace] members crates/* bins/*
  crates/
    koji-core/                       # PURE domain — no db/http/tokio
      geometry         ← was model/api/{single_vec,multi_vec,point_*,collection,feature,geometry,bbox}
                         Action: move (1:1). The ~18 To* conversion traits + GeoFormats land here unchanged.
      config           ← was model/api/args.rs  [RW]
                         Action: rewrite. Args(37 fields) → ClusteringConfig/RoutingConfig/BootstrapConfig/
                         S2Config/OutputConfig/AreaInput/DataFilter/DevConfig. Drop 6 deprecated fields.
      enums            ← was model/db/sea_orm_active_enums (domain copy)
                         Action: define domain FenceType/DataType/Category/ReturnType/GeometryType + strategy
                         enums (ClusterMode/CalculationMode/SortBy/SpawnpointTth). db maps sea-orm↔these.
      error            ← was model/error.rs
                         Action: refactor to thiserror; no sea-orm leakage.
    koji-macros/        ← was macros/            Action: move (1:1, rename).
    koji-nominatim/     ← was nominatim/         Action: move; depend on koji-core for geometry.
    koji-plugins/       ← was algorithms/plugin.rs  [RW]
                         Action: rewrite. AlgorithmPlugin trait + SubprocessPlugin + PluginRegistry.
                         plugin.toml manifest, versioned JSON stdin/stdout protocol, KOJI_PLUGINS_DIR (out of source).
    koji-algorithms/    ← was algorithms/ (minus plugin.rs)
                         Action: keep logic, restructure inputs. cluster()/route()/bootstrap() take config structs
                         (was 15 primitives). stats.rs kept. rayon retained.
    koji-golbat/       ← was model/db/{gym,pokestop,spawnpoint,station}.rs
                         Action: move + isolate. Read-only golbat (GOLBAT_DB_URL). Returns koji-core points.
    koji-db/            ← was model/db/{geofence,route,project,property,geofence_project,geofence_property,tile_server}
                         Action: refactor. Koji's OWN db. sea-orm entities stop being public DTOs; From/Into to
                         core domain enums at the boundary. + NEW area_fence + dragonite_area_id linkage.
    koji-dragonite/     ← NEW (replaces model/db controller writes)
                         Action: write fresh. Typed client over Dragonite /v2/areas/* (PATCH by id, tri-state
                         geofence, JSend types). base_url+bearer+ua. SingleVec→route array, Feature→ApiGeofence.
    koji-jobs/          ← NEW
                         Action: write fresh. `job` entity + queue API + worker pool + JobHandler trait +
                         ProgressHandle/CancelToken. Claim via FOR UPDATE SKIP LOCKED + lease/heartbeat.
    koji-events/        ← NEW (generalizes the old inline send_api_req)
                         Action: write fresh. event_outbox + webhook_subscription + dispatcher worker (backoff,
                         dead-letter) + Subscriber trait + generic WebhookSubscriber (HMAC).
    koji-migration/     ← was migration/
                         Action: keep history; ADD job, event_outbox, webhook_subscription, area_fence,
                         area.dragonite_area_id.
    koji-service/       ← was api/  [RW]
                         Action: rewrite. actix app. v2 typed resources + /jobs + /calc sugar + golbat-data +
                         geo utils + meta. v1 shim module. Managed AppState (DB pool, JobQueue, Dispatcher,
                         registries, dragonite client). Concrete CalculateHandler (jobs) + DragoniteSubscriber (events).
                         Explicit response DTOs (entity→DTO mapping). JSend envelope.
  bins/
    koji-server/        ← was server/src/main.rs  Action: thin main → koji-service::run().
    koji-cli/           ← NEW  Action: run (direct) | enqueue [--wait] | worker (drains shared queue).
  client/  or-tools/  docs/   # untouched phase 1
```

## Bulk 1:1 moves (mechanical, Phase 0)
| Old | New | Note |
|---|---|---|
| `server/macros` | `crates/koji-macros` | rename only |
| `server/nominatim` | `crates/koji-nominatim` | + dep on koji-core |
| `server/algorithms/{clustering,routing,bootstrap,s2,rtree,stats,utils}` | `crates/koji-algorithms` | move; signatures change in Phase 2 |
| `server/migration` | `crates/koji-migration` | move; new migrations added later |
| `server/src/main.rs` | `bins/koji-server/src/main.rs` | move |

## Dropped (not in V2)
- `model/db/instance.rs` — RDM `instance` writes. **Replaced by koji-dragonite API calls.**
- `model/db/area.rs` controller-write logic (`upsert_from_geometry` to controller) — the "rude" path. **Replaced by DragoniteSubscriber.**
- `GolbatType` (+ `Unown`/`RDM`/`Hybrid`) & golbat-type autodetect — **single path now**; no RDM, no Hybrid.
- `RdmInstance`, `RdmInstanceArea`, `InstanceParsing::Rdm`, `parse_golbat_instance` RDM arms.
- `KojiDb.controller` connection — replaced by the Dragonite HTTP client. (KojiDb: 3 conns → 2: `koji`+`golbat`.)
- Deprecated env: `UNOWN_DB_URL`, `UNOWN_DB`, `DATABASE_URL`. Keep `GOLBAT_DB_URL` + Koji DB + `DRAGONITE_URL`/`DRAGONITE_TOKEN`.
- `Args` deprecated fields: `devices`, `fast`, `generations`, `only_unique`, `route_chunk_size`, `routing_time`.
- GET-that-mutates `push/{id}` routes — become `POST /:id/publish` (+ event).

## Client touch-points (DEFERRED — note only, phase 1 doesn't touch client)
The client today calls `/internal/admin/*`, `/internal/routes/*`, `/api/v1/{geofence,route,s2,convert}` + the legacy `{message,status,status_code,data,stats}` envelope. The v1 shim keeps ALL of these working unchanged in phase 1. Future client-migration work (separate effort) will need to:
- Switch base path `/api/v1` → `/api/v2`; adopt JSend envelope (`status`/`data`/`meta`/`error`).
- Replace `GET …/push/{id}` with `POST …/:id/publish`.
- Replace path-segment `…/{return_type}` with `?format=`.
- Move calc from sync POST to `POST /jobs` (or `/calc/*`) + poll `GET /jobs/:id`.
- New UI surfaces: job status/history, webhook-subscription CRUD, `dragonite_area_id` linkage editor, per-mode (`area_fence`) fence assignment.
