# Koji Frontend V2 — Old→New Map (Phase 4)

_Primary deliverable. Living doc — brainstorming may re-map. `←` = old source. Action ∈
port | port+redesign | rewrite | replace-with-lib | split | drop. Paths relative to repo roots:
`web-client/` = `apps/web-client/src/`, `shadmin/` = `shadcn-admin-kit/packages/shadmin/src/`._

> **Gating decisions** (see goals.md open decisions): V2 client home, map-framework direction,
> engagement mode. Admin-port mapping below holds regardless; map backport waits on the framework call.

## New structure (proposed — V2 client in Koji repo)

```
apps/web/                         ← new app; ponytail: do NOT fork apps/web-client, build fresh
  package.json                      Vite + React 19 + Tailwind v4 + shadcn new-york
  components.json                   shadcn registry config (style new-york, base neutral, lucide)
  src/
    main.tsx                        ← was web-client/index.tsx — createRoot + StrictMode
    App.tsx                         ← was web-client/App.tsx
      Action: rewrite
      Notes: drop createBrowserRouter route table; render one <Admin>. react-router v7 comes
             via ra-core. Config-gate (GET /api/v2/config logged_in→/login) becomes the authProvider.

    admin.tsx                       ← was web-client/pages/admin/index.tsx
      Action: rewrite
      Notes: <Admin dataProvider authProvider i18nProvider layout dashboard disableTelemetry>
             with <Resource {...project} group="Data"/> … shadmin Layout/AppBar/AppSidebar.

    dataProvider.ts                 ← was web-client/pages/admin/dataProvider.ts
      Action: rewrite (replace-with-lib base + custom)
      Notes: ra-data adapter over /api/v2. Lean on NEW row endpoints (see Backend below) so the
             lossy featureToRecord() projection and client-side total/sort/filter all DELETE.
             Keep envelope unwrap + plugins composite-id (kind:name) routing.

    authProvider.ts                 ← NEW (was App.tsx config gate, informal)
      Action: write fresh (~40 lines)
      Notes: login→POST /auth/login, logout→POST /auth/logout, checkAuth→GET /auth/me,
             checkError→401→redirect. Session-cookie based; no token store.

    dashboard/                      ← NEW (no old equivalent)
      Action: write fresh
      Notes: recharts via shadmin extras/dashboard-charts (MetricCard/TrendChart/Donut). Needs
             GET /api/v2/stats aggregate endpoint (Backend #4) to avoid 6+ list round-trips.

    resources/
      project/{index.ts,list,edit,create,show}.tsx   ← was web-client/pages/admin/project/*
        Action: port+redesign
        Notes: ResourceProps {name,list,edit,create,show,recordRepresentation:'name',icon}.
               DataTable cols name/description/golbat(Bool)/geofences count. Edit:
               ReferenceArrayInput projects→geofence (AutocompleteArrayInput). Export action ports
               (FIX the wrong project→geofences/{id} export endpoint). Bulk assign-geofences dialog
               → shadcn Dialog + useGetMany('geofence').
      geofence/{…}.tsx                                ← was web-client/pages/admin/geofence/*
        Action: port+redesign
        Notes: List + FilterList aside (project/parent/geotype/mode). Form: name/mode(Select)/
               parent(ReferenceInput)/properties(ArrayInput+SimpleFormIterator, category-driven
               value input)/geometry. GEOMETRY: replace CodeInput JSON →
               shadmin/components/leaflet PolygonInput (interactive map edit). Edit adds
               ReferenceArrayInput projects. Create renders ImportWizard (port) + single form.
               Bulk: assign-parent, assign-projects, publish, export, delete.
      route/{…}.tsx                                   ← was web-client/pages/admin/route/*
        Action: port+redesign
        Notes: geometry CodeInput(MultiPoint) → shadmin MultiPointInput. points FunctionField,
               geofence_id ReferenceInput. Bulk publish/delete/export.
      property/{…}.tsx                                ← was web-client/pages/admin/property/*
        Action: port+redesign
        Notes: category SelectInput drives dynamic default_value input (bool/string/number/color/
               object/array/database) ← was inputs/Properties.tsx. Use shadcn field types.
      tileserver/{…}.tsx                              ← was web-client/pages/admin/tileserver/*
        Action: port+redesign
        Notes: name/url + live map preview (shadmin BaseMap TileLayer).
      plugins/{index.ts,list,edit}.tsx               ← was web-client/pages/admin/plugins/*
        Action: port+redesign
        Notes: composite id kind:name. List+Edit only (no create/show). args_default JSON →
               shadmin Monaco JSON input. entrypoint/interpreter/protocol disabled.

    actions/                        ← was web-client/pages/admin/actions/*
      Action: port+redesign
      Notes: PushToProd/BulkPushToProd (publish), Export/BulkExport, AssignProjectFence,
             AssignParentFence, Extras menu. ra-core mutation hooks unchanged; MUI buttons/menus
             → shadcn Button/DropdownMenu. useRaStore dialog flags → ra-core useStore.

    lib/
      geo-utils.ts                  ← was web-client/services/utils.ts
        Action: port
        Notes: pure turf/color/mode helpers. Fix 12→4 KojiModes collapse here.
      koji-client.ts                ← split from web-client/services/fetches.ts
        Action: split+port
        Notes: keep calc/job/geometry/s2/golbat-data typed calls (convert/area/merge-points/jobs/
               pollJob/getMarkers/getS2Cells) for the map backport + any admin calc surfaces.
               CRUD calls move into dataProvider.
      types.ts                      ← was web-client/assets/types.ts
        Action: port
        Notes: GeoJSON-branded Feature/FeatureCollection, Koji entities, JobRecord/ApiEnvelope,
               KojiModes (reconcile to v2 4-mode set). Drop v1 KojiResponse.
      constants.ts                  ← was web-client/assets/constants.ts
        Action: port (trim)
        Notes: keep MODES/CATEGORIES/PROPERTY_CATEGORIES/CONVERSION_TYPES/S2 levels; drop MUI
               icon-SVG maps that move to lucide.
```

## Reused from shadmin (copy-in via shadcn registry, not rewritten)

| New use | shadmin source | Notes |
|---|---|---|
| Admin shell, Resource, CRUD views, DataTable, all inputs/fields | `shadmin/components/admin/*` | ra-core 5.14 contract; stock react-admin docs apply |
| Layout / AppBar / Sidebar / Menu / ThemeModeToggle / Breadcrumb | `shadmin/components/admin/layout/*` | replaces MUI Layout/AppBar |
| Notifications (sonner) / Confirm / Loading / Error / NotFound | `shadmin/components/admin/feedback/*` | replaces `notifications/*` |
| **Geometry shape Inputs/Fields** (Polygon, MultiPoint, all types, GeoJSON, FeatureCollection) | `shadmin/components/leaflet/shapes/*`, `geoman/use-geoman-rhf.ts` | **replaces CodeInput JSON** — interactive editing in admin forms |
| Nominatim geocoding input | `shadmin/components/leaflet/geocoding/*` | reconcile w/ Koji's vendored Nominatim backend |
| OSM/Overpass set-ops, simplify, turf wrappers | `shadmin/components/leaflet/osm/*`, `simplify-input.tsx` | geofence derivation helpers |
| Dashboard charts | `shadmin/components/extras/dashboard-charts.tsx` | recharts wrappers |
| Monaco JSON input/field | `shadmin/components/monaco/*` | replaces Code.tsx codemirror |
| Color/Cron/Duration inputs+fields | `shadmin/components/extras/*` | property/plugin editors |
| Realtime WS transport (later) | `shadmin/components/realtime/*` | golbat live scan updates |

## Backend — NEW internal endpoints (additive; align with "we can add endpoints" mandate)

| Endpoint | Why | Priority |
|---|---|---|
| `GET /api/v2/geofences?view=rows` + `/routes?view=rows` (paginated flat records + real meta, honor page/per_page/sortBy/order/q) | kills lossy GeoJSON→row client projection + ignored sort/filter | **P0** |
| Server-side sort/filter on macro CRUD (thread Pagination→AdminReqParsed; DB already supports it) | projects/properties/tile-servers honor sortBy/order/q | **P0** |
| `GET /api/v2/{resource}?ids=…` batch-by-ids | dataProvider.getMany stops doing N parallel GETs (reference fields) | **P1** |
| `GET /api/v2/{geofences,routes}/choices` slim `{id,name}[]` | ReferenceInput selects stop pulling whole FeatureCollection | **P1** |
| `GET /api/v2/stats` aggregate (entity counts, job rollup, plugin counts, golbat totals) | dashboard one-shot | **P1** |
| `DELETE /api/v2/{resource}?ids=…` bulk delete | BulkDeleteButton stops doing N DELETEs | **P2** |
| `GET /api/v2/openapi.json` alias (utoipa serves JSON under .yaml) | codegen DX | **P2** |

## Deferred — `/map` backport (after admin lands; framework decision required)

Whole `web-client/pages/map/**` + `components/{drawer,dialogs/import}/**` + `hooks/{useShapes,
usePixi,useSyncGeojson,useLayers}.ts`. Renderer-agnostic parts (store linked-list logic, turf
combine/cut/split, S2/geohash math, all `/jobs` + `/geometry` + `/golbat-data` calls) **port
unchanged**. The Leaflet/geoman/arrowheads/pixi binding is the only renderer-coupled layer.

**Framework options (trace.md §10):**
1. **Reuse shadmin's Leaflet+geoman geo suite** — lowest friction, already Tailwind-themed, already built. ← likely path.
2. **Terra Draw** (headless, portable across Leaflet/MapLibre/OL) — best shadcn fit, survives a future basemap swap; one React-fragility risk to prototype.
3. **MapLibre GL + native circle/symbol layers** — retires Pixi, GL perf; the "real upgrade".
4. **OpenLayers** — strongest single-framework editing; only if backport becomes a full rewrite.

## Dropped (not in V2)

- MUI + `@emotion/*` + `assets/theme.ts` — replaced by Tailwind v4 + shadcn tokens.
- `useRaStore` — → ra-core `useStore`.
- `SaveToGolbat` / `getGolbatCache` — v2 gap; superseded by admin `publish`.
- `/play` Playground, react-admin-color-picker, codemirror stack, react-window (shadmin DataTable
  virtualizes), leaflet-easybutton/locatecontrol (map backport decides).
- `/convert` standalone route — fold conversion into admin or drop (confirm).
