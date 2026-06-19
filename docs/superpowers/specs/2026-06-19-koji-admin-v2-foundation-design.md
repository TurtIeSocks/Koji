# Koji Admin V2 — Foundation Design

_Date: 2026-06-19 · Status: approved (brainstorming) · Next: writing-plans_

First sub-project of the Koji client V2 effort (MUI → shadmin/shadcn port). Audit + trace in
`refactor-workspace/` (`trace.md`, `goals.md`, `assessment.md`, `map.md`). This spec covers the
**foundation + one vertical slice**; later sub-projects each get their own spec → plan → implement.

## Goal

Stand up a new shadcn/shadmin-powered admin client (`apps/web`) and prove the entire stack end to
end on one rich resource (**geofence**), with a clean data layer and **realtime wired in from day
one**. Old `apps/web-client` (MUI map + admin) stays live throughout (strangler-fig); production
cutover is a later, deliberate step.

## Locked decisions (from brainstorming)

- **V2 client home:** new `apps/web` in the Koji repo (standalone, like `apps/web-client`).
- **Map framework:** **MapLibre GL** + (later) **Terra Draw** as the editing layer — *not* shadmin's
  Leaflet+geoman suite (so shadmin's Leaflet shape inputs are reference-only). One map stack across
  admin + future `/map`.
- **Realtime:** full — frontend decorator **and** a koji-server WS hub with macro-emitted resource
  events + job-progress events, plus a minimal live dashboard.
- **API architecture:** public `/api/v2` stays clean + versioned (no client-specific shapes — the v1
  mistake). The client uses a **private, unversioned `/internal/*` surface exclusively** —
  **forwards** to the public handler via route alias / shared handler where identical, **bespoke**
  private handlers only where genuinely client-specific (row lists, WS, future dashboards).
- **Engagement:** participate (section-by-section sign-off — done).

## Section 1 — App scaffold & stack

- New app `apps/web/` (Koji repo; Koji has no JS workspace, so it's its own package — matches
  `apps/web-client`).
- **Stack:** Vite 8 + React 19 + TypeScript 6 + Tailwind v4 + shadcn `new-york` (base `neutral`,
  lucide icons).
- **Package manager: bun** (`bun.lock` text lockfile). shadmin stays pnpm (it's a library).
- **shadmin consumption:** `shadcn add` from shadmin's registry → components copied into
  `apps/web/src/` under the `@/` alias and **committed to Koji's git** (the shadcn model — copied
  source is owned + inherently pinned). Registry source = published
  `https://shadmin.turtlesocks.dev/r/`, with a local `pnpm --filter shadmin registry:build` of the
  sibling repo (`/Users/rin/GitHub/shadcn-admin-kit`) as fallback when the published one lags.
- **Dev/serving:** Vite on **port 5273** (+100 offset convention) with proxies `/internal` + `/api`
  → `http://0.0.0.0:8080` (the client hits `/internal`; `/api` proxied for WS + any shared assets).
  koji-server keeps serving the old
  `web-client/dist` this phase; `apps/web` is dev/standalone.
- **MapLibre deps now:** `maplibre-gl` + `react-map-gl` (`/maplibre` subpath). `terra-draw` deferred
  to the editor spec.
- **Skipped:** root JS monorepo/turborepo (YAGNI for two independent apps), Storybook, i18n setup.

## Section 2 — Admin shell, auth & layout

- One `<Admin>` (shadmin / ra-core 5.14) wiring `dataProvider` / `authProvider` / `layout` /
  `disableTelemetry`; default `localStorageStore` + English `i18nProvider`. `dashboard` slot used by
  the minimal live dashboard (Section 4). Only **geofence** `<Resource group="Geo">` registered this
  spec (no guesser stubs).
- **authProvider (`src/auth-provider.ts`, ~40 lines)** — formalizes today's informal config-gate:
  - `login({password})` → `POST /api/v2/auth/login`; `logout()` → `POST /api/v2/auth/logout`;
    `checkAuth()` → `GET /api/v2/auth/me` (`{authenticated}`); `checkError(401/403)` → reject→login;
    `getPermissions`/`canAccess` → allow-all (single-secret app, no RBAC).
  - **Login page:** a **password-only** variant of shadmin `<LoginForm>` (single field — Koji auth
    has no username). Small custom component.
- **Layout:** shadmin `<Layout>` + `<AppBar>` (title "Kōji Admin", `<ThemeModeToggle>`, a link to the
  live old `/map` during transition) + `<AppSidebar>` (auto resource menu). Admin at app root `/`;
  routing = react-router v7 via ra-core.
- **Theme:** shadcn `neutral` base + light/dark toggle (persisted via ra-core `useStore`). Fancier
  shadmin palettes + Kōji branding polish are a later pass.
- **Skipped:** RBAC, multi-locale, palette theming.

## Section 3 — Data layer (private `/internal` API + dataProvider)

The client uses a **private, unversioned `/internal/*` surface exclusively** — never `/api/v2`
directly. Public `/api/v2` gets **no client-specific shapes**.

**Backend (koji-service, Rust):**
- **Mount a private `/internal` scope** with the same session/bearer auth gate as `/api/v2`
  (`public_validator`).
- **Forwards (route alias / shared handler):** for endpoints identical to public, register the
  **same handler** under `/internal/X` as `/api/v2/X` (or a 1-line wrapper) — no HTTP redirect, no
  round-trip, zero duplicated logic. Covers: geofence/route `getOne`+create+update+delete,
  project/property/tile-server/plugins CRUD, config, auth/*, nominatim, geometry/*, s2/*, jobs/*,
  golbat-data/*.
- **Bespoke private endpoints (own handler, own shape — NOT in public/OpenAPI):**
  - **`GET /internal/geofences`** — row list: `{data: Row[], meta:{total,page,per_page,total_pages,…}}`
    honoring `page/per_page/sortBy/order/q` + filters. Row ≈
    `{id,name,mode,parent,geo_type,projects[],property_count}`. **Reuses the same DB query layer**
    (`AdminReqParsed`/`paginate`) the public geofence handler uses — just serialized as rows instead
    of GeoJSON. (Route's row list rides the same pattern but lands **with the route UI**.)
  - **`GET /internal/realtime`** — the WS hub (Section 4); private infra.
- **One public-API change (general correctness, not a client shape):** the macro CRUD list
  (`koji_resource!`) currently hardcodes `sort_by:"id"`, `order:"ASC"`, `q:""` — thread
  `Pagination → AdminReqParsed` so it honors `sortBy/order/q`. The client reaches it via the
  `/internal` forward. Public's GeoJSON FC reads (geofence/route `?format=feature`) stay untouched —
  the map backport needs them.

**Frontend dataProvider (`src/data-provider.ts`):**
- Thin ra-data adapter, **base URL `/internal`**, envelope-unwrapped.
- `getList`/`getManyReference` for **geofence** → `GET /internal/geofences` (row list); for
  project/property/tile-server/plugins → the forwarded macro CRUD (now server-sorted/filtered) →
  real `{data, total}`. **No lossy `featureToRecord` client projection.**
- `getOne(geofence)` → forwarded public Feature read; map the single Feature → record
  (lossless — only the *list* collection was the problem).
- **Writes** (`create`/`update`/`delete`) → forwarded public CRUD unchanged (form posts GeoJSON).
- `getMany` stays **N-parallel** for now. `getOne` normal.
- **Deferred (later specs):** bespoke `/internal/stats`, `/internal/.../choices`, batch `?ids=`,
  bulk-delete `?ids=`.

## Section 4 — Realtime (frontend + koji-server WS)

**Frontend:**
- `dataProvider = addEventsForMutations(realtimeDataProvider(base, wsTransport, { lockProvider }), …)`
  where `wsTransport = webSocketTransport({ url: '/internal/realtime' })` — shadmin's production WS
  client (reconnect/heartbeat/auth/pending-publish queue).
- Resources use `<ListLive>` / `<EditLive>` / `<ShowLive>`; reference counts via live `<Count>`.
  Topic names match shadmin's `resourceTopic`/`recordTopic` helpers exactly.

**Backend (koji-service, Rust):**
- **`GET /internal/realtime`** WS upgrade via **`actix-ws`** (modern non-actor API). Session-cookie auth
  on the handshake (reuse `public_validator`); reject unauthenticated upgrades.
- **`RealtimeHub`** in app state — in-process pub/sub. *ponytail: single `tokio::broadcast` of
  `(topic, event)` + per-connection topic filter; shard to a `DashMap<topic, …>` only if connection
  count ever justifies it.* Per connection: read client frames (`subscribe`/`unsubscribe`/`publish`/
  `ping`), hold a subscribed-topic set, forward matching hub events, pong.
- **Event sources:**
  - **Resource mutations** — emitted inside the **`koji_resource!` macro** (create/update/delete) +
    the geofence/route handlers → `publish("resource/{name}", {type:'created'|'updated'|'deleted',
    id})`. Drift-killed at the macro, so every resource is live by construction.
  - **Job lifecycle** — where the worker writes `JobRecord` status/progress/phase →
    `publish("jobs", …)` + `publish("jobs/{id}", …)`. Long-poll `?wait=` stays working (additive).
- **Record locks** — `lockProvider` wired to hub `lock/{resource}/{id}` topics. *Included but minimal
  (single-user value is low; plumbing is free given the hub).*
- **Skipped:** SSE transport (WS covers it), server-persisted locks beyond in-hub.

**Minimal realtime dashboard (this spec):** landing dashboard with **live entity-count cards** (per
resource, via live `<Count>`) + a **live job-queue panel** (subscribes the `jobs` topic — queue
depth, running/recent jobs). No `/stats` endpoint needed (counts from list totals; jobs from the
live feed). Richer recharts analytics dashboard deferred.

## Section 5 — Geofence vertical slice

`src/resources/geofence/{index.ts, geofence-list, geofence-edit, geofence-create, geofence-show}.tsx`

- **`index.ts`** → `ResourceProps { name:'geofence', list, edit, create, show,
  recordRepresentation:'name', icon }` (lucide); `<Resource {...geofence} group="Geo" />`.
- **List** — `<ListLive>` + shadmin `<DataTable>` cols `name` / `parent` (ReferenceField→geofence) /
  `mode` / `geo_type`. Sidebar `<FilterLiveSearch>` (q) + `<FilterList>` for project / parent /
  geotype / mode. Server pagination+sort+filter via `GET /internal/geofences`. Bulk: **delete only**.
- **Edit / Create** — `<EditLive>` / `<Create>` + `<SimpleForm>`: `name` (required), `mode`
  (SelectInput), `parent` (ReferenceInput→geofence + Autocomplete), `geometry` (**Monaco JSON input +
  read-only MapLibre preview**, Section 6). Create = single form only.
- **Show** — `<ShowLive>` + layout: name / mode / geo_type / parent ref / **MapLibre geometry
  preview** + raw geometry (Monaco read-only).
- **Mode set** — reconcile v1 12-mode `KojiModes` vs v2's collapsed set into one canonical list in
  `lib/constants.ts`; SelectInput choices derive from it.
- **Deferred (geofence's heavy bits, follow-up specs):** properties array-input (category-driven
  dynamic value), projects array-ref, import wizard (shapefile/Nominatim/golbat), publish + assign
  bulk actions.

## Section 6 — MapLibre geometry preview

`src/components/geometry/geometry-preview.tsx` — read-only, reusable in geofence edit + show; the
seed of the future Terra Draw editor.

- **`react-map-gl/maplibre` `<Map>`** + `<Source type="geojson">` + `<Layer>` (fill+outline for
  polygons, circle for points). Read-only (pan/zoom only).
- **Reads the RHF geometry field** (`useWatch`) in edit / the record in show. Monaco JSON input is
  source of truth; preview re-renders on change. Invalid JSON → holds last-valid / empty, no crash.
- **Fit bounds** to geometry on change (turf `bbox`).
- **Basemap = raster style from Koji's tile-servers.** `tileServerToMapLibreStyle(url)` builds a
  MapLibre raster style from an XYZ tile URL — defaults to CartoDB Voyager (matching today's map),
  upgradeable to `/api/v2/tile-servers` later. Deliberately reuses Koji's tile-server concept.
- **Skipped:** vector/glyph styles, Terra Draw interaction, layer toggles, the 10k-point GL layer.

## Section 7 — Testing & verification

- **Backend (Rust):** integration tests for `GET /internal/geofences` (pagination/sort/filter/q) +
  the `/internal` forward-aliases (auth applies, shapes match public), macro-CRUD
  sort/filter wiring, and the WS hub (subscribe→delivery, resource-mutation publish, job-event
  publish — a test WS client asserting frames). Needs the test DB (`KOJI_DB_URL`; reconstruct
  `.env.test`; copy into any worktree).
- **Frontend:** dataProvider unit tests vs a mock backend (row-mode mapping, envelope unwrap);
  component tests (vitest-browser-react / Playwright provider) — geofence list/edit, MapLibre preview
  renders a geometry, a realtime check (mutation → `<ListLive>` updates). Browser suite runs at **end
  of each TDD task**, not per-step.
- **TDD throughout.** Manual verify via **Claude Preview on :5273** (inline, no Chromium window).

## Section 8 — Out of scope → future-spec roadmap

This spec = foundation only. Deferred, each its own spec → plan → implement:

1. **Route resource** + geofence's heavy bits (properties array, projects ref, import wizard,
   publish/assign) + route row-shape.
2. **project / property / tileserver / plugins** resources (lighter).
3. **MapLibre + Terra Draw interactive editor** — adopted by all geometry forms at once.
4. **Rich dashboard** (recharts analytics + `/stats`) + P1/P2 endpoints (batch `?ids=`, `/choices`,
   bulk-delete).
5. **Full `/map` backport** — drawer, popups, clustering/bootstrap calc UI, S2 layers, pixi→native-GL
   points, import wizard.
6. **Production cutover** — koji-server serves `apps/web/dist`, retire `web-client`.

## Risks / call-outs

- **Bigger foundation than a CRUD slice** — the realtime pillar (WS transport + server broker + macro
  event emission) is a third unknown alongside shadcn theming and the data layer. Accepted explicitly.
- **shadmin churn** — realtime, native-CSS theming, granular registry are `[Unreleased]`/0.1.0. The
  copied-in source is committed to Koji (pins it); track shadmin's contract as ra-core 5.14.
- **Terra Draw #197** (React re-render layer-loss) is *not* in this spec (preview is read-only) but is
  the thing to prototype first in the editor spec; Koji is structurally positioned to dodge it
  (markers live in imperative layers, not React state).
- **actix-ws auth** — validate the session cookie on the `/internal/realtime` WS handshake;
  same-origin only (no CORS today).
- **Private `/internal` scope** — must sit behind the same auth gate as `/api/v2`; forwards via
  shared handler mean a handler is mounted under two scopes (verify the auth middleware applies to
  both). Keep `/internal` out of the OpenAPI doc — it's intentionally uncontracted.
