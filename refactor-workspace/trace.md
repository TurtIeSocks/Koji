# Koji Frontend V2 — Refactor Trace (Phase 1)

_Generated 2026-06-19 via systematic-refactor Phase 1. Raw material; not the final plan._

## Contents

- [1. Koji web-client — App Shell & Routing](#1-koji-web-client--app-shell--routing)
- [2. Koji /admin — react-admin Resources](#2-koji-admin--react-admin-resources)
- [3. Koji /map — Leaflet Feature Inventory (backport spec)](#3-koji-map--leaflet-feature-inventory-backport-spec)
- [4. Koji web-client — Shared Layer (services/hooks/types/store)](#4-koji-web-client--shared-layer-serviceshookstypesstore)
- [5. Koji Backend — HTTP API Surface](#5-koji-backend--http-api-surface)
- [6. shadmin Library — Component Inventory & Contracts](#6-shadmin-library--component-inventory--contracts)
- [7. shadmin Demo — Patterns to Copy](#7-shadmin-demo--patterns-to-copy)
- [8. Research — MapLibre GL Ecosystem](#8-research--maplibre-gl-ecosystem)
- [9. Research — Alternative Map Frameworks](#9-research--alternative-map-frameworks)
- [10. Map Framework — Recommendation](#10-map-framework--recommendation)



---

# 1. Koji web-client — App Shell & Routing

I have everything needed. Producing the dense trace.

---

# Koji web-client — App Shell + Routing Trace

## Stack / build
- **Framework**: React 18.2 (`react`/`react-dom` ^18.2.0), classic JSX runtime (`vite.config.ts:11` `jsxRuntime:'classic'`; `tsconfig.json:20` `"jsx":"react"`). `package.json:name = "koji"`, v1.4.0.
- **Router**: `react-router` + `react-router-dom` **^6.6.1** (data router API — `createBrowserRouter`/`RouterProvider`). `package.json:46-47`.
- **Admin**: `react-admin` **^4.11.3** + `ra-data-simple-rest` ^4.11.3 + `ra-core`. `package.json:38,42`.
- **UI**: MUI **^5.11.2** (`@mui/material`, `@mui/icons-material` ^5.11.0, `@mui/x-date-pickers` ^5.0.12), emotion 11. `package.json:24-27`.
- **State**: `zustand` **^4.3.6** (`package.json:60`).
- **Map**: `react-leaflet` 4.2.1, `leaflet` ^1.9.3, `leaflet-pixi-overlay`, `pixi.js` ^6.5.8, geoman, locatecontrol. `package.json:48-52`.
- **Bundler**: Vite **^4.3.9** + `@vitejs/plugin-react` + `vite-plugin-checker` (^0.6.1, overlays TS + eslint). TypeScript ^4.9.3, sass for `.scss`.
- **Build out**: `vite build` → `dist/` (`vite.config.ts:40` `outDir`, `:44` `assetsDir:''` so assets sit flat in dist root, `:43` single input `index.html`, `:39` legacy browser targets safari11.1/chrome64/firefox66/edge88, `:41-42` sourcemap+unminified only in dev).

## Entry chain
- `index.html:16` `<div id="root">`; `:33` `<script type="module" src="./src/index.tsx">`. Loads Material Icons font + **leaflet 1.8.0 CSS via unpkg CDN** (`:23-28`) + locatecontrol CSS via jsdelivr (`:29-32`) + Google Fonts preconnect. Title `Kōji`. Viewport locked `maximum-scale=1,user-scalable=no`. `<noscript>` fallback `:35`.
- `src/index.tsx:10` `createRoot(getElementById('root')!).render(...)` wrapping order: `React.StrictMode` → **`ErrorBoundary`** (`@components/ErrorBoundary`) → `App`. Imports global `@assets/index.scss` (`:4`).

## App.tsx — router + root providers (`src/App.tsx`)
Router built **module-level** via `createBrowserRouter` (`:19-60`), rendered through `<RouterProvider router={router}>` (`:104`). Path aliases: `@assets`,`@components`,`@hooks`,`@pages`,`@services` (`vite.config.ts:27-33`, `tsconfig.json:26-42`).

### Top-level routes (path → element, all share `errorElement=<ErrorPage error="500"/>`)
| Path | Element | Source | Notes |
|---|---|---|---|
| `/` | `<Home />` | `@pages/Home` | `App.tsx:21-24` landing grid of 4 nav tiles |
| `/login` | `<Login />` | `@pages/Login` | `:25-29` |
| `/map` | `<Map />` | `@pages/map` (dir `index.tsx`) | `:30-34` |
| `/map/:lat/:lon/:zoom?` | `<Map />` | same | `:35-39` params drive `forcedLocation`/`forcedZoom` |
| `/admin/*` | `<AdminPanel />` | `@pages/admin` (dir `index.tsx`) | `:40-44` wildcard → react-admin sub-router |
| `/convert` | `<ConvertPage />` | `@pages/Convert` | `:45-49` |
| `/play` | `<Playground />` | `@pages/Playground` | `:50-54` API playground (dev tool) |
| `*` | `<ErrorPage />` (default 404) | `@pages/Error` | `:55-59` catch-all |

Note import aliasing: `Map` here = the **page** `@pages/map` (`App.tsx:12`), distinct from `@components/Map` (leaflet wrapper) and `@mui/icons-material/Map`.

### Global providers (App component `:62-108`)
- **Theme**: `darkMode = usePersist((s)=>s.darkMode)` (`:63`); `theme = useMemo(()=>createTheme(dark?'dark':'light'),[darkMode])` (`:65-69`); side-effect sets `document.body.style.backgroundColor` to `theme.palette.background.paper`. `createTheme` = `@assets/theme`.
- Tree: `<ThemeProvider theme={theme}>` → `<CssBaseline/>` → `<RouterProvider/>` → conditional `{error && <ErrorPage error={error}/>}` (`:102-107`). **No top-level QueryClientProvider** (react-admin owns its own query client internally under `/admin`).
- **Zustand stores are not "mounted" via provider** (zustand is providerless); root references `usePersist` + `useStatic` (`:7-8`). `usePersist.getState()` / `useStatic.getState()` pulled imperatively at `:71-72`.

### Bootstrap / config gate (`App.tsx:74-99`)
- `fetched` state gates render: **`if (!fetched) return null`** (`:99`) — blank screen until config resolves.
- Effect (`:77-97`): `fetchWrapper<Config>('/api/v2/config')` (v2 envelope-unwrapped). On success: if persisted `location===[0,0]` seed `[start_lat,start_lon]`; `setStatic` for `dangerous`,`route_plugins`,`clustering_plugins`,`bootstrap_plugins`; **if `!res.logged_in` → `router.navigate('/login')`** (imperative, the auth redirect mechanism); then `setFetched(true)`. On null → `setError('Unable to fetch config…')`.

## Auth / login
- **Session-cookie based**, no token store on client. Login page `src/pages/Login.tsx`: `onSubmit` (`:26-42`) `POST /api/v2/auth/login {password}`; on 200 `navigate('/')`, else `setError('Wrong Password')`. Uses `useNavigate()` from `react-router` (`:18,24`). Comment `:30-31`: 200 `{authenticated:true}` on `KOJI_SECRET` match else 401.
- Redirect-to-login is **App-level** (config `logged_in:false` → `router.navigate('/login')`), not a route guard/loader. No `authProvider` is passed to react-admin's `<Admin>` (`admin/index.tsx:54-64` omits it) — admin relies on the same server session cookie.

## Error handling (3 layers)
1. **`ErrorBoundary` class** `src/components/ErrorBoundary.tsx` — wraps whole app in `index.tsx` AND re-wraps inside map page (`map/index.tsx:45`). `componentDidCatch` increments `errorCount`; **>5 errors → full-screen "Kōji encountered an error!" + Refresh button** (`:32-57`); ≤5 → recoverable `Notification` banner (`@components/notifications/Base`) + renders children (`:58-72`).
2. **Router `errorElement`** = `<ErrorPage error="500"/>` on every route (`App.tsx`).
3. **`ErrorPage`** `src/pages/Error.tsx` — default prop `error='404'`; renders `GradientText` of the code + `<Button component={Link} to="/">Back</Button>` (react-router-dom `Link`).

## MUI theming (`src/assets/theme.ts`)
- `create(mode)` → `responsiveFontSizes(createTheme({...}))` (`:9-52`). `palette.mode` from arg; primary/secondary commented out (defaults). `typography.h1.fontSize='10rem'`.
- `components` overrides: `MuiLink.defaultProps.component = LinkBehavior` (`@components/styled/LinkBehavior`), `MuiButtonBase.defaultProps.LinkComponent = LinkBehavior` (`:27-36`) — routes MUI links through react-router; `MuiGrid2` centered defaults (`:37-43`); `MuiPaper` `elevation:0, square:true` (`:44-49`).
- **react-admin theme** (`admin/index.tsx:59-62`): merges `{...defaultTheme, ...(useTheme() as RaThemeOptions)}` — pulls the live MUI theme via `useTheme()` so admin inherits dark/light from App's `ThemeProvider`.

## `/admin` sub-app (`src/pages/admin/index.tsx`)
- `<Admin basename="/admin" title="Kōji Admin" dataProvider={dataProvider} disableTelemetry theme={...} layout={Layout}>` (`:54-64`). **`basename="/admin"`** is what makes the `/admin/*` wildcard work (react-admin owns its own nested router under that prefix).
- On mount `useEffect → getFullCache()` (`:48-50`) → loads geofence/route/project/golbat caches into `useDbCache`.
- **Resources** (`:65-116`): `project` (icon AccountTree), `geofence` (Architecture), `route` (Route), `property` (ListAlt), `tileserver` (Map), `plugins` (Extension; list+edit only, no show/create). Each wires `list/edit/show/create` from its sibling dir + `recordRepresentation = record.name`.
- `<NetworkAlert/>` rendered alongside `<Admin>` (`:118`).
- **Layout** `admin/Layout.tsx`: thin wrap of RA `Layout` injecting custom `AppBar`. **AppBar** `admin/AppBar.tsx`: RA `BaseAppBar` + title slot + icon links — external info `https://koji.vercel.app/` (`:18`, `target=_blank`), Home `href="/"` (`:25`), Map `href="/map"` (`:28`), `ThemeToggle`.
- **dataProvider** `admin/dataProvider.ts`: `simpleRestProvider('/', httpClient)` overridden for v2. `RESOURCE_MAP` (`:35-42`) maps RA resource → v2 segment + `geo` flag: geofence→`geofences`(geo), route→`routes`(geo), project→`projects`, property→`properties`, tileserver→`tile-servers`, plugins→`plugins`. Geo resources project GeoJSON Feature→flat RA row (`featureToRecord` `:74-85`); `unwrap` strips v2 `{status:'ok',data}` envelope (`:88-89`). Overrides: `getList`/`getManyReference` (`:101-152` 1-based `?page&per_page&sortBy&order&q`), `getMany` (parallel per-id, no batch endpoint `:153-174`), `getOne` (`:175-185`), `create`/`update`(PATCH)/`delete`(204)/`deleteMany` (`:186-239`). Plugins use composite id `kind:name` → path `/{kind}/{name}` (`itemPath` `:140-147`).

## `/map` page (`src/pages/map/index.tsx`)
- `MapWrapper` (`:32`): `useParams()` (react-router) → `forcedLocation`/`forcedZoom` from `:lat/:lon/:zoom`. `drawerWidth` 515 (Geojson tab) else 345 from `usePersist.menuItem` (`:35`). `useEffect → getFullCache()` (`:38-40`).
- Tree: `Box(flex)` → `Loading` + `ErrorBoundary` → `DrawerIndex`(`@components/drawer`) + `Main`(styled, drawer-aware) → `@components/Map` (leaflet) with Panes (z 500-505) + `Markers`×4 categories (pokestop/station/spawnpoint/gym) + `Interface` + vector layers (Points/MultiPoints/LineStrings/MultiLineStrings/Polygons via `./markers/Vectors`) + `S2Cells`/`SimplifiedPolygons` (`./markers/S2`) + alert/dialog overlays (NetworkAlert, GeneralAlert, Import/Export Polygon+Route, KeyboardShortcuts) (`:42-93`).

## Root state stores (zustand, providerless)
- **`usePersist`** `src/hooks/usePersist.ts` — `create(persist<UsePersist>(...,{name:'local',partialize:…}))` (`:94-179`). **localStorage key `'local'`**, persists all keys except `export` (`:172-177`). Holds drawing/client settings/layers/clustering/dev config + `setStore(key,value)` setter (`:169`). Defaults: `darkMode:true`, `zoom:18`, `location:[0,0]`, `category:'pokestop'`, `cluster_mode:'Balanced'`, default carto voyager tileServer (`:111-112`).
- **`useStatic`** `src/hooks/useStatic.ts` — `create<UseStatic>` (`:84-169`), **non-persisted** runtime/session state: `notification`, `bounds`, `loading`/`loadingAbort`, `geojson` FeatureCollection, plugin lists (`route_plugins`/`clustering_plugins`/`bootstrap_plugins`), `dangerous`, `tileServers`, `kojiRoutes`/`golbatRoutes`, `importWizard`, `layerEditing`, `dialogs`, `projects`, `combinePolyMode`; setters `setStatic` (`:148`), `setGeojson` (`:153`).
- Other stores referenced by services: `useShapes`, `useDbCache` (`services/fetches.ts:17-18`).

## API layer + envelope (`src/services/fetches.ts`)
- `fetchWrapper<T>` (`:33-62`) — central fetch; parses **v2 envelope** `{status:'ok',data,meta?}|{status:'error',error}`, returns unwrapped `data` (`T`) or `null`; on error pushes `useStatic.notification`. All app calls hit **`/api/v2/*`** (config, jobs, golbat-data, geometry/convert, s2, geofences/routes, auth/login, projects). Async calc via job queue: `POST /api/v2/jobs` → `pollJob` long-polls `GET /api/v2/jobs/{id}?wait=10` until terminal (`:75-106`).

## Env usage
- **No `import.meta.env` anywhere** in src (grep empty).
- **`process.env.NODE_ENV === 'development'`** used in 8 files for dev-only UI/logging: `components/drawer/Settings.tsx:92`, `hooks/useSyncGeojson.ts:29`, `pages/map/markers/{Point.tsx:156,index.tsx:39,S2.tsx:35,48,96,182}`, `pages/map/popups/{Point.tsx:76,LineString.tsx:13}`, `services/fetches.ts:154,184,550`.
- `src/vite-env.d.ts`: only `/// <reference types="vite/client" />`.

## Dev server / proxy (`vite.config.ts:47-71`)
- `host 0.0.0.0`, `port 8081`, `open:true`. Proxies `/api`, `/internal`, `/config` → `http://0.0.0.0:8080` (the koji-server backend). `fs.strict:false`.

## How the built SPA is served (Rust — `crates/koji-service/src/lib.rs`)
- **Served by koji-server via `actix_files`** (`use actix_files::{Files, NamedFile}` `lib.rs:3`). Static root `path()` (`:293-300`): `"./dist"` when `is_docker()` else `"../client/dist"` (note: legacy relative path, not `apps/web-client/dist`).
- `is_docker()` (`utils/mod.rs:19-23`) = `current_dir().join("dist").is_dir()`.
- Mount (`:390-402`): `Files::new("/", path()).index_file("index.html")` with a `default_handler` that **serves `index.html` for any unmatched path** — this is the SPA fallback enabling react-router + react-admin client-side wildcards (`/admin/*`, `/map/:lat/:lon`, `*`). **Registered last**, after `/api`, `/api/v2/openapi.yaml`, `/healthz`, `/readyz`.
- **No base path** on the SPA — served from `/` (Vite has no `base` config → defaults `/`; `index.html` references `./src/index.tsx` relative; `assetsDir:''` flat). API base path is `/api/v2`.
- Backend bind: `HOST` env or `0.0.0.0`, `PORT` env or `8080` (`:404-409`). Session cookies via actix `SessionMiddleware` + `CookieSessionStore` (`:327-333`), `cookie_secure` unless `KOJI_INSECURE_COOKIES` set, `SameSite::Lax`.
- **Docker** (`Dockerfile`): stage `client` = `node:22-alpine`, `yarn install && yarn build` over `apps/web-client` → `/app/dist`; runner copies `--from=client /app/dist ./dist` (`:34`), so the docker-runtime cwd has `./dist` → `is_docker()` true → serves `./dist`.

## Key observations for refactor
- Three-name `Map` collision (page `@pages/map`, component `@components/Map`, icon).
- Auth is an App-effect redirect, not a router loader/guard — no protected-route abstraction; `/admin` relies purely on the server session, no RA `authProvider`.
- `if (!fetched) return null` blanks the whole tree during config fetch (no loading UI at root; `Loading` exists only inside `/map`).
- Router is constructed at module scope but `router.navigate` is called from the App effect — couples module-level router to component lifecycle.
- Static-serve relative path `"../client/dist"` (non-docker) is stale vs the actual `apps/web-client/dist` location — dev relies on the Vite proxy instead, so this path only matters for a non-docker `cargo run` serving prebuilt assets.


---

# 2. Koji /admin — react-admin Resources

I have everything. Producing the dense trace.

---

# Koji react-admin ADMIN trace

## `<Admin>` setup & wiring
- `apps/web-client/src/App.tsx:13,41-44` — route `/admin/*` → `AdminPanel` (`@pages/admin`), wrapped in app-level `ThemeProvider`/`createBrowserRouter`. Auth gate is app-level: `App.tsx:80-91` fetches `GET /api/v2/config`; if `!res.logged_in` → `router.navigate('/login')`. **No react-admin `authProvider`** anywhere.
- `apps/web-client/src/pages/admin/index.tsx:54-117` — `<Admin basename="/admin" title="Kōji Admin" dataProvider={dataProvider} disableTelemetry theme={{...defaultTheme, ...muiTheme}} layout={Layout}>`. No `authProvider`, `i18nProvider`, `dashboard`, or custom routes. `useEffect`→`getFullCache()` (`index.tsx:48-50`) preloads the map db-cache (not RA-managed). Renders `<NetworkAlert/>` (`@components/notifications/NetworkStatus`) sibling to `<Admin>`.
- **6 `<Resource>`s** (`index.tsx:65-116`): `project` (icon `AccountTree`), `geofence` (`Architecture`), `route` (`Route`), `property` (`ListAlt`), `tileserver` (`Map`), `plugins` (`Extension`). All set `recordRepresentation={(r)=>r.name||''}`. `plugins` has **no Create, no Show** (list+edit only); all others have full LECS.

## Layout / AppBar / Menu
- `Layout.tsx:6-8` — wraps RA `<Layout>` with custom `appBar={AppBar}`. No custom `<Menu>` (default RA menu, auto from resources).
- `AppBar.tsx:10-33` — extends RA `<AppBar>`: title slot (`Typography#react-admin-title`), `IconButton`→`https://koji.vercel.app/` (info, `target=_blank`), `IconButton href="/"` (Home), `IconButton href="/map"` (Map), `<ThemeToggle/>` (`@components/ThemeToggle`).

## dataProvider (`dataProvider.ts`)
- Base: `ra-data-simple-rest` `simpleRestProvider('/', httpClient)` (`:71`), but **every CRUD verb is overridden** — base only used as spread skeleton. Custom `httpClient` (`:48-69`) wraps `fetchUtils.fetchJson`, rebuilds `Headers`. No auth header / token injection (relies on session cookie via `fetchJson`).
- `RESOURCE_MAP` (`:35-42`) maps RA resource name → v2 segment + `geo` flag:
  - `geofence`→`geofences` `geo:true`; `route`→`routes` `geo:true`; `project`→`projects`; `property`→`properties`; `tileserver`→`tile-servers`; `plugins`→`plugins`.
- All endpoints under `/api/v2/{seg}`. v2 ok-envelope unwrapped via `unwrap()` (`:88`, `json.status==='ok' ? json.data : json`).
- **Geo resources** (`geofence`/`route`): list returns GeoJSON `FeatureCollection`, get-one a `Feature`. `featureToRecord()` (`:74-85`) flattens feature→row `{...properties, id, name, mode, geometry, geo_type}` with `__id/__name/__mode` fallbacks; mode defaults `'unset'`.
- `getList` (`:101-135`): query `?page=&per_page=&sortBy=&order=&q=` + spread `params.filter`. `total = meta?.total ?? records.length` (geo resources return whole collection, no server pagination — flagged TODO `:22-28`). `getManyReference = getList` (`:152`).
- `getMany` (`:153-174`): no batch endpoint → parallel `Promise.allSettled` per id via `itemPath`, drop misses.
- `getOne` (`:175-185`): `itemPath`; geo normalizes FeatureCollection→`features[0]`.
- `itemPath` (`:140-147`): normal `/api/v2/{seg}/{id}`; **plugins composite id** `kind:name` → `/api/v2/plugins/{kind}/{name}` (split on `:`).
- `create` POST (`:186-195`) `/api/v2/{seg}`; synth `id:'0'` if absent. `update` PATCH (`:196-212`) `itemPath`. `delete` DELETE→204, synth `{data:{id}}` (`:213-221`). `deleteMany` parallel DELETEs (`:222-239`).
- Numerous `TODO(v2-verify)` re: lossy GeoJSON→row projection, collapsed 4-mode set, ignored server-side page/sort/filter.

## Shared inputs / components
- `inputs/CodeInput.tsx` — CodeMirror JSON editor (`@components/Code.tsx` = `@uiw/react-codemirror` + `@codemirror/lang-json` + `linter(jsonParseLinter())`). Props `source,label,conversionType,geometryType`. On blur: `safeParse`→`convert(parsed, conversionType, simplifyPolygons, type)` (`POST /api/v2/geometry/convert?format=<type>`, `fetches.ts:558-585`); warns if multiple features returned. `Code` editor also auto-fetches if value `startsWith('http')` (remote JSON). Used by Geofence (`geometry`, Polygon) & Route (`geometry`, MultiPoint) forms.
- `inputs/Properties.tsx` — three controlled inputs over RA `useInput`: `BoolInputExpanded` (MUI `Switch`), `TextInputExpanded` (MUI `TextField`, text/number/database types, coerces number), `ColorInputExpanded` (**`react-admin-color-picker` `ColorInput`** = the color-picker, default `#000000`, validates `#`-prefix). Used by Property form + Geofence property-array.

## Custom actions / bulk-actions (`actions/`)
- `PushToApi.tsx` — `PushToProd` (row) & `BulkPushToProd` (bulk): `POST /api/v2/{geofences|routes}/{id}/publish` (`PUBLISH_SEG` map `:23`), MUI `SyncIcon`, label "Sync", `react-query` mutation, `useNotify`. TODO flags publish-vs-golbat-sync semantic mismatch + 422 on no Dragonite area.
- `Export.tsx` — `ExportButton`/`BulkExportButton`: `react-query` `enabled:false` fetch via `getUrl` (`:32-49`, `/api/v2/{geofences|routes}/{id}`; **project falls back to geofences/{id} — flagged wrong** `:38-39`), pushes result into `useImportExport` Zustand store → opens `exportPolygon` dialog (`@components/dialogs/Polygon`, rendered as `<ExportPolygon/>` at end of each geo/project List).
- `AssignProjectFence.tsx` — `BulkAssignButton`/`AssignFencesToProjects`: bidirectional geofence↔project link. Always PATCHes the **geofence** side `PATCH /api/v2/geofences/{id} {projects:[...]}`. Uses `useRaStore` (`@hooks/useRaStore`) keys `bulkAssignGeofence`/`bulkAssignProject` for dialog open; `KojiAuto` (`@components/AutoComplete`), `useGetMany('geofence'|'project')`, `useUnselectAll`, `useNotify`. TODO: destructive full-replace semantics.
- `AssignParentFence.tsx` — `BulkAssignFenceButton`/`AssignParentToFences` (geofence-only): `PATCH /api/v2/geofences/{id} {parent}` (`selected===0`→Remove/clear). MUI `Autocomplete`, store key `bulkAssignParent`.
- `Extras.tsx` — `ExtraMenuActions`: MUI `Menu` (`MoreVertIcon`) wrapping `DeleteWithUndoButton` + `ExportButton` + mobile-only `PushToProd`. Used in geofence/route datagrids.

## Import flow reachable from admin
- Geofence Create page (`GeofenceCreate.tsx:35-36`) renders `GeofenceCreateButton` ("Open the Wizard", `geofence/CreateDialog.tsx`) which opens **`ImportWizard`** (`@components/dialogs/import/ImportWizard.tsx`) via `useStatic` store (`importWizard.open`). 5-step stepper (Import→Properties→Fences→Routes→Confirm) + tabs (select/code/preview). Code tab uses CodeMirror `<Code>`. `AssignStep.tsx` bulk-sets mode/parent/projects/geofence_id per feature (reads `/api/v2/projects?per_page=9999`, `useDbCache`). Finish (`Finish.tsx`) → `SaveToKoji` (`buttons/SaveToKoji.tsx`): loops `save('geofences', …)` then `save('routes', …)` (one `POST /api/v2/geofences|routes` per feature), re-caches via `getKojiCache`; maps internal `__name/__mode/__projects/__geofence_id/__parent` props. On close → `refresh()`+`redirect('list','geofence')`.

## RESOURCE TABLE

| resource | v2 endpoint (seg) | List cols (Datagrid) | Filter / Form inputs | Edit/Create special | Show fields | special features |
|---|---|---|---|---|---|---|
| **project** `index.tsx:65` | `projects` (plain) | `name`, `description`, `api_endpoint`(BooleanField looseValue), `api_key`(BooleanField looseValue), `golbat`(BooleanField), `geofences.length`(NumberField "Geofences"); `EditButton`,`DeleteWithUndoButton`,`PushToProd`,`ExportButton` `ProjectList.tsx:50-61` | Filter: `<SearchInput source="q" alwaysOn>`. Form (`ProjectForm.tsx`): `name`(req,fullWidth), `description`, `golbat`(BooleanInput), `api_endpoint`(helper Unown hint), `api_key`(helper hint) | Edit adds `ReferenceArrayInput source="geofences" reference="geofence"` + `AutocompleteArrayInput` (custom OptionRenderer/inputText/matchSuggestion, perPage 1000). Create: `mutationOptions.onSuccess`→notify+redirect | `name`,`description`,`api_endpoint`,`api_key`,`golbat`(Bool), `ReferenceArrayField geofences→geofence` (ChipField) | Bulk: `BulkDeleteWithUndo`+`BulkAssignButton`(→assign geofences)+`BulkExportButton`. `<ExportPolygon/>` dialog. ref→geofence. **Export endpoint wrong (falls to geofences/{id})** |
| **geofence** `index.tsx:74` | `geofences` (**geo**) | `name`, `ReferenceField source="parent" reference="geofence"`, `mode`, `geo_type`; `EditButton`,`PushToProd`,`ExtraMenuActions` `GeofenceList.tsx:59-65`. `rowClick="expand"`→`GeofenceExpand` | Filter aside (`GeofenceFilter.tsx`): `FilterLiveSearch`; FilterLists Project(`{project:id}`, from `useGetList('project')`), Parent(`{parent:id}`, from `GET /api/v2/geofences`→features), Geography Type(`{geotype}` Polygon/MultiPolygon), Mode(`{mode}` unset+`UNOWN_FENCES`). Form (`GeofenceForm.tsx`): `name`(req), `mode`(SelectInput `UNOWN_FENCES`), `parent`(ReferenceInput→geofence), `properties`(ArrayInput+SimpleFormIterator: per-row `property_id` ReferenceInput→property w/ AutocompleteInput groupBy category, + dynamic value input by category via FormDataConsumer: bool/string/number/object(N.I.)/array(N.I.)/color(ColorInput)/database), `geometry`(**CodeInput** Polygon), live `GeofenceMap` | Edit (`mutationMode="pessimistic"`, `transform` JSON.parse geometry) adds `ReferenceArrayInput source="projects" reference="project"` + AutocompleteArrayInput. Create: `transform` sets `id:0`; renders Create-Multiple (`ImportWizard`) + Divider + Create-One (`GeofenceForm`) | `name`,`mode`,`geo_type`,`area.geometry.type`(label Geometry Type), `PropertyFields` (custom: maps `record.properties` name:value), `ReferenceArrayField projects→project`(Chip), `GeofenceMap` preview, raw geometry in `<Code>` | Bulk: `BulkAssignFenceButton`(parent)+`BulkDeleteWithUndo`+`BulkAssignButton`(projects)+`BulkPushToProd`+`BulkExportButton`. Validation: `name` required only. refs: parent→geofence, projects→project, properties→property. **Import Wizard write path.** Loads properties via `GET /api/v2/properties?per_page=9999` |
| **route** `index.tsx:83` | `routes` (**geo**) | `name`, `description`, `mode`, `ReferenceField source="geofence_id" reference="geofence"`, `points`(NumberField); `EditButton`,`PushToProd`,`ExtraMenuActions` `RouteList.tsx:51-61` | Filter aside (`RouteFilter.tsx`): `FilterLiveSearch`; FilterLists Mode(`{mode}` `UNOWN_ROUTES`+unset), Points buckets(`{pointsmin,pointsmax}` ×1000), Geofence(`{geofenceid:id}` from `GET /api/v2/geofences`). Form (`RouteForm.tsx`): `name`(req), `description`, `mode`(SelectInput `UNOWN_ROUTES`), `geofence_id`(ReferenceInput→geofence, req), Points(FunctionField, disabled, counts coords), `geometry`(**CodeInput** MultiPoint), live `RouteMap` | Edit/Create `mutationMode="pessimistic"`, `transform` JSON.parse geometry; Create sets `id:0`, notify+redirect | `name`,`description`,`mode`, `ReferenceField geofence_id→geofence`, `points`, `RouteMap` preview, raw geometry in `<Code>` | Bulk: `BulkPushToProd`+`BulkDeleteWithUndo(size=small)`+`BulkExportButton`. ref geofence_id→geofence. `RouteMap` renders MultiPoint as numbered Points w/ `next` prop |
| **property** `index.tsx:92` | `properties` (plain) | `name`, `category`, `default_value`, `geofences.length`(NumberField "Geofences", `sortable=false`); `EditButton`,`DeleteWithUndoButton` `PropertyList.tsx:38-48` | Filter: `<SearchInput source="q" alwaysOn>`. Form (`PropertyForm.tsx`): `name`(req), `category`(SelectInput `PROPERTY_CATEGORIES`, req, drives local state), dynamic `default_value` input by category (Bool/Text/Number/object(N.I.)/array(N.I.)/color(ColorInput)/database(explanatory Typography)) | Edit `pessimistic`. Create passes `create` prop. Category-change warning Typography (resets values) shown when `!create && record.category!==tempState` | `name`,`category`,`default_value`,`created_at`(DateField),`updated_at`(DateField) | Bulk: `BulkDeleteWithUndo` only. No refs. No push/export. Custom value editors from `inputs/Properties` |
| **tileserver** `index.tsx:101` | `tile-servers` (plain) | `name`, `url`; `EditButton`,`DeleteWithUndoButton` `TileServerList.tsx:35-39` | No filter. Form (`TileServerForm.tsx`): `name`(req), `url`(req), live `TileServerMap` (react-leaflet `TileLayer`, fallback cartocdn url) | Edit `pessimistic`. Create notify+redirect | `name`,`url`, `TileServerMap` preview | Bulk: `BulkDeleteWithUndo` only. No refs/push/export |
| **plugins** `index.tsx:110` | `plugins` (plain, **composite id `kind:name`**) | `name`, `kind`, `enabled`(BooleanField), `version`; `EditButton`. `rowClick="edit"`, `bulkActionButtons={false}` `PluginList.tsx:13-20` | No filter. **No Create.** Edit form (`PluginEdit.tsx`): `entrypoint`/`interpreter`/`protocol`(TextInput **disabled**, disk-owned manifest), `enabled`(BooleanInput), `args_default`(TextInput multiline, **JSON parse/format** custom — JSON editor-lite), `description` | Edit `pessimistic` | — (no Show) | No bulk actions, no refs. Composite id routing in `itemPath` (`dataProvider.ts:142-145`) → `/plugins/{kind}/{name}` |

## Constants (`assets/constants.ts`)
- `UNOWN_FENCES = ['auto_quest']` (`:107`); `UNOWN_ROUTES = ['circle_pokemon','circle_raid','circle_quest','circle_station']` (`:109-114`); `PROPERTY_CATEGORIES = ['boolean','string','number','object','array','database','color']` (`:143-151`); `GEOMETRY_CONVERSION_TYPES = ['Point','MultiPoint','Polygon','MultiPolygon']` (`:136-141`).

## Notes for refactor
- No `authProvider`/`i18nProvider`/`queryClient` customization, no dashboard, no `<CustomRoutes>`. Auth entirely external (config `logged_in` + `/login` redirect in `App.tsx`).
- `getMany`/`getManyReference` exploit `useGetMany('geofence',{ids:[0]})` in assign dialogs as a "fetch all/representation" hack (id 0 sentinel).
- Cross-store coupling: actions use Zustand `useRaStore` (`@hooks/useRaStore`), `useImportExport`, `useStatic`, `useDbCache`, `usePersist` — outside RA's own store.
- `ExportPolygon` dialog (`@components/dialogs/Polygon`) is rendered standalone inside project/geofence/route Lists, driven by `useImportExport` state, not an RA construct.


---

# 3. Koji /map — Leaflet Feature Inventory (backport spec)

I have full coverage. Compiling the dense trace inventory.

# Koji web-client MAP section — full feature inventory

## Map setup / core
- **react-leaflet `4.2.1`** + **leaflet `^1.9.3`**; `MapContainer` w/ `maxBounds [[-85,-180],[85,180]]`, center/zoom from persist (or URL `:lat/:lon/:zoom` params) — `components/Map.tsx:31-51`, `pages/map/index.tsx:48-62`.
- **Tile layer**: single raster `TileLayer`, URL from `usePersist.tileServer` (default CartoDB Voyager `voyager_labels_under` `constants.ts` default `usePersist.ts:111`), keyed on tileServer for re-init; `ATTRIBUTION` = inline Koji/TurtleSocks SVG (`constants.ts:22`). `renderOwnTileLayer` prop lets dialogs (Convert/MiniMap) skip it. Tile-server list fetched `GET /api/v2/tile-servers?per_page=9999` (`Settings.tsx:35`).
- **Custom z-index Panes** (`pages/map/index.tsx:64-69`): `dev_markers`505, `circles`504, `lines`503, `arrows`502, `polygons`501, `s2`500.
- **leaflet-pixi-overlay `^1.9.4`** + **pixi.js `^6.5.8`** (`hooks/usePixi.ts`): GPU sprite layer for the high-count golbat data markers (pokestop/gym/spawnpoint/station — potentially tens of thousands). SVG icon → base64 data-uri sprite per category (`ICON_SVG`) or per-geohash color (`getHashSvg`). `PIXI.Loader.shared`, anchor 0.5/0.5, scale `1/getScale(zoom)`; `scaleMarkers` toggles dynamic-zoom scaling (else fixed zoom 16). Dev-only fallback to native leaflet `Circle`s when `nativeLeaflet` flag set (`markers/index.tsx:99-165`).
- **leaflet.locatecontrol `^0.79.0`** (`interface/Locate.tsx`): `L.control.locate` bottomright, `keepCurrentZoomLevel`, `setView:'untilPan'`.
- **leaflet-easybutton `^2.4.0`** (`interface/EasyButton.tsx` + `interface/index.tsx:54`): one custom bottomright button (shield SVG) → navigate `/admin`. Also `ZoomControl position="bottomright"`.
- Map-move handler writes `location`/`zoom`→persist and `bounds`→useStatic on `moveend` (`interface/index.tsx:33-47`).

## Geometry editing — leaflet-geoman (`react-leaflet-geoman-v2 ^0.2.2`, vendored geoman fork `github:TurtIeSocks/leaflet-geoman#a42d8fd`)
- `<GeomanControls>` in a `FeatureGroup` (`interface/Drawing.tsx:47-478`). Toolbar topright. **Enabled draw tools**: `drawCircle`, `drawRectangle`, `drawPolygon`. **Disabled**: drawText, drawMarker, drawCircleMarker, drawPolyline.
- **Global modes wired** (each writes `useStatic.layerEditing.{drawMode,cutMode,dragMode,editMode,removalMode,rotateMode}`): draw, **cut**, **drag**, **edit**, **remove**, **rotate** — `Drawing.tsx:304-351`.
- **Editable geometry types**: Polygon, MultiPolygon, Rectangle (→Polygon), Circle (drawn as route points). Points/MultiPoints rendered as `Circle` are `pmIgnore` for MultiPoint but per-circle `pm:remove`/`pm:drag`/`pm:dragend` listeners attached in `Point.tsx:99-130`. LineStrings/arrows are `pmIgnore`+`snapIgnore`.
- **globalOptions**: `continueDrawing`, `snappable` (persist toggles), `radiusEditCircle:false`, templine radius = `radius||70` (Radius mode) else 100.
- **onCreate** (`Drawing.tsx:183-303`): Rectangle/Polygon→`setShapes('Polygon'|'MultiPolygon')` keyed by leaflet layer id. Circle→builds route point graph: sets `__forward`/`__backward`/`__multipoint_id` link props, creates connecting `LineString` features (incl. closing segment), tracks `firstPoint`/`lastPoint`/`newPoints`. Layer removed from FeatureGroup after capture (store is source of truth).
- **Per-shape geoman event re-binding** in refs:
  - Polygon (`markers/Polygon.tsx:67-130`): `pm:remove`, `pm:dragend`, **`pm:cut`** (replaces feature w/ cut result, sets `__leafletId`), **`pm:rotateend`**, `pm:edit` (only when `editMode`). Each → `setters.update`.
  - Point (`markers/Point.tsx:99-130`): `pm:remove`, `pm:drag`/`pm:dragend` → recompute S2 coverage (`s2Coverage`) + `update`.
- **Custom toolbar controls** (`Drawing.tsx:68-172`): `removalMode` actions (Lines/Circles/Polygons/Finish — bulk `remove` by type); `drawCircle` actions (Finish / New Route / Cancel — manage `activeRoute`+`newRouteCount`); custom **`mergeMode`** control ("Merge" / "Merge All" / cancel → `setters.combine()`).
- **Keyboard shortcuts** via `onKeyEvent` (`Drawing.tsx:363-470`): user-rebindable map of action→key (`usePersist.kbShortcuts`, edited in `dialogs/Keyboard.tsx`, categories in `KEYBOARD_SHORTCUTS`). Toggles draw modes, drag/edit/remove/rotate/cut, cycle tileServer, theme, drawer, layer toggles (arrows/circles/lines/polygons), data toggles (gyms/pokestops/spawnpoints).
- **combinePolyMode** (`onButtonClick`/`onActionClick`): polygon/point multi-select-to-merge; selection highlights ORANGE; popups suppressed while editing or combining (`interface/index.tsx:22-31`, `Polygon.tsx:49-64`, `Point.tsx:61-78`).
- Color revert on mode/button change via `revertColors()` recoloring all geoman layers (`Drawing.tsx:29-44`).

## Routes / MultiPoint
- **leaflet-arrowheads `^1.4.0`** (`markers/LineString.tsx:34-47`): each `Polyline` gets `.arrowheads({size:'30m', offsets:{end: dis/2 'm'}, pane:'arrows'})`; color by distance via `getColor` (rules from `usePersist.lineColorRules`, editable in `inputs/LineStringColor`).
- **Route model**: ordered points = `useShapes.Point` (id→Feature<Point>) connected by `LineString` features keyed `${a}__${b}`, with `firstPoint`/`lastPoint`/`activeRoute`/`newRouteCount`. A route persisted as `MultiPoint` feature; `activeRoute(id)` "explodes" a MultiPoint into editable Points+Lines and re-collapses the prior active one (`useShapes.ts:187-274`).
- **Rendering**: `Points`/`MultiPoints`/`LineStrings`/`MultiLineStrings` in `markers/Vectors.tsx`. `KojiMultiPoint` (`markers/MultiPoint.tsx`) renders each coord as Point + connecting Line (closes loop). Route-index `Tooltip` when `showRouteIndex`. Hover/click activates route (`setActiveMode` hover|click).
- **Route editing ops**: 
  - **Split** a line at midpoint — `splitLine` (`useShapes.ts:385-458`); triggered from `LineString` popup "Split" button (`popups/LineString.tsx:16`) and Point popup ◄+ / +► buttons (split backward/forward, `popups/Point.tsx:141-161`).
  - **Remove point** (rebridges neighbor lines), **Remove All** — `popups/Point.tsx:162-172`; bulk remove via geoman removal toolbar.
  - **Reroute**: Point popup "Reroute" (`popups/Point.tsx:194-280`) → `POST /api/v2/jobs {mode:'reroute', clusters:[[lat,lon]], instance, routing:{sortBy,pluginArgs}, output:{returnType:'feature',saveToGolbat}}` → `pollJob` → replace route w/ result `features[0]`.
  - **Join/merge** routes & polygons — `setters.combine` (`useShapes.ts:275-384`): unions selected polygons (turf `union`, optional cutout-keep via `keepCutoutsOnMerge`) + concatenates selected points/multipoints into one MultiPoint.
  - Point popup also: merge-points + create/save route — `POST /api/v2/geometry/merge-points?format=feature` then `POST|PATCH /api/v2/routes`; delete `DELETE /api/v2/routes/{id}`; "Export Route" → ImportExport route dialog.

## Popups (`pages/map/popups/*`), all in `StyledPopup` (`Styled.tsx`, MUI Paper, `autoPan:false autoClose:false`)
- **Point.tsx** (`MemoPointPopup`): lat/lng (+ dev: id, geohash9/12, S2 cell id+face); editable **Name** (`__name`), **Mode** select (`UNOWN_ROUTES`), **Geofence** select (lazy `getKojiCache('geofence')`); split ◄/► buttons; Remove / Remove All; **Export Route**; **Reroute** (job); Delete (route) / **Create|Save** (`POST|PATCH /api/v2/routes`, sets `setRecord('route',...)`).
- **Polygon.tsx** (`MemoPolyPopup`): editable Name/Mode(`UNOWN_FENCES`)/Parent-geofence; geometry type + **area km²** (`POST /api/v2/geometry/area`); per-category **stats** (`MemoStat`→`POST /api/v2/golbat-data/{category}/stats` returning `{total}`, auto-loads under `*MaxAreaAutoCalc`); **Run Clustering** (`clusteringRouting({feature})`), **Cluster Children** (`clusteringRouting({parent})`); **Map menu**: Export, Create Shape from Cutouts (`getFeatureCutouts`), Remove, Remove Intersecting/Non-Intersecting Polys (`filterPolys`), Remove/Combine Contained / Non-Contained Points (`filterPoints`), Split into Polygons / Remove this Polygon / Remove Others (`splitMultiPolygons`/`removeThisPolygon`/`removeAllOthers`), To MultiPolygon/To Polygon; **Database menu**: Save/Update Kōji (`POST|PATCH /api/v2/geofences`), Delete (`DELETE /api/v2/geofences/{id}`); golbat-direct save commented out (v2 gap → admin publish).
- **LineString.tsx** (`MemoLinePopup`): distance (m) + **Split** button.
- **Geohash.tsx / Markers** popups (dev): lat/lng + group/unique geohash.
- **S2 cell** (`markers/S2.tsx:33-53`): dev click copies cell id to clipboard; dev `Tooltip` shows id.

## Markers (golbat data layer)
- `Markers({category})` for pokestop/station/spawnpoint/gym (`pages/map/index.tsx:70-73`, `markers/index.tsx`). Fetches via `getMarkers` → `POST /api/v2/golbat-data/{category}` body `{last_seen, tth, area|bbox}` returning `{points:[[lat,lon]]}`, adapted to `PixiMarker {i:prefix+idx, p}` (prefix map g/p/v/s, `fetches.ts:483`). Query type `data`: `all`(v2-gap→empty) | `area`(geojson polys) | `bound`(bbox). Re-fetch on `data/geojson/bounds/enabled/last_seen/focus/pokestopRange/tth`; aborts on blur/move/update. `pokestopRange` adds 70m green range circles. `colorByGeohash`+`geohashPrecision` recolor.
- `ICON_RADIUS`/`ICON_COLOR` per prefix (`constants.ts:57-73`).

## Drawer / control panels (`components/drawer/*`) — tabs `TABS`: Drawing, Clustering, Layers, Manage, Geojson, Settings (`index.tsx`, `ICON_MAP`)
- **Drawing tab** (`drawer/Drawing.tsx`): toggles `snappable`, `continueDrawing`, `keepCutoutsOnMerge`; `radius` input (disabled in S2 mode); `setActiveMode` hover|click; `LineColorSelector` (distance→color rules).
- **Clustering/Routing tab** (`drawer/Routing.tsx`) — main calc panel:
  - `mode` select (`MODES`: cluster|bootstrap).
  - `category` (`CATEGORIES`: pokestop/gym/fort/spawnpoint/station) + `tth` when spawnpoint.
  - `calculation_mode` (`CALC_MODE` Radius|S2, or bootstrap plugins). Radius→`radius`; S2→`s2_level`(`S2_CELL_LEVELS` 10-20), `s2_size`(`BOOTSTRAP_LEVELS` 1/3/5/7/9). Bootstrap+plugin→`bootstrapping_args`.
  - **Clustering** group (cluster mode): `min_points`, `cluster_mode` (`CLUSTERING_MODES` Honeycomb/Fastest/Fast/Balanced/Better + server `clustering_plugins`), `max_clusters` (unless Fastest), `clustering_args` (plugin), `center_clusters` toggle.
  - **Routing** group: `sort_by` (`SORT_BY` None/Random/S2Cell/Geohash/LatLon/PointCount + `route_plugins`; tsp special-label), `routing_args` (plugin).
  - **Saving**: `save_to_db`, `save_to_golbat`, `skipRendering` toggles; **Update** button → `clusteringRouting()` (disabled while editing or `updateButton`).
  - Backend: `clusteringRouting` (`fetches.ts:225-478`) builds tagged `CalcJobRequest` (`mode:'cluster'|'bootstrap'`, nested camelCase arg-groups clustering/bootstrap/routing/output/dataFilter) → `POST /api/v2/jobs` → `pollJob` (long-poll `GET /api/v2/jobs/{id}?wait=10`, abort→`DELETE`) → result FC `features[0]`. Plugin lists come from `useStatic.{route,clustering,bootstrap}_plugins`.
- **Layers tab** (`drawer/Layers.tsx`): Vectors toggles `showCircles/showLines/showPolygons/showArrows` (pane hide/show via `useLayers.ts`); Markers toggles `gym/spawnpoint(+tth)/pokestop/pokestopRange/station`; `data` query type all|area|bound; `last_seen` DateTime (UTC); **S2 Cells**: `s2DisplayMode` none|covered|all, `calculation_mode` Radius|S2, `s2FillMode` simple|all, multi-select `s2cells` levels (Radius) or `s2_level`+`s2_size` (S2).
- **Manage tab** (`drawer/manage/index.tsx`): Import Wizard; Import Polygons / Import Routes (code dialogs); Import from Golbat / Import Geofences / Import Routes / SelectProject (`Instance` selectors); Export Polygons / Export Routes; JSON Manager; Conversion Playground.
- **Geojson tab** (`drawer/Geojson.tsx`): live editable CodeMirror of full `useStatic.geojson`; parse→`setFromCollection` (round-trips map state).
- **Settings tab** (`drawer/Settings.tsx`): toggles `loadingScreen/simplifyPolygons/showRouteIndex/scaleMarkers`; Keyboard Shortcuts dialog; TileServer select; per-category Max Area to Auto Calc (km²); dev toggles `nativeLeaflet/colorByGeohash/geohashPrecision`; Documentation/Sponsor links; Admin Panel nav; **Logout** `POST /api/v2/auth/logout`.

## Import dialogs (`components/dialogs/import/*`) — wizard `ImportWizard.tsx` (5 steps: Import/Properties/Fences/Routes/Confirm; tabs select/code/preview)
- **ImportStep.tsx** geometry sources: **JSON** file (areas.json/geofence.json/any GeoJSON — `manage/Json`), **Shapefile** (`shapefile ^0.6.6`, `.shp`+optional `.dbf`, `ShapeFile.tsx`→`convert(...,'featureCollection')`), **Golbat** DB fences (`Instance` selector, `__golbat` tagged), **Nominatim** search.
- **Nominatim.tsx**: `GET /api/v2/nominatim?query=` → Polygon/MultiPolygon results in Autocomplete; only [Multi]Polygon selectable; tags `__nominatim`.
- **PropsStep**: rename/select feature properties. **AssignStep.tsx** (Fences + Routes modes): per-feature + bulk assign Name / Mode (`UNOWN_FENCES`|`UNOWN_ROUTES`) / Parent (geofence) / Projects (fence) or Geofence-parent (route); virtualized `ReactWindow`; projects from `GET /api/v2/projects?per_page=9999`. **MiniMap.tsx**: `LayersControl` preview (Collection bbox / Feature bbox / Features w/ tooltips). **Finish.tsx**: SaveToKoji / Send to Map / Download.

## Geometry ↔ backend wire format
- **Internal model**: `useShapes` holds GeoJSON `Feature`s split by geometry type (Point/MultiPoint/LineString/MultiLineString/Polygon/MultiPolygon/GeometryCollection), keyed by id. Coords stored GeoJSON order `[lon,lat]`; leaflet rendering flips to `[lat,lon]` (`Polygon.tsx:132-140`, `LineString.tsx:49`, `Point.tsx:134`).
- **Composite ids**: `${id}__${mode}__${KOJI|GOLBAT|CLIENT}` encode persistence source + mode (`getPolygonColor`/`getPointColor` color by `__GOLBAT`/`__KOJI`; `setFromCollection`/`add` parse).
- **Wire**: every `/api/v2/*` JSON is the envelope `{status:'ok',data,meta}|{status:'error',error}`; `fetchWrapper<T>` unwraps `data` or returns null+notification (`fetches.ts:33`). Raw exports (sql/text/poracle/altText) are NOT enveloped (`convert` reads body). KojiMeta props use `__name/__mode/__geofence_id/__parent/__id/__multipoint_id/__forward/__backward`; v2 row create/patch bodies use snake_case `name/mode/geofence_id/parent/geometry`.
- **Conversion**: `convert` → `POST /api/v2/geometry/convert?format=<type>` (`CONVERSION_TYPES`: array/multiArray/geometry(_vec)/feature(_vec)/featureCollection/struct/multiStruct/text/altText/poracle/sql; `GEOMETRY_CONVERSION_TYPES` Point/MultiPoint/Polygon/MultiPolygon) body `{area, simplify, output:{returnType,geometryType}}`.
- **S2 cells** (`nodes2ts ^3.0.0`): `getS2Cells` `POST /api/v2/s2/{level}` body `{bbox:{min_lat,min_lon,max_lat,max_lon}}` → `S2Response[]` (id, coords, 20k cap); coverage `s2Coverage` `POST /api/v2/s2/{circle|cell}-coverage` body `{lat,lon,radius?,size?,level}`→cell-id `string[]`. `SimplifiedCell` builds cell polys client-side via `S2Cell.getVertex` + turf `union`. `useSyncGeojson` recomputes coverage for all points on shape change.
- **Geohash** (`ngeohash ^0.6.3`): dev id display + per-geohash marker coloring (`getDataPointColor` seeded RNG, `seedrandom`).
- **save()** (`fetches.ts:629`): loops one `POST /api/v2/geofences|routes` per feature (v1 batch save-koji gone — always CREATE, dup risk flagged).

## Zustand stores
- **usePersist** (`persist` middleware, key `'local'`): all user/client settings + layer toggles + calc params + drawing prefs + `location/zoom/tileServer/kbShortcuts/lineColorRules/menuItem/drawer`. Full shape `usePersist.ts:15-92`; `setStore(key,value)`.
- **useShapes** (`useShapes.ts`): the geometry working set. `Point/MultiPoint/LineString/MultiLineString/Polygon/MultiPolygon/GeometryCollection` records; `activeRoute/firstPoint/lastPoint/newPoints/newRouteCount/combined/s2cellCoverage`; getters (`getFirst/getLast/getGeojson/getNewPointId/getPointsAsMp`); setters (`add/remove/update/updateProperty/combine/setFromCollection/activeRoute/splitLine`); `setShapes`. This is the map's source of truth; `useSyncGeojson` projects it into `useStatic.geojson`.
- **useStatic** (`useStatic.ts`): ephemeral runtime — `bounds`, `geojson` (synced FC), `notification`, `loading/loadingAbort/updateButton/totalStartTime/totalLoadingTime`, `layerEditing` (6 geoman flags), `combinePolyMode`, `dialogs{convert,manager,keyboard}`, `importWizard{...}`, `tileServers`, `{route,clustering,bootstrap}_plugins`, `projects`, `clickedLocation`, `selected`. `setStatic`, `setGeojson` (merges).
- **useDbCache** (referenced): `geofence/route/project/golbat` caches + `feature` cache; `getFromKojiKey/getRouteByCategory/getOptions/setRecord/setRecords`. Hydrated by `getFullCache`→`getKojiCache` (`GET /api/v2/{geofences|routes|projects}?per_page=9999`; geofence/route return GeoJSON FC lossily mapped to `DbOption`; golbat stubbed empty — v2 gap).

## Backend endpoint → trigger map
- `POST /api/v2/jobs` + `GET /api/v2/jobs/{id}?wait=10` + `DELETE /api/v2/jobs/{id}` — clustering/bootstrap (Routing Update, Polygon popup Run Clustering/Cluster Children) and reroute (Point popup).
- `POST /api/v2/golbat-data/{category}` — markers; `.../stats` — polygon popup counts.
- `POST /api/v2/geometry/{convert?format=,area,merge-points?format=feature}` — conversion playground / shapefile / polygon area / route merge.
- `POST /api/v2/s2/{level}` + `s2/{circle|cell}-coverage` — S2 layer + coverage.
- `GET /api/v2/nominatim?query=` — import.
- `GET|POST|PATCH|DELETE /api/v2/geofences[/{id}][?format=feature][/{id}/publish]` and `…/routes[/{id}]` — geometry CRUD.
- `GET /api/v2/projects`, `GET /api/v2/tile-servers`, `POST /api/v2/auth/logout`.

**Known v2 gaps/unverified flagged in code** (backport caveats): golbat-sourced route browsing removed (`getGolbatCache` stub); `data:'all'` markers mode returns empty; batch `save-koji` upsert→per-feature CREATE; geofence/route metadata cache lossy from GeoJSON; job poll loop + reroute + calc arg-group mapping + S2 endpoints runtime-unverified (no backend in env).

Files: all under `/Users/rin/GitHub/Koji/apps/web-client/src/` — `pages/map/{index,interface/*,markers/*,popups/*}.tsx`, `components/{Map,GeojsonWrapper}.tsx`, `components/drawer/**`, `components/dialogs/**`, `hooks/{usePersist,useShapes,useStatic,usePixi,useLayers,useSyncGeojson}.ts`, `services/{fetches,utils,geoUtils}.ts`, `assets/constants.ts`.


---

# 4. Koji web-client — Shared Layer (services/hooks/types/store)

I have full coverage. Producing the trace doc.

---

# Koji web-client SHARED layer trace

Base: `/Users/rin/GitHub/Koji/apps/web-client/src`. All `@assets`=`assets/`, `@hooks`=`hooks/`, `@services`=`services/`, `@components`=`components/`. Backend is **v2** (`/api/v2/*`, discriminated envelope, async job queue). Many calls carry `TODO(v2-verify)`/`TODO(v2-gap)` — flagged inline below.

## (1) API-CALL TABLE

All HTTP lives in `services/fetches.ts`. `fetchWrapper<T>` (`fetches.ts:33`) is the single fetch primitive: parses the v2 envelope `{status:'ok',data}|{status:'error',error}`, returns unwrapped `data` (`T`) or `null`, pushes `error.message` to `useStatic.notification`. `pollJob` (`fetches.ts:75`) long-polls `GET /jobs/{id}?wait=10` to terminal, `DELETE`s on abort.

| Function (`fetches.ts:line`) | Method | Endpoint | Used-by |
|---|---|---|---|
| `fetchWrapper<T>` `:33` | per-call | (generic wrapper; also fetches arbitrary URL incl. raw http in `Code.tsx:45`) | every fn below + `Code.tsx:7`, `App.tsx`, `drawer/Settings.tsx`, `drawer/inputs/{Instance,SelectProject,ProjectsAC}.tsx`, `dialogs/import/{Nominatim,AssignStep}.tsx`, `hooks/useImportExport.ts`, all of `pages/admin/*` (RouteFilter, Export, GeofenceFilter/Form, PushToApi), `pages/map/popups/{Polygon,Point}.tsx` |
| `pollJob` `:75` | GET (loop) / DELETE on abort | `GET /api/v2/jobs/{id}?wait=10`; `DELETE /api/v2/jobs/{id}` | `useImportExport.ts:267`, `pages/map/popups/Point.tsx`, internal to `clusteringRouting` |
| `getKojiCache<T>` `:136` (project branch) | GET | `GET /api/v2/projects?per_page=9999` (ROW records) | `SaveToKoji.tsx`, `popups/{Polygon,Point}.tsx`, `refreshKojiCache`, `getFullCache` |
| `getKojiCache<T>` `:136` (geofence/route branch) | GET | `GET /api/v2/geofences?per_page=9999` / `GET /api/v2/routes?per_page=9999` (returns GeoJSON FC → **lossy** `DbOption` derivation, `TODO(v2-verify):158`) | same as above |
| `getGolbatCache` `:208` | — | **STUBBED** (`TODO(v2-gap)`): v1 `/internal/routes/from_golbat` has no v2 equiv; clears `golbat` cache, returns `{}` | `SaveToGolbat.tsx`, `getFullCache`, internal `clusteringRouting:473` |
| `refreshKojiCache` `:189` | GET ×3 | parallel `getKojiCache('geofence'|'project'|'route')` | (no direct callers found — dead) |
| `getFullCache` `:217` | GET ×4 | geofence+route+project caches + golbat stub | `pages/admin/index.tsx`, `pages/map/index.tsx` |
| `clusteringRouting` `:225` | POST + poll | `POST /api/v2/jobs` (tagged `CalcJobRequest`: `mode:'cluster'|'bootstrap'`, nested camelCase arg-groups `clustering`/`bootstrap`/`routing`/`output`/`dataFilter`), then `pollJob` → terminal `JobRecord.result` (`{data:FC,stats}`); refreshes route/golbat cache on save. v1 `fast` flag dropped (`:381`). Whole job path `TODO(v2-verify):327` | `drawer/Routing.tsx`, `popups/Polygon.tsx` |
| `getMarkers` `:490` | POST | `POST /api/v2/golbat-data/{category}` body `{last_seen,tth, area|bbox}` → `{points:[[lat,lon]]}` adapted to `PixiMarker` via `MARKER_PREFIX` (`:483`). `data:'all'` mode → empty (`TODO(v2-gap):504`) | `pages/map/markers/index.tsx` |
| `convert<T>` `:558` | POST | `POST /api/v2/geometry/convert?format=<return_type>` body `{area,simplify,output:{returnType,geometryType}}`. **Not** via `fetchWrapper` — reads body itself (raw sql/text/poracle bodies aren't enveloped, `:596`) | `App.tsx`, `drawer/manage/index.tsx`, `drawer/manage/Json.tsx`, `drawer/manage/ShapeFile.tsx`, `dialogs/Convert.tsx`, `useImportExport.ts:98,126`, `useStatic.ts`, `pages/admin/inputs/CodeInput.tsx` |
| `save` `:629` | POST (loop) | `POST /api/v2/geofences` or `POST /api/v2/routes`, **one create per feature** (no batch upsert; always CREATE → dup risk, `TODO(v2-verify):620`). Returns `{updates:0,inserts:n}` | `SaveToKoji.tsx`, `SaveToGolbat.tsx`, `popups/Polygon.tsx` |
| `getS2Cells` `:678` | POST | `POST /api/v2/s2/{level}` body `{bbox:{min_lat,min_lon,max_lat,max_lon}}` → `S2Response[]`; caps at 20 000 | `pages/map/markers/S2.tsx` |
| `s2Coverage` `:723` | POST (allSettled) | `POST /api/v2/s2/circle-coverage` (Radius) or `POST /api/v2/s2/cell-coverage` (S2) body `{lat,lon,radius?,size?,level}` → `string[]` cell ids | `useSyncGeojson.ts:59`, `pages/map/markers/Point.tsx` |

Notable side-channel: `Code.tsx:45` — when CodeMirror body starts with `http`, it calls `fetchWrapper<object>(url)` to fetch remote JSON into the editor.

## (2) Shared UI components — PORT vs DROP

`services/utils.ts` (pure, no JSX) — **PORT WHOLESALE** if backend contract unchanged; many funcs are v1-era. Key: `getMapBounds`, `getColor`/`getPolygonColor`/`getPointColor`/`getDataPointColor` (seedrandom cache `:314`), `fromCamelCase`/`fromSnakeCase`, `safeParse`, `combineByProperty` (turf union, `:97`), `splitMultiPolygons` (`:137`), `removeThisPolygon`/`removeAllOthers` (turf point-in-poly + mutate `useShapes`), `mpToPoints`, `getRouteType`/`getCategory` (KojiModes↔Category mapping, v1 12-mode assumption), `collectionToObject`/`filterImports`, `buildShortcutKey`, `getKey`.

| Component (`file:line`) | Verdict | Reason / used-by |
|---|---|---|
| `components/Code.tsx` (codemirror wrapper) `:20` | **PORT** | Only JSON editor in app. `@uiw/react-codemirror` + `@codemirror/lang-json` + `lint`. Dark-mode keyed remount, remote-URL fetch hook. Used: `drawer/Geojson.tsx`, `drawer/manage/index.tsx`, all 3 ImportExport/Manager/Convert dialogs, `dialogs/import/ImportWizard.tsx` |
| `components/ReactWindow.tsx` (react-window) `:4` | **PORT** (but thin) | Generic `FixedSizeList<T,U>` wrapper, `itemData={{rows,...data}}`. **Only ONE consumer**: `dialogs/import/AssignStep.tsx`. Trivial to re-derive; port if keeping AssignStep |
| `notifications/Base.tsx` `:22` | **PORT** | Toast shell: `Collapse`+`Stack`+`Alert`, auto-dismiss via `useAlertTimer`, close→clears `useStatic.notification`. Foundation for General/NetworkStatus |
| `notifications/General.tsx` `:7` | PORT | Reads `useStatic.notification`, title-maps severity. Used `pages/map/index.tsx` |
| `notifications/NetworkStatus.tsx` `:7` | **DROP/merge** | Near-identical to General; only differs in title (HTTP-status map). Used `pages/admin/index.tsx`+`pages/map/index.tsx`. Collapse into one severity-aware Notification |
| `buttons/Download.tsx` `:4` | PORT | Pure client blob download, no backend. Used `dialogs/ImportExport.tsx`, `dialogs/import/Finish.tsx` |
| `buttons/SplitMultiPolygons.tsx` `:11` | PORT | Pure `splitMultiPolygons(fc)`. Used Manager, ImportExport, ImportWizard |
| `buttons/CombineByName.tsx` `:13` | PORT | Pure `combineByProperty`. Single consumer: ImportWizard |
| `buttons/SaveToKoji.tsx` `:11` | **PORT w/ rework** | Wraps `save('geofences')`→`getKojiCache`→`save('routes')` relink-by-name. v2 dup-create risk. Used Manager, import/Finish |
| `buttons/SaveToGolbat.tsx` `:9` | **DROP-candidate** | `TODO(v2-verify)`: v1 `save-golbat` has no v2 equiv; now just saves Kōji fences + clears stubbed golbat cache. Used Manager, `popups/Polygon.tsx`. Re-evaluate vs admin publish action |
| `dialogs/Base.tsx` `:26` | PORT | Generic dialog shell (Header+Content+Actions, theme-aware). Used Keyboard, Route, Convert, ImportWizard |
| `dialogs/Header.tsx` `:10` | PORT | DialogTitle + clear-X. Foundation for Base/Manager/ImportExport |
| `dialogs/Convert.tsx` `:25` | PORT | Conversion playground; `convert()` + live map preview (`Map`+`GeoJsonWrapper`). Used `drawer/manage/index.tsx`, `pages/Convert.tsx` |
| `dialogs/Manager.tsx` `:16` | PORT | Fullscreen geojson editor (Code + Split/SaveKoji/SaveGolbat). Used `drawer/manage/index.tsx` |
| `dialogs/ImportExport.tsx` `:20` | PORT | Shared Import/Export shell for Polygon+Route (Code + format select + clipboard/download). Backs Polygon.tsx & Route.tsx |
| `dialogs/Polygon.tsx` / `dialogs/Route.tsx` | PORT | Thin wrappers binding `useImportExport.open` to ImportExportDialog. Route adds stats panel |
| `dialogs/Keyboard.tsx` `:13` | PORT (optional feature) | Keybind editor over `usePersist.kbShortcuts` + `KEYBOARD_SHORTCUTS` const. Drop if shortcuts feature cut |
| `styled/Drawer.tsx` `:29` | PORT-or-replace | MUI mini-variant drawer open/closed mixins. Used `drawer/index.tsx`, `pages/map/index.tsx` |
| `styled/Main.tsx` `:3` | PORT-or-replace | `<main>` margin-shift for drawer. `pages/map/index.tsx` |
| `styled/DrawerHeader.tsx` `:21` | PORT | Header w/ ThemeToggle + close. `drawer/index.tsx` |
| `styled/Subheader.tsx` `:3` | PORT (trivial) | Styled `ListSubheader`. Widely used across `drawer/*` + Playground |
| `styled/Slide.tsx` `:5` | PORT (trivial) | `forwardRef` right-Slide transition. `drawer/index.tsx`, `drawer/MiniItem.tsx` |
| `styled/LinkBehavior.tsx` `:4` | PORT | react-router `Link` adapter for MUI; wired into theme (`assets/theme.ts:7`) |

Styled `*.tsx` are all MUI-`styled` glue. If the refactor target drops MUI, all of `styled/` + every dialog/notification/button gets rewritten — they are MUI-coupled, not framework-neutral. Only `services/utils.ts` and the two zustand stores are genuinely portable as-is.

## (3) Core domain TS types (`assets/types.ts` + `assets/constants.ts`)

**GeoJSON layer (branded over `geojson` base):**
- `Properties<G>` `types.ts:50` — `GeoJsonProperties &` Kōji `__`-prefixed fields: `__leafletId,__forward,__backward,__start,__end,__multipoint_id,__name,__id,__geofence_id,__parent,__mode:KojiModes,__projects,__cells,__index`. This `__`-namespace is the spine of shape identity across `useShapes`.
- `Feature<G,P=Properties>` `:68` — base Feature with a **discriminated `id`**: Point→`number`, LineString→`` `${number}__${number}` ``, else→`KojiKey|string`.
- `FeatureCollection<G,P>` `:77`; `GeometryTypes` `:84` (geojson types minus Feature/FC/GeometryCollection).

**Kōji identity:**
- `KojiModes` `:99` = `KojiFenceModes | KojiRouteModes | 'unset'`, built from consts `RDM_FENCES`/`RDM_ROUTES`/`UNOWN_FENCES`/`UNOWN_ROUTES` (`constants.ts:93-114`). NOTE v1-era 12-mode set; v2 collapses to 4 (`pokemon`/`fort`/`quest`/`unset`) per `fetches.ts:127` TODO.
- `KojiSource` `:101` = `'KOJI'|'GOLBAT'|'CLIENT'`.
- `KojiKey` `:103` = `` `${number}__${KojiModes}__${KojiSource}` `` — the cross-store record key. Parsed by `useDbCache.parseKojiKey` (`useDbCache.ts:101`).

**DB entities:** `BasicKojiEntry` `:105` (id/name/created_at/updated_at). `KojiGeofence` `:112`, `KojiProperty` `:119`, `KojiGeoProperty` `:124`, `KojiProject` `:131` (has `golbat:boolean`), `KojiRoute` `:138` (geometry:MultiPoint, points), `KojiTileServer` `:146`. Admin variants: `AdminGeofence` `:150` (+properties/projects/routes), `AdminProject` `:156` (+geofences).
- `DbOption` `:250` — the unified cache option: `{id,name,mode:KojiModes,geo_type?,geofence_id?,geofences?,projects?}`. The shape stored in all `useDbCache` records.

**Calc/stats + v2 envelope:**
- `KojiStats` `:160` — `best_clusters,best_cluster_point_count,cluster_time,route_time,total_points,points_covered,total_clusters,total_distance,longest_distance,fetch_time,mygod_score`.
- `KojiResponse<T>` `:174` — **v1 legacy** envelope (data/status_code/status/message/stats). Superseded by:
- v2 envelope: `ApiMeta` `:190`, `ApiOk<T>` `:200`, `ApiErr` `:207`, `ApiEnvelope<T>` `:212`. `JobStatus` `:215` (`queued|running|succeeded|failed|canceled`). `CalcJobResult` `:227` (`{data:FC|null,stats:KojiStats}`). `JobRecord` `:236` (`id,kind,status,progress,phase?,result?,error?`).

**Markers / misc:** `PixiMarker` `:272` (`i:` branded `` `${'p'|'g'|'v'|'u'|'r'|'s'}${number}` ``, `p:[lat,lon]`), `Data` `:265` (gyms/pokestops/spawnpoints/stations), `Category` `:292` (`pokestop|gym|spawnpoint|station`), `Config` `:279` (start coords, plugins lists), `S2Response` `:294` (`{id,coords}`), `CombinedState` `:290` (Partial UsePersist+UseStatic), `TabOption` `:263`.

**Conversion types:** `ObjectInput`/`MultiObjectInput`/`ArrayInput`/`MultiArrayInput` `:303-307`, `Poracle` `:309`, `Conversions` union `:322` (the `convert()` input/output universe), `ConversionOptions` `:336` (=`CONVERSION_TYPES[number]`). `PopupProps` `:341`.

**Utility types** `:29-44`: `SpecificValueType`, `OnlyType<T,U,V>`, `StoreNoFn<T>` (`keyof OnlyType<T,Function,false>` — used by stores to type their `setX` key params).

**Const-derived enums (`constants.ts`):** `TABS:75`, `RDM_FENCES:93`/`RDM_ROUTES:100`/`UNOWN_FENCES:107`/`UNOWN_ROUTES:109`/`ALL_FENCES:116`/`ALL_ROUTES:118`, `CONVERSION_TYPES:120`, `GEOMETRY_CONVERSION_TYPES:136`, `PROPERTY_CATEGORIES:143`, `S2_CELL_LEVELS:153`, `BOOTSTRAP_LEVELS:158`, `KEYBOARD_SHORTCUTS:160`, `MODES:197`(cluster/bootstrap), `CATEGORIES:199`, `TTH:201`, `CALC_MODE:203`(Radius/S2), `CLUSTERING_MODES:205`, `SORT_BY:213`. Plus icon maps `ICON_SVG/ICON_RADIUS/ICON_COLOR:31-73`, `VECTOR_COLORS:189`, `ATTRIBUTION:22`.

## Zustand stores (definitions + shape)

Five `create()` stores, all flat with a generic setter. Cross-store reads via `.getState()` are pervasive (esp. in `fetches.ts`, `utils.ts`).

- **`usePersist`** (`usePersist.ts:94`) — `persist` middleware, localStorage key `'local'`, `partialize` strips `export` (`:173`). The big settings/clustering store: `interface UsePersist:15` ~70 fields across Drawing / Client Settings / Layers / Clustering / Dev groups. Clustering fields are the v2 calc inputs: `mode,category,cluster_mode,radius,min_points,max_clusters,calculation_mode,s2_level,s2_size,sort_by,center_clusters,genetic_post_processing,save_to_db,save_to_golbat,routing_args,clustering_args,bootstrapping_args,tth,last_seen,data`. Setter: `setStore(key,value)` `:91`. (`fast` field `:71` is dead — dropped at `fetches.ts:381`.)
- **`useStatic`** (`useStatic.ts:84`) — non-persisted runtime/UI state: `notification`(message/status/severity), `bounds`, `loading`/`loadingAbort` (per-instance KojiStats|AbortController), `geojson:FeatureCollection`, `selected`, `*_plugins`, `tileServers`, `koji/golbatRoutes`, `dialogs`(convert/manager/keyboard), `importWizard`(full wizard state `:58`), `projects`, `clickedLocation`, `layerEditing`, `forceRedraw/forceFetch`, `combinePolyMode`, `dangerous`. Setters: `setStatic(key,init|fn)` `:74`, `setGeojson(fc,noSet?)` `:78` (merges via `collectionToObject`). `CacheKey=StoreNoFn<UseStatic>` `:14`.
- **`useShapes`** (`useShapes.ts:82`) — the editable-geometry engine. Per-type record maps `Point/MultiPoint/LineString/MultiLineString/Polygon/MultiPolygon/GeometryCollection` keyed by id; `activeRoute`, `firstPoint`/`lastPoint`, `newPoints`, `s2cellCoverage`, `combined`. `getters` `:42` (getGeojson/getPointsAsMp/getFirst/getLast/getNewPointId). `setters` `:49` (add/remove/update/updateProperty/combine/splitLine/activeRoute/setFromCollection) — heavy point↔line↔multipoint linked-list maintenance via `__forward/__backward/__start/__end`. Calls `useDbCache.setRecord` on sourced adds (`:182`). Setter `setShapes` `:74`.
- **`useDbCache`** (`useDbCache.ts:49`) — metadata cache: `route/project/geofence:Record<id,DbOption>`, `golbat:Record<KojiKey,DbOption>`, `feature:Record<KojiKey,Feature>`. Getters: `getOptions` `:56` (builds KojiKey-keyed map), `getRouteByCategory` `:70` (matches `mode.includes('raid'|'quest'|'station'|'pokemon')` — **v2 4-mode collapse breaks this**, `fetches.ts:127`), `parseKojiKey` `:101`, `getFromKojiKey` `:113`, `getRecord(s)`. Setters: `setRecord(s)`, `setDbCache`.
- **`useRaStore`** (`useRaStore.ts:14`) — tiny react-admin UI-flags store: `bulkAssignProject/Geofence/Parent`, `geofenceCreateDialog`, `setRaStore`. Admin-only.

Sixth store-like module: **`useImportExport`** (`hooks/useImportExport.ts:93`) — a zustand store but behaviorally a controller for the import/export dialogs: `code,error,open,feature,skipSend,fileName,stats` + `importConvert`/`exportConvert`/`fireConvert`/`updateStats`/`setCode`/`reset`. `updateStats` (`:160`) fires a v2 `routeStats` job (`POST /api/v2/jobs` mode:'routeStats', `:249`) — `TODO(v2-verify):241` flags that v2 StatsReq has no `area` field so coverage/score may be 0.

## Other hooks (non-store)
- `useAlertTimer` `:4` — 5s auto-dismiss of `useStatic.notification`, pauses on hover. (notifications/Base)
- `useLayers` `:5` — toggles leaflet pane `.hidden` from `usePersist.show{Circles,Lines,Polygons,Arrows}`.
- `usePixi` `:42` — leaflet-pixi-overlay marker renderer; consumes `PixiMarker[]`, `ICON_SVG`/`getDataPointColor`, geohash coloring, sprite scaling. Heavy + leaflet/pixi-coupled.
- `usePopupStyle` `:18` — imperatively repaints leaflet popup DOM to MUI theme bg on `popupopen`.
- `useSkipFirstEffect` `:8` — useEffect that skips first render.
- `useSyncGeojson` `:18` — `useDeepCompareEffect` flattening all `useShapes` maps → `useStatic.geojson`, recomputes `s2cellCoverage` via `s2Coverage`. Exposes `window.shapes`/`window.cache` in dev (`:11`).


---

# 5. Koji Backend — HTTP API Surface

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


---

# 6. shadmin Library — Component Inventory & Contracts

I have everything needed. The contract is confirmed: 100% react-admin/ra-core `DataProvider` + `AuthProvider` (no custom provider — `realtimeDataProvider` is a decorator wrapping a standard one). Compiling the dense inventory now.

---

# shadmin Library Capability Scan — Raw Inventory

## 0. Architecture / contract (load-bearing facts)

- **Distribution model:** shadcn **registry** (copy-in source), NOT a published npm package. `packages/shadmin/package.json` is `"private": true`, `version 0.1.0`, no `exports`/`main`. Consumers `shadcn add @shadmin/<item>` OR pull the monolith `admin` block. `@` alias → `packages/shadmin/src`. Granular per-file registry items auto-derived from import graph (CHANGELOG "Registry refactor"). `components.json`: style `new-york`, RSC off, base color `neutral`, icon lib lucide.
- **Headless layer:** `shadmin-core` (`packages/shadmin-core/src/index.ts:12`) is **`export * from "ra-core"` verbatim** — a deliberate seam for a future in-house replacement. All admin components import from `"shadmin-core"`, never `"ra-core"` directly (Biome `noRestrictedImports` enforced). **So today the entire data/auth/i18n/routing/state contract IS react-admin's ra-core 5.14, unchanged.**
- **DataProvider contract:** standard **ra-core `DataProvider`** (`getList/getOne/getMany/getManyReference/create/update/updateMany/delete/deleteMany`). No custom provider shipped in `src`. README + `data-providers.md` point at the 50+ ra-data-* adapters (`ra-data-simple-rest`, `ra-data-json-server`, `ra-data-fakerest` (dev), `ra-supabase-core`). The only provider *code* in src is a **decorator**: `realtimeDataProvider(base, opts)` (`realtime/realtime-data-provider.ts`) wraps any DataProvider to add subscribe/publish/lock methods; `addEventsForMutations` auto-emits events.
- **AuthProvider contract:** standard **ra-core `AuthProvider`** (`login/logout/checkAuth/checkError/getPermissions/getIdentity/canAccess`). Passed via `<Admin authProvider>` (`admin.tsx:170`). Access control via ra-core `useCanAccess` (used in `data-table.tsx:84`, bulk buttons). Optional **Supabase** auth bundle in `components/supabase/` (login/forgot/set-password pages + 18 social-auth buttons; depends on optional peer `ra-supabase-core` + `@supabase/supabase-js`).
- **i18n:** ra-core `polyglotI18nProvider` (`lib/i18n-provider.ts:4`), default `ra-language-english`, `allowMissing:true`. `<LocalesMenuButton>` + `<TranslatableInputs>`/`<TranslatableFields>` for multi-locale records. French pack available as dev dep.
- **Routing:** **React Router v7** (`react-router@^7.12`), via ra-core. `<CustomRoutes>`/`<Route>`, `useNavigate`. **NOT TanStack Router** (TanStack Query is used for data caching only, via ra-core). Docs include TanStack *Start* + React Router setup tutorials.
- **Theming:** **Tailwind CSS v4** + shadcn `new-york`, oklch CSS custom properties, light/dark via `.dark` class on `<html>`. `ThemeProvider` (`layout/theme-provider.tsx`) manages **mode only** (light/dark/system) persisted through ra-core `useStore`; **palettes are pure CSS** now (`src/styles/themes/{aurora,bw,house,nano,radiant}.css` + registry `cssVars`) — the old JS theme objects (`bwTheme`, `defaultTheme`, etc.) were **removed/moved** (CHANGELOG BREAKING; `useThemes`/`AdminTheme` deleted from admin barrel). Extra `glass.css`/`aurora.css` + `<Glass>` liquid-glass primitives.
- **Forms:** React Hook Form + Zod. The admin form-field fork was **deleted** in favor of shadcn `ui/field`; `admin/index.ts:61` re-exports RHF `FormProvider as Form`.
- **Primitive seam:** `components/ui/*` wraps radix-ui; namespaces exposed (`PopoverPrimitive` etc.) so radix↔base-ui swap is a `ui/`-only edit. Biome rule bans `radix-ui`/`@base-ui/react` imports outside `ui/`.
- **Layering:** `extras → realtime → admin → ui`. `admin/` must not import `realtime/` or `extras/`. (`AGENTS.md:49-63`).
- **`temp/` + `temp-rich-text-input/`:** **gitignored scratch dirs** (registry install-test harnesses, consume shadcn MCP). NOT library source — ignore for inventory.
- **`examples/`:** single file `examples/example-admin.tsx` (demo wiring). `src/test/_test-helpers.tsx` = shared `StoryAdmin` wrapper.

---

## 1. COMPONENT INVENTORY (grouped; ready = shipped+exported+tested/storied; partial = experimental/caveat; missing = absent)

### Root / app shell — `components/admin/`
| Component | Status | File |
|---|---|---|
| `<Admin>` / `<AdminContext>` / `<AdminUI>` | ready | `admin/admin.tsx` |
| `<Resource>` | ready | `admin/resource.tsx` |
| `CustomRoutes` (from ra-core) | ready | re-export |

### Views (CRUD) — `admin/views/` + `guessers/`
| Component | Status | File |
|---|---|---|
| `<List>` / `<InfiniteList>` | ready | `list/list.tsx`, `list/infinite-list.tsx` |
| `<Create>` `<Edit>` `<Show>` | ready | `views/create.tsx`, `edit.tsx`, `show.tsx` |
| `<SimpleShowLayout>` `<TabbedShowLayout>` | ready | `views/simple-show-layout.tsx`, `tabbed-show-layout.tsx` |
| `<Labeled>` `<CardContentInner>` | ready | `views/labeled.tsx`, `card-content-inner.tsx` |
| `<ListGuesser>` `<EditGuesser>` `<ShowGuesser>` | ready | `guessers/*` |
| `<TranslatableFields>` (+ tab/tabs/tab-content) | ready | `views/translatable-fields*` |

### Data table / list internals — `admin/list/`
| Component | Status | File |
|---|---|---|
| `<DataTable>` + `.Col` + `.NumberCol` + Head/Body/Row/Cell/Empty/Loading, SelectPage/RowCheckbox | ready | `list/data-table.tsx` (full sort, bulk-select, column reorder/hide via store, row expand, rowClick, density) |
| `<SimpleList>` (+ item, loading) | ready | `list/simple-list*.tsx` |
| `<SingleFieldList>` | ready | `list/single-field-list.tsx` |
| `<ListPagination>` `<InfinitePagination>` `<PrevNextButtons>` | ready | `list/*pagination*`, `prev-next-buttons.tsx` |
| `<Count>` `<ReferenceManyCount>` | ready | `list/count.tsx`, `reference-many-count.tsx` |
| `<FilterForm>` `<FilterList>` (+item/section) `<FilterLiveSearch>` | ready | `list/filter-*.tsx` |
| `<BulkActionsToolbar>` | ready | `list/bulk-actions-toolbar.tsx` |
| `<ListActions>` `<ListToolbar>` `<ListNoResults>` | ready | `list/*` |
| `<SavedQueries>` | ready | `layout/saved-queries.tsx` |
| **DataGrid (legacy react-admin Datagrid)** | n/a | superseded by DataTable (no `<Datagrid>` — only `<DatagridInput>`) |

### Form layouts — `admin/form/`
| Component | Status | File |
|---|---|---|
| `<SimpleForm>` + `<FormToolbar>` | ready | `form/simple-form.tsx` |
| `<SimpleFormConfigurable>` | ready | `form/simple-form-configurable.tsx` |
| `<TabbedForm>` | ready | `form/tabbed-form.tsx` |
| `<SimpleFormIterator>` (array sub-form) | ready | `form/simple-form-iterator.tsx` |
| `<Toolbar>` | ready | `form/toolbar.tsx` |
| `<TranslatableInputs>` (+tab/tabs/content) | ready | `form/translatable-inputs*` |
| `Form` (RHF FormProvider) | ready | re-export `admin/index.ts:61` |

### Inputs — `admin/inputs/`
| Input | Status | File |
|---|---|---|
| Text / Password / ResettableText / Search | ready | `text-input`, `password-input`, `resettable-text-input`, `search-input` |
| Number | ready | `number-input.tsx` |
| Select / SelectArray | ready | `select-input.tsx` (FIXME radix issue #3135 noted), `select-array-input.tsx` |
| Autocomplete / AutocompleteArray | ready | `autocomplete-input.tsx`, `autocomplete-array-input.tsx` |
| Boolean / NullableBoolean | ready | `boolean-input.tsx`, `nullable-boolean-input.tsx` |
| CheckboxGroup / RadioButtonGroup | ready | `checkbox-group-input.tsx`, `radio-button-group-input.tsx` |
| Date / DateTime / Time | ready | `date-input.tsx`, `date-time-input.tsx` (TODO react-compiler note), `time-input.tsx` |
| Array (`<ArrayInput>`) / TextArray | ready | `array-input.tsx`, `text-array-input.tsx` |
| File / Image | ready | `file-input.tsx`, `image-input.tsx` (react-dropzone) |
| Reference / ReferenceArray | ready | `reference-input.tsx`, `reference-array-input.tsx` |
| `<DatagridInput>` | **partial** | `datagrid-input.tsx:41` `@experimental` — "upstream WIP, simplified port" |
| `<LoadingInput>` | ready | `loading-input.tsx` |
| **RichText (input)** | ready | `components/rich-text-input/rich-text-input.tsx` (TipTap "minimal-tiptap"; NOT under admin/) |
| **Color / Currency / Phone / Cron / Duration / Rating / ApiKey / Webhook** | ready (extras) | `extras/*-input.tsx` |
| **JSON (Monaco)** | ready (monaco) | `monaco/monaco-json-input.tsx` (+lazy) |
| **MDX** | ready (mdx) | `mdx-editor/mdx-input.tsx` |
| **Block editor (Notion-style)** | ready (block-editor) | `block-editor/block-editor-input.tsx` |

### Fields — `admin/fields/`
| Field | Status | File |
|---|---|---|
| Text / Number / Boolean / Email / Url / Date | ready | `*-field.tsx` |
| Select / Badge / Chip | ready | `select-field`, `badge-field`, `chip-field` |
| File / Image | ready | `file-field.tsx`, `image-field.tsx` |
| Array / TextArray | ready | `array-field.tsx`, `text-array-field.tsx` |
| Function / Record / Wrapper | ready | `function-field`, `record-field` (FIXME TS<5.4), `wrapper-field` |
| Reference / ReferenceArray / ReferenceMany / ReferenceOne | ready | `reference-*-field.tsx` (FIXME ts-expect-error on total in 2 files) |
| RichText (display) | ready | `rich-text-field.tsx` (dompurify+html-react-parser) |
| **Color / Currency / Phone / Cron / Duration / Rating / ApiKey / Webhook / UsageMeter / SubscriptionPlan** | ready (extras) | `extras/*-field.tsx` |
| **JSON (Monaco/read)** | ready (monaco) | `monaco/json-field.tsx`, `monaco-json-field.tsx` |
| **MDX (display)** | ready (mdx) | `mdx-editor/mdx-field.tsx` |
| **BlockDoc** | ready (block-editor) | `block-editor/block-doc-field.tsx` |

### Layout — `admin/layout/`
| Component | Status | File |
|---|---|---|
| `<Layout>` | ready | `layout/layout.tsx` |
| `<AppBar>` `<AppSidebar>` | ready | `layout/app-bar.tsx`, `app-sidebar.tsx` |
| `<Menu>` `<MenuItemLink>` `<ResourceMenuItem>` `<ResourceMenuItemGroup>` `<DashboardMenuItem>` | ready | `layout/menu*`, `resource-menu-item*`, `dashboard-menu-item.tsx` |
| `<UserMenu>` | ready | `layout/user-menu.tsx` |
| `<Breadcrumb>` | ready | `layout/breadcrumb.tsx` |
| `<Title>` `<TitlePortal>` | ready | `layout/title.tsx`, `title-portal.tsx` |
| `<TopToolbar>` | ready | `layout/top-toolbar.tsx` |
| `<ThemeModeToggle>` `<ThemeProvider>` | ready | `layout/theme-mode-toggle.tsx`, `theme-provider.tsx` |
| `<SidebarToggleButton>` `<HideOnScroll>` | ready | `buttons/sidebar-toggle-button.tsx`, `layout/hide-on-scroll.tsx` |

### Buttons / actions — `admin/buttons/`
| Button | Status |
|---|---|
| Create / Edit / Show / Clone / Delete / List | ready (`*-button.tsx`) |
| Save / Cancel / Refresh / RefreshIcon | ready |
| Export / BulkExport / BulkDelete / BulkUpdate / Update | ready |
| SelectAll / Columns / Filter / ToggleFilter / Sort | ready |
| Inspector / LocalesMenu / SkipNavigation | ready |

### Feedback / notifications — `admin/feedback/`
| Component | Status |
|---|---|
| `<Notification>` (sonner) / `<Confirm>` | ready (`notification.tsx`, `confirm.tsx`) |
| `<Loading>` `<LoadingIndicator>` `<LinearProgress>` `<Spinner>` | ready |
| `<Error>` `<NotFound>` `<Empty>` `<Placeholder>` `<Ready>` `<Offline>` | ready |

### Auth — `admin/auth/`
| Component | Status |
|---|---|
| `<LoginPage>` `<LoginForm>` `<LoginWithEmail>` `<Logout>` | ready |
| `<AuthLayout>` `<AuthCallback>` `<AuthError>` `<AuthenticationError>` `<AccessDenied>` | ready |
| **Supabase variants** (login/forgot/set-password + 18 social buttons + guessers) | ready (opt-in, `components/supabase/`) |

### Inspector / configurable — `admin/inspector/`
`<Inspector>` `<InspectorRoot>` `<Configurable>` `<FieldsSelector>` `<FieldToggle>` — ready.

### Realtime — `components/realtime/` (ALL ready; net-new vs stock react-admin OSS)
`realtimeDataProvider`, `addEventsForMutations`, 4 transports (`webSocketTransport`/`sseTransport`/`broadcastChannelTransport`/`fakeTransport`), `inMemoryLockProvider`, 18 hooks (`useSubscribe*`, `usePublish`, `useGetListLive`/`useGetOneLive`/`useGetManyLive`, `useLock`/`useUnlock`/`useGetLock(s)(Live)`, `useLockOnMount`, `useRealtimeStatus`, `useOnReconnect`), 6 components (`<ListLive>` `<EditLive>` `<ShowLive>` `<MenuLive>`+`<MenuLiveItemLink>` `<LockOnMount>` `<LockStatus>`).

### Geo / mapping — `components/leaflet/` (HIGH RELEVANCE TO KOJI; all ready unless noted)
| Capability | Status | File |
|---|---|---|
| `<SharedMap>`/`BaseMap` (react-leaflet MapContainer+TileLayer, OSM default tiles) | ready | `leaflet/shared-map.tsx` |
| `<LatLngField>` / `<LatLngInput>` | ready | `lat-lng-field.tsx`, `lat-lng-input.tsx` |
| Shape fields+inputs: Point, MultiPoint, LineString, MultiLineString, Polygon, MultiPolygon, GeometryCollection, BBox | ready | `leaflet/shapes/*` (+ `shape-field-shell`, `shape-input-shell`) |
| `<GeoJsonField>` / `<GeoJsonInput>` | ready | `geojson-field.tsx`, `geojson-input.tsx` |
| `<FeatureField>`/`<FeatureInput>` / `<FeatureCollectionField>`/`<FeatureCollectionInput>` | ready | `feature*.tsx` |
| `<SimplifyInput>` (turf/simplify) | ready | `simplify-input.tsx` |
| Geoman drawing/edit RHF bridge | ready | `geoman/use-geoman-rhf.ts`, `geoman-shape-mapping.ts`, `shape-constraints.ts` |
| OSM/Overpass: feature add/subtract/operator, presets, tag catalog, snap-to-roads | ready (partial polish) | `osm/*` (`use-geoman-rhf.ts:240` notes a dedup hack) |
| Geocoding (Nominatim) + reverse geocode + map-with-search | ready | `geocoding/*` (`nominatim-client.ts`, `use-geocode.ts`, `use-reverse-geocode.ts`, `geocoding-input.tsx`, `reverse-geocode-field.tsx`, `map-with-search.tsx`) |
| Turf ops (area, bbox, buffer, difference, union, simplify) | ready (deps) | via `@turf/*` |

### Extras — `components/extras/` (all ready; "premium-feature" tier)
Inputs/fields listed above PLUS: `<CommandMenu>` (cmd+K), `<Assistant>`+`assistantTransport`, `<ApprovalQueue>`, `<DualApprovalButton>`, `<StatusTransitionButton>`, `<BulkEditDrawer>`, `<CalendarList>`, `<CommentsThread>`, `<DashboardCharts>` (recharts), `<DataProviderDevtools>`, `<DiffViewer>`, `<I18nKeyEditor>`, `<InPlaceEditor>`, `<JobMonitor>`, `<KanbanBoard>` (dnd-kit), `<LayoutBuilder>`, `<OnboardingTour>`, `<PermissionMatrix>`, `<PivotGrid>`, `<PresenceBar>`, `<RecordTimeline>`, `<SchemaDrivenView>`, `<ThemeStudio>`, `<TreeList>`, `<WizardForm>`, `<FilterLiveForm>`.

### Block editor / CSV / Monaco / MDX (specialized)
- `components/block-editor/`: TipTap block editor (`defaultBlocks`: callout/toggle/image/embed; `dataBlocks`: referenceRecord/recordList/chart), `defineBlock`, `block-registry`, `<BlockEditorInput>`, `<BlockDocField>` — ready.
- `components/csv-import/`: `<CsvImport>` + `useCsvImport` (papaparse) — ready. **Covers bulk import gap.**
- `components/monaco/`: JSON input/field with schema validation + lazy variants — ready.
- `components/mdx-editor/`: `@mdxeditor/editor` input/field — ready.

### shadcn/ui primitives — `components/ui/` (59 files)
accordion, alert(-dialog), aspect-ratio, avatar, badge, breadcrumb, button(-group), calendar, card, carousel, chart, checkbox, collapsible, color-picker, command, context-menu, dialog, direction, drawer, dropdown-menu, empty, **field**, glass(+filter), hover-card, input(-group/-otp), item, kbd, label, menubar, native-select, navigation-menu, pagination, popover, progress, radio-group, resizable, scroll-area, select, separator, sheet, sidebar, skeleton, slider, slot, sonner, spinner, switch, table, tabs, textarea, toggle(-group), tooltip — all ready.

### Hooks / lib (`src/hooks`, `src/lib`)
Hooks: `useMobile`, `useTheme`, `useGlassLens`, `useGlassPointer`. Lib: `cn`/utils, `i18nProvider`, `field-types` (`FieldProps`), `resolveLabel`, `sanitizeInputRestProps`, `areIdsEqual`, `notifyAuthError`, `theme-context`, `title-portal-id`, glass helpers. (Plus all ra-core hooks via `shadmin-core`: `useListContext`, `useRecordContext`, `useInput`, `useGetList`, `useDataProvider`, `useCanAccess`, `useStore`, etc.)

---

## 2. DataProvider / AuthProvider contract (exact)

**It IS the react-admin/ra-core 5.14 contract — unchanged.** Build against react-admin docs directly.

- **DataProvider** — pass any object implementing ra-core `DataProvider` to `<Admin dataProvider>`. Methods: `getList(resource,{pagination,sort,filter,meta})`, `getOne`, `getMany`, `getManyReference`, `create`, `update`, `updateMany`, `delete`, `deleteMany`. Pick/write an `ra-data-*` adapter for Koji's REST/JSON-server API (`ra-data-json-server` or `ra-data-simple-rest` are the closest matches and are already dev-deps). To get live updates, wrap it: `realtimeDataProvider(base, { transport, lockProvider })`.
- **AuthProvider** — ra-core `AuthProvider`: `login`, `logout`, `checkAuth`, `checkError`, `getIdentity`, `getPermissions`, `canAccess({resource,action,record})`. Passed via `<Admin authProvider>`. Access control is wired throughout (DataTable bulk-delete gated on `useCanAccess` delete). Supabase impl available if Koji ever uses Supabase; otherwise hand-write a ~40-line provider hitting Koji's auth endpoint.
- **i18nProvider** — optional; defaults to English polyglot. Override via `<Admin i18nProvider>`.
- **store** — defaults to `localStorageStore()` (`admin.tsx:34`); drives column visibility, saved queries, theme mode.

---

## 3. GAPS vs what a Koji react-admin app needs

**What's MISSING / requires work:**
1. **No Koji DataProvider.** Library ships zero concrete providers (only the realtime decorator + dev fakerest). Koji must author/select an `ra-data-*` adapter mapping its API (filter/sort/pagination param encoding, total-count header). This is the #1 build item.
2. **No Koji AuthProvider** unless using Supabase. Hand-write against Koji's auth backend.
3. **Not an npm dependency.** No `exports`/built artifact — you **copy source via the shadcn registry** (`@shadmin/<item>`) into Koji's tree under the `@/` alias, then own/maintain it. Requires Tailwind v4 + shadcn `new-york` + the `ui/` primitives + CSS theme tokens to be set up in Koji's app. Not a drop-in `import from "shadmin"`.
4. **Geo stack is Leaflet/OSM/Turf/Geoman, not Koji-native.** Strong fit for a geofencing admin, BUT: tiles default to OSM, geocoding is Nominatim (Koji already vendors its own Nominatim fork per memory — reconcile), and OSM/Overpass helpers have a noted dedup hack (`use-geoman-rhf.ts:240`). No S2-cell / honeycomb / clustering primitives — Koji's geometry domain (S2, route TSP, clustering) has **no UI here**; only generic GeoJSON/shape editing.
5. **`shadmin-core` is a thin alias, not a real boundary yet.** Anything you rely on is really ra-core 5.14 — track react-admin's roadmap/breaking changes; the "in-house type-safe replacement" is aspirational/empty today.
6. **`<DatagridInput>` is experimental** (`datagrid-input.tsx:41` WIP). Avoid for production embeds; prefer `<ReferenceArrayInput>`+`<AutocompleteArrayInput>`.
7. **No tree/nested-resource routing helper beyond `<TreeList>` (extras)**; no built-in multi-tenant/project-scoping. Koji's project/area hierarchy needs custom wiring.
8. **Private/0.1.0 + Unreleased churn:** realtime + the granular-registry + native-CSS-theming refactor are all in `[Unreleased]` (CHANGELOG) — API not frozen; the theme-JS→CSS move was a documented BREAKING change. Pin a commit.
9. **Build/test cost:** tests are Vitest + **Playwright browser provider** (real Chromium) — heavier CI than jsdom.

**What's already covered (no gap):** full CRUD + guessers, DataTable (sort/filter/bulk/columns/export/expand/pagination), every standard input incl. Reference/Autocomplete/Array/Date/File, RichText (TipTap), JSON (Monaco), Color, MDX, block editor; layout/menu/appbar/sidebar/breadcrumb/user-menu; auth pages + access control; i18n + translatable inputs/fields; light/dark + 5 CSS palettes + liquid-glass; notifications (sonner); CSV import; realtime/live/locks; dashboards/charts (recharts); cmd+K palette; **and a complete Leaflet geo input/field suite** (GeoJSON, all geometry types, bbox, feature collections, geocoding, OSM editing) that is directly reusable for Koji's map-editing surfaces.

**Key file refs:** entry `packages/shadmin/src/components/admin/admin.tsx:144`; public barrel `packages/shadmin/src/components/admin/index.ts`; seam `packages/shadmin-core/src/index.ts:12`; DataTable `packages/shadmin/src/components/admin/list/data-table.tsx:142`; geo barrel `packages/shadmin/src/components/leaflet/index.ts`; realtime barrel `packages/shadmin/src/components/realtime/index.ts`; i18n `packages/shadmin/src/lib/i18n-provider.ts:4`; theming `packages/shadmin/src/components/admin/layout/theme-provider.tsx` + `packages/shadmin/src/styles/themes/*.css`.


---

# 7. shadmin Demo — Patterns to Copy

I have exhaustive coverage. Producing the dense refactor-trace notes.

---

# shadcn-admin-kit ("shadmin") — pattern harvest for Koji admin port

## 0. Monorepo shape & stack identity
- pnpm + turbo monorepo. Root `package.json:7` `dev` = `turbo run dev --filter=shadmin-demo`. Lockfile `pnpm-lock.yaml` (NOT bun here).
- Workspaces: `apps/{demo,docs,website}` + `packages/{shadmin, shadmin-core}`.
  - `packages/shadmin` = the component library (admin shell, fields, inputs, leaflet, realtime, charts, etc.). Imported as bare `shadmin/components/...` and `shadmin/hooks/...`, `shadmin/lib/utils` from demo.
  - `packages/shadmin-core` = headless logic (re-exports `ra-core` primitives like `useTranslate`, `useNotify`, `useRecordContext`; library code imports from `shadmin-core`, app code imports the same names from `ra-core`).
- **Foundation = react-admin's `ra-core` v5.14** (`Admin`, `Resource`, `ResourceProps`, `CustomRoutes`, `useRecordContext`, `useGetList`, `useListContext`, `required`, `RecordContextProvider`, `Translate`, `useTranslate`, `useNotify`). UI layer is shadcn/ui (Radix + Tailwind v4) — `shadmin/components/admin` wraps `ra-core` headless hooks in shadcn presentation. Forms use `react-hook-form` directly (`useFormContext`, `useWatch`, `form.setValue`).
- Library is shadcn-registry-distributed: `packages/shadmin/registry.json` (189KB) + `dist/r/leaflet-admin.json` — components are `shadcn add`-able, hence every file is `"use client"` + self-contained.

## 1. Resource end-to-end (canonical: `customers`, `products`, `places`)
**File layout per resource** (`apps/demo/src/<resource>/`): `index.ts(x)` (exports a `ResourceProps` object) + `<resource>-list.tsx` + `-edit.tsx` + `-create.tsx` + `-show.tsx` (subset as needed) + optional `seed.ts` + `<resource>-schema.ts`.

**`index.ts` = the registration object** (`apps/demo/src/customers/index.ts:7`):
```ts
export const customers: ResourceProps = {
  name: "customers", list: CustomerList, edit: CustomerEdit, create: CustomerCreate,
  recordRepresentation: (record) => `${record.first_name} ${record.last_name}`, // or a string field name, e.g. "name"
  icon: Users, // lucide-react
};
```
`places/index.ts:8` adds `show: MapShow`. `recordRepresentation` is either a field-name string (`map/index.ts:14` `"name"`, `products/index.tsx:12` `"reference"`) or a fn.

**Router/Admin wiring** (`apps/demo/src/app.crm.tsx:28-57`): single `<Admin>` with `dataProvider/authProvider/i18nProvider/dashboard/layout` props, children are `<Resource {...customers} group="Sales" />` — `group=` is the sidebar section label (Koji can mirror: "Map", "Planning", "Analytics"). Custom non-CRUD pages via `<CustomRoutes><Route path="/products/schema-view" element={<ProductSchemaList/>}/></CustomRoutes>` (`app.crm.tsx:53`).

**Demo multiplexer** (`app.tsx:11-42`): each "demo" is a lazily-imported `app.*.tsx` module (`app.crm`, `app.guessers`, `app.realtime`, `app.rich-text-input`, `app.supabase`) chosen by `?demo=` URL param; bottom-right `<DemoSwitcher>`. Koji-relevant: each app file = one full `<Admin>` tree, swappable data provider.

**List patterns:**
- `<List perPage sort actions pagination>` wrapping `<DataTable>` + `<DataTable.Col source render>` / `<DataTable.NumberCol options conditionalClassName>` (`customer-list.tsx:50-117`). `actions` = a flex div of `<CreateButton/><CsvImport schema/><ColumnsButton/><ExportButton/>`.
- Sidebar filters: a `<Card>` of `<FilterLiveSearch/>` + `<FilterList label icon>` → `<FilterListItem label value={{field_gte:..., field_lte:...}}/>` (`customer-list.tsx:142-236`). Products uses `<ToggleFilterButton>` variant (`product-list.tsx:140`).
- Custom cell components read `useRecordContext<T>()` and return JSX (`SegmentList`, `TypeBadge`).
- Product list is a non-table grid: `useListContext<Product>()` → `.map` into `<RecordContextProvider>` cards, with `<BulkActionsToolbar>` + `<BulkEditDrawer>` (`product-list.tsx:31-126`).
- i18n: labels are translation keys (`"resources.customers.fields.name"`) resolved via `i18nProvider`; `<Translate i18nKey=.../>`.

**Edit/Create forms** (`customer-edit.tsx`, `map-create.tsx`): `<Edit>`/`<Create redirect="show">` wrapping `<SimpleForm defaultValues>` or `<TabbedForm toolbar={<FormToolbar/>}>` with `<TabbedForm.Tab label>`. Inputs: `<TextInput source validate={required()} multiline rows>`, `<SelectInput source choices=[{id,name}]>`, `<BooleanInput>`, `<ReferenceInput source reference><AutocompleteInput/></ReferenceInput>`, plus extras `<CurrencyInput>`, `<PhoneInput defaultCountry>`, `<MonacoJsonInput>`, `<BlockEditorInput>`.

**Data provider — fake REST** (`apps/demo/src/data-provider.ts:153`): `fakeRestDataProvider(data, true, 500)` from `ra-data-fakerest` (loggingEnabled, 500ms latency). `data` = `data-generator-retail` output (`generateData()`) spread + per-resource seed arrays (`places: placesSeed`, `tasks: tasksSeed`, ...). Seeds live in each resource's `seed.ts`/`places-seed.ts` and are merged centrally (`data-provider.ts:134-151`). **Supabase is a separate opt-in app** (`app.supabase.tsx`) using `<AdminGuesser instanceUrl apiKey>` from `shadmin/components/supabase` (`ra-supabase-core` + `@supabase/supabase-js`); intentionally not wired into `main.tsx`. → **Koji: keep fakerest for demo/storybook; real backend is a swapped `dataProvider` prop, zero component changes.**

## 2. Dashboard pattern
- `apps/demo/src/dashboard/dashboard.tsx`: a plain component set as `<Admin dashboard={Dashboard}>`. Pulls data via `useGetList<Order>("orders", {filter,sort,pagination})` (`dashboard.tsx:33`), aggregates in a `useMemo` reducer, lays out with flex/`md:basis-1/2` columns.
- Composed of small card components in `dashboard/`: `monthly-revenue.tsx`, `nb-new-orders.tsx`, `pending-orders.tsx`, `pending-reviews.tsx`, `new-customers.tsx`, `welcome.tsx`, `order-chart.tsx`, `card-with-icon.tsx`, plus `<JobMonitor resource="scheduled_jobs" pollInterval={5000}>` from extras.
- **Chart lib = `recharts` v3.8.0** (`packages/shadmin/package.json`). Reusable wrappers in `packages/shadmin/src/components/extras/dashboard-charts.tsx`: `MetricCard` (label/value/delta with up/down arrow), `ChartShell` (ResponsiveContainer + Card or `bare`), `TrendChart` (Area/Line, gradient fill, `smooth`→monotone curve, tick/tooltip formatters), `BarChart`, `DonutChart` (Pie innerRadius 60%). Colors via CSS vars `var(--chart-1..5)` with hsl fallbacks (`dashboard-charts.tsx:314`). Usage: `order-chart.tsx:40` `<TrendChart data xField="date" yField="total" area smooth bare xTickFormatter yTickFormatter tooltipFormatter/>`. → **Koji dashboard: copy recharts + the `dashboard-charts.tsx` wrapper set verbatim; theme via `--chart-*` vars.**

## 3. Realtime pattern
- Lives in `packages/shadmin/src/components/realtime/`. **Transport-abstracted** — `RealtimeTransport` interface with 4 pluggable transports (`transports/`):
  - `broadcast-channel-transport.ts` (cross-tab demo, no server — used by `app.realtime.tsx:15`),
  - `websocket-transport.ts` — full WS client: JSON `{op:subscribe|unsubscribe|publish|ping}` client frames / `{topic,type,payload,meta}` server frames, exponential backoff + jitter reconnect, heartbeat ping/pong, idle-disconnect, auth via query-param or subprotocol, pending-publish queue flushed on reconnect (`websocket-transport.ts:49-403`).
  - `sse-transport.ts` — `EventSource`, topics as `?topics=` filter param, per-topic `addEventListener`, separate `publishUrl` POST for sending, same backoff/idle logic (`sse-transport.ts:27-310`).
  - `fake-transport.ts` + `in-memory-lock-provider.ts` (record locking).
- Wiring (`app.realtime.tsx:15-24`): `transport` → `realtimeDataProvider(baseDP, transport, {locks})` → `addEventsForMutations(dp, dp)`. DP-level: mutations auto-publish events.
- Hooks (`realtime/hooks/`): `useGetListLive`, `useGetOneLive`, `useSubscribe`, `useSubscribeToRecord`, `usePublish`, `useOnReconnect`, `useRealtimeStatus`, `useLock/useUnlock/useLockOnMount/useGetLockLive`. Components: `<ListLive>`, `<EditLive>`, `<ShowLive>`, `<MenuLive>`, `<LockStatus>`, `<LockOnMount>`. Usage trivial — `post-list-live.tsx:6` just swaps `<List>`→`<ListLive>`. Topic helpers `resourceTopic/recordTopic/lockTopic` (`realtime/topics.ts`).
- → **Koji golbat live updates: WS transport already written, server speaks `{topic,type,payload}` JSON frames + `subscribe/publish/ping` ops. SSE is the lower-infra alternative. Backport `websocket-transport.ts` near-verbatim.**

## 4. ⭐ MAP — the critical backport material
**Technology: Leaflet 1.9.4 + react-leaflet 5.0 + Geoman (`@geoman-io/leaflet-geoman-free` 2.19.3 via `react-leaflet-geoman-v2` 1.1.1) for editing + Turf 7.3.5 for geometry ops + osmtogeojson + Nominatim + Overpass.** Confirmed **NO maplibre / mapbox / openlayers** in source — the grep hits were only minified vendor bundles in `dist/`/`node_modules` (Leaflet's own bundle). All geo is Leaflet/OSM/free-tile.

**Package boundary:** all map code is `packages/shadmin/src/components/leaflet/` exported via `leaflet/index.ts` (42 exports), consumed by demo as `from "shadmin/components/leaflet"`. The `places` resource (`apps/demo/src/map/`) is purely a consumer — every map widget is a library component.

**Two map primitives:**
1. `shared-map.tsx` — `<BaseMap zoom defaultCenter height tileUrl attribution>`: `<MapContainer>`+`<TileLayer>` in a `rounded-md border` div, `height` as inline style, `MAP_STYLE = {height:100%,width:100%}`. Plus `<FitBoundsOnMount bounds padding maxZoom>` (uses `useMap()` + `map.fitBounds` in effect). Tile defaults in `shared.ts:12`: `https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png` + OSM attribution. Custom SVG `MarkerIcon` (L.divIcon, `hsl(217 91% 60%)` pin) at `shared.ts:4`.
2. `coordToLatLng/latLngToCoord` helpers (`shared.ts:20`) — **GeoJSON `[lng,lat]` ↔ Leaflet `[lat,lng]` swap** is centralized here (Koji bbox-order bug territory — note the explicit RFC 7946 vs Nominatim order conversion in `nominatim-client.ts:85-94`).

**Geometry editing (THE differentiator):**
- `shapes/shape-input-shell.tsx` — the editing map. Renders `<BaseMap>` → `<FeatureGroup ref>` → `<GeomanControls options globalOptions {...handlers}>`. Toolbar derived from `shape` via `geojsonTypeToGeomanShape` (`geoman-shape-mapping.ts:4`: Point→Marker, LineString→Line, Polygon→Polygon, etc.) or explicit `geomanShapes` override. Toolbar opts: draw{Marker,Line,Rectangle,Polygon,Circle}, editMode, dragMode, cutPolygon (polygons only), removalMode (`shape-input-shell.tsx:154-170`). `globalOptions`: `{snappable, snapDistance, pathOptions}`.
- `geoman/use-geoman-rhf.ts` — **the core RHF↔Geoman bridge** (293 lines, the single most important file to port). Two-way sync between a react-hook-form field (`source`) and drawn Leaflet layers:
  - Hydration effect (`:78`): builds layers from form value on mount + rebuilds on external change; dedups its own `setValue` echo via `lastWrittenValue` JSON compare; supports plain geometry, FeatureCollection mode, and `valueParse` inverse transform.
  - `persist()` (`:129`): reads `group.getLayers()`, converts via `layerToGeometry`, writes back. Modes: single (replace), `multi` (combine into Multi* via `combineMulti` `:260`), `collection` (GeometryCollection), `featureCollection` (one Feature/layer, preserves `properties` by index). `valueTransform(geom, prev)` lets BBoxInput store `[w,s,e,n]` and FeatureInput preserve `Feature.properties`.
  - Geoman event handlers: `onCreate` (single-shape removes other layers so new draw replaces), `onUpdate`, `onLayerRemove`, `onMapCut`/`onLayerCut` (map-level persists, layer-level ignored to avoid double-fire).
- `geoman/geoman-shape-mapping.ts` — `layerToGeometry` (`:17`: `L.Circle`→64-vertex polygon ring via equirectangular `circleToPolygon`, else `.toGeoJSON().geometry`), `geometryToLayer` (`:99`: `L.geoJSON` with marker pointToLayer), `geometryToLatLngs` (the `[lng,lat]`→`[lat,lng]` recursive swap per geom type).
- Per-shape input/field wrappers (`shapes/*.tsx`) wrap the shells: `PointInput/Field`, `MultiPoint*`, `LineString*`, `MultiLineString*`, `Polygon*`, `MultiPolygon*`, `GeometryCollection*`, `BBoxInput/Field` (`+ valueTransform` to `[w,s,e,n]`). Plus composites `FeatureInput/Field`, `FeatureCollectionInput/Field`, free-form `GeoJsonInput/Field`.
- Read-only display: `shapes/shape-field-shell.tsx` — `useRecordContext()` → `<GeoJSON>` layer + auto `FitToData` (computes `L.geoJSON().getBounds()`), `emptyText` placeholder.
- Simple coord inputs: `lat-lng-input.tsx` (draggable marker + click-to-set + `RecenterOnChange`, writes two RHF fields `latSource`/`lngSource`), `lat-lng-field.tsx` (read-only mini-map, used as a `<DataTable.Col>` cell in `map-list.tsx:36` `MiniMapCell`).

**Geocoding (Nominatim):** `geocoding/nominatim-client.ts` — `GeocodingProvider` iface (`search`/`reverse`), 1s polite throttle (`:38`), browser-safe headers (no User-Agent), RFC7946 bbox-order conversion. `useGeocode`/`useReverseGeocode` hooks. `geocoding-input.tsx` — shadcn `<Command>` combobox (not Popover, to keep input focus), writes `source`+optional `latSource/lngSource/bboxSource` on select. `map-with-search.tsx` — composite: `<GeocodingInput>` + `<LatLngInput>` with two-way sync (drag marker → reverse-geocode → address field). `reverse-geocode-field.tsx` (read-only display).

**OSM/Overpass set operations (advanced — directly Koji-relevant):**
- `osm/overpass-client.ts` — POST to `https://overpass-api.de/api/interpreter`, typed `OverpassError/RateLimit/Timeout`, AbortController timeout.
- `osm/osm-presets.ts` — `OSM_PRESETS` catalog (water, buildings, forest, roads, + ~16 category catch-alls) each → Overpass tag filters; `bufferLinesMeters` auto-buffers line features to polygons; `buildOverpassQueryFromSources(sources, bbox)` compiles `[out:json][timeout:25]; (...); out geom;`. Note bbox→Overpass order `s,w,n,e` (`:115`).
- `osm/use-osm-features.ts` — `bbox + sources → useOverpass → osmtogeojson → filter to Polygon/MultiPolygon (+ buffer lines via @turf/buffer)`. React-Query-style `{data,isLoading}`.
- `osm/geometry-ops.ts` — Turf wrappers: `subtract` (@turf/difference), `unionAll` (@turf/union), `bboxOf` (@turf/bbox), `areaM2` (@turf/area), `polygonToBBox`/`bboxToPolygon`, `aspectLockedBBox`.
- `osm/osm-feature-operator.tsx` — the UI button (`<OsmFeatureAdd>`/`<OsmFeatureSubtract presets={["water"]}>`): takes current polygon form value, computes its bbox, fetches matching OSM polygons, subtracts/unions, writes result + toasts km² delta via `useNotify`/`useTranslate`.
- `simplify-input.tsx` — Douglas-Peucker via @turf/simplify, live `<Slider>` tolerance + Default/High `<ToggleGroup>`, snapshots original on mount, re-mounts `<GeoJSON key={JSON.stringify}>` per change.

**Map resource usage** (`apps/demo/src/map/`): `map-edit.tsx` = `<TabbedForm>` with tabs Details/Location/Multi-geometries/Composite exercising every input. `map-show.tsx` = `<TabbedShowLayout>` with every field. `map-list.tsx` = DataTable with a `LatLngField` mini-map cell + type `<Badge>`. `places-seed.ts` = 8 NYC POIs with every GeoJSON shape populated (Central Park, Brooklyn Bridge, High Line, etc.) — `Place` interface (`places-seed.ts:24`) carries `lat/lng + location:Point + area:Polygon + bbox + alt_locations:MultiPoint + route:LineString + trails:MultiLineString + boundaries:MultiPolygon + feature/features/geometries`. → great Koji fixture template.

## 5. planning / segments / analytics / scheduled-jobs (geo/scheduling-adjacent)
- **planning** (`planning/planning-list.tsx`): toggleable `<KanbanBoard groupBy columns cardRenderer>` ↔ `<CalendarList startSource titleSource>` (both from `shadmin/components/extras`). `groupBy="status"`, custom card via `<RecordContextProvider>` + `<DurationField>`. → Koji scheduling/job-board UI.
- **scheduled-jobs** (`scheduled-jobs/index.ts`): standard CRUD resource (`name:"scheduled_jobs"`); pairs with dashboard `<JobMonitor resource pollInterval>` and `extras/cron-input.tsx`/`cron-field.tsx`/`cron-utils.ts` (cron expression editor). → Koji golbat scheduled scans / cron jobs.
- **analytics** (`analytics/analytics-show.tsx`): no charts here — uses `<RecordTimeline>` + `<DiffViewer before after labels formatters>` (snapshot diffing) from extras. Charts are the dashboard's job.
- **segments** (`segments/data.ts`): just a static array `[{id,name:translationKey}]` consumed as filter choices + customer group badges (`customer-list.tsx:124,228`). Not geo — CRM segmentation. Koji "areas/segments" would instead be geometry-backed.

## 6. Notable extras components (in `packages/shadmin/src/components/extras/`)
`dashboard-charts`, `job-monitor`, `kanban` (KanbanBoard), `calendar-list`, `cron-input/field`, `currency-input/field`, `phone-input`, `color-input/field`, `duration-field`, `bulk-edit-drawer`, `csv-import`, `monaco` (JSON editor), `block-editor`/`rich-text-input` (TipTap 3.x), `command-menu`, `data-provider-devtools`, `diff-viewer`, `record-timeline`, `approval-queue`, `api-key-field/input`, `assistant` (AI). All shadcn-registry components.

---

## PATTERNS TO COPY FOR KOJI
1. **Map stack = Leaflet 1.9 + react-leaflet 5 + Geoman + Turf 7 + osmtogeojson + Nominatim/Overpass — NOT maplibre/mapbox.** Koji's existing Leaflet skill (`anthropic-skills:coding-with-leaflet`) and the user's `leaflet` skill align. Backport the whole `packages/shadmin/src/components/leaflet/` directory near-verbatim.
2. **`geoman/use-geoman-rhf.ts` is the keystone** — the RHF↔Geoman two-way bridge with hydration-echo dedup, multi/collection/featureCollection modes, and `valueTransform`/`valueParse` hooks. Port this first; everything else (per-shape inputs) is thin wrappers over `shape-input-shell.tsx`.
3. **Centralize the `[lng,lat]`↔`[lat,lng]` swap** in one `shared.ts` (`coordToLatLng`/`latLngToCoord`) + explicit RFC7946-vs-Nominatim bbox-order conversion (`nominatim-client.ts:85`). Directly addresses Koji's recorded geojson-bbox-order bug.
4. **OSM set operations** (`osm/` + `geometry-ops.ts` Turf wrappers + `osm-feature-operator.tsx`): `subtract`/`union`/`simplify`/`bbox` against live Overpass-fetched OSM polygons, with preset tag catalog and line-buffering — a ready-made model for Koji's geofence editing/derivation.
5. **Resource convention**: `<resource>/index.ts` exports a `ResourceProps` object (`name/list/edit/create/show/recordRepresentation/icon`); register via `<Resource {...r} group="..."/>` under one `<Admin>`. Seeds in `seed.ts`, merged centrally into a `fakeRestDataProvider(data, true, 500)`. Real backend = swap the `dataProvider` prop only.
6. **Read-only vs editable split**: every geometry has a `*Field` (display, `useRecordContext` + `<GeoJSON>` + auto-fit-bounds via `shape-field-shell.tsx`) and a `*Input` (edit, geoman + RHF via `shape-input-shell.tsx`). Mini-map as a DataTable cell (`LatLngField`, `map-list.tsx:36`).
7. **Dashboard = recharts v3 + the `extras/dashboard-charts.tsx` wrapper set** (`MetricCard`/`TrendChart`/`BarChart`/`DonutChart`), themed by `--chart-1..5` CSS vars.
8. **Realtime = transport-abstracted** (`RealtimeTransport` iface); WS transport (`websocket-transport.ts`) is production-ready with reconnect/heartbeat/auth/lock support — backport for golbat live scan updates; SSE alt available.
9. **Scheduling UI**: KanbanBoard/CalendarList toggle (`planning-list.tsx`) + cron-input/field + JobMonitor polling — maps onto Koji golbat scheduled-scan management.
10. **Distribution model**: components are shadcn-registry items (`registry.json`), each `"use client"` + self-contained, deps in the library `package.json`. If Koji web-client wants the same, mirror the registry build.

Key files (absolute):
- `/Users/rin/GitHub/shadcn-admin-kit/packages/shadmin/src/components/leaflet/` (whole dir — `index.ts`, `shared.ts`, `shared-map.tsx`, `lat-lng-input.tsx`, `simplify-input.tsx`, `geoman/use-geoman-rhf.ts`, `geoman/geoman-shape-mapping.ts`, `shapes/shape-input-shell.tsx`, `shapes/shape-field-shell.tsx`, `osm/*`, `geocoding/*`)
- `/Users/rin/GitHub/shadcn-admin-kit/apps/demo/src/map/` (consumer reference: `map-edit.tsx`, `map-show.tsx`, `map-list.tsx`, `map-create.tsx`, `places-seed.ts`, `index.ts`)
- `/Users/rin/GitHub/shadcn-admin-kit/packages/shadmin/src/components/extras/dashboard-charts.tsx`
- `/Users/rin/GitHub/shadcn-admin-kit/packages/shadmin/src/components/realtime/` (`index.ts`, `transports/websocket-transport.ts`, `transports/sse-transport.ts`)
- `/Users/rin/GitHub/shadcn-admin-kit/apps/demo/src/app.crm.tsx`, `data-provider.ts`, `customers/`, `products/`, `dashboard/dashboard.tsx`, `planning/planning-list.tsx`, `scheduled-jobs/index.ts`


---

# 8. Research — MapLibre GL Ecosystem

MDPI is hard-walled (Akamai). The benchmark's key finding (MapLibre mid-pack, Mapbox fastest at 50k+) is already captured from the search abstract. I have everything needed. Compiling the dense trace-doc material.

---

# Modern Web Map Frameworks for Geometry + Route Editing — MapLibre Ecosystem (current 2026-06-19)

## §1 Base renderer — MapLibre GL JS

- **`maplibre-gl`** — current **v5.24.0** (2026-04-23); 161 releases; **BSD-3-Clause** (the clean license, the whole reason it was forked from mapbox-gl-js v1 after Mapbox's Dec-2020 relicense to proprietary BSL). GitHub **10.9k★**, 1.1k forks, 343 open issues, actively maintained (TS 88.5% / GLSL 2.4%). [github.com/maplibre/maplibre-gl-js]
- v5.x line is the current major. Globe projection shipped; WebGPU is **roadmap, NOT released** for GL-JS as of 2026-04 — WebGPU work is landing in MapLibre **Native** (mobile/desktop) first. Treat WebGPU as "future, not bankable." [maplibre.org/news 2025-09/2025-10]
- Rendering model: GPU-accelerated vector-tile renderer (WebGL). Points/lines/polygons drawn as GPU layers off a `GeoJSONSource` or vector-tile source. Built-in **clustering** on `GeoJSONSource` (`cluster: true`, `clusterRadius` default 50, `clusterMaxZoom`). [maplibre.org/maplibre-gl-js/docs/examples/create-and-style-clusters]
- **Perf verdict for Koji's "thousands of pokestops via pixi":** a `circle` or `symbol` layer over a `GeoJSONSource` is the native, idiomatic replacement for the Pixi overlay — points become part of the GPU texture, tens of thousands render with smooth pan/zoom **without** a separate canvas. The 2025 MDPI benchmark (*Vector Data Rendering Performance Analysis of Open-Source Web Mapping Libraries*, ISPRS IJGI 14(9):336) ranks MapLibre **mid-pack**: at ≥50k points Mapbox GL JS fastest, then OpenLayers, then MapLibre, then Leaflet. So MapLibre comfortably covers low-tens-of-thousands of circles; it is *not* the absolute fastest at 50k+, but it beats Leaflet and beats a Pixi-overlay-on-Leaflet maintenance burden. [mdpi.com/2220-9964/14/9/336 — abstract only, full text Akamai-walled]
- For **>~100k–millions** of points, MapLibre's own large-data guide says: cut coordinate precision to 6 dp, chunk/stream, pre-tile with **tippecanoe**/**Martin** server-side (Martin "comfortably handles a 13GB DB"), or push to **deck.gl** hexbin/Scatterplot. Hard limit of GeoJSON-source + circle layer is not documented but clustering/`maxZoom:12`/disable-overlap are the prescribed escape hatches. [maplibre.org/maplibre-gl-js/docs/guides/large-data]

## §2 React integration

- **`@vis.gl/react-maplibre`** — the standalone repo `visgl/react-maplibre` was **ARCHIVED 2025-01-29** (read-only, 62★). Source folded into `visgl/react-map-gl`. Do **not** depend on the standalone repo's issue tracker. [github.com/visgl/react-maplibre]
- **`react-map-gl`** — current **v8.1.1** (2026-04-11), **MIT**, **8.5k★**, 1.4k forks, active. v8.0 introduced the **`react-map-gl/maplibre`** subpath import (typed for `maplibre-gl>=4`, no mapbox-gl/placeholder dep needed). v8.1 rewrote camera sync via Proxy (better terrain/non-mercator). React **>=16.3**. This is the canonical React wrapper. [github.com/visgl/react-map-gl/releases]
- Two install shapes: `npm i react-map-gl maplibre-gl` then `import {Map} from 'react-map-gl/maplibre'`, OR the legacy `@vis.gl/react-maplibre` package (MIT, same components, published from the unified repo). Prefer the `react-map-gl/maplibre` subpath going forward.
- TypeScript: first-class (wrapper is TS, maplibre-gl ships its own types). SSR/Next.js: standard caveat — `maplibre-gl` touches `window`/WebGL, so the `<Map>` must be client-only (dynamic import `{ssr:false}` or `'use client'`); no special SSR support beyond that.

## §3 Geometry EDITING libraries

### §3.1 Mapbox GL Draw — DEAD END for MapLibre
- `@mapbox/mapbox-gl-draw` is **no longer actively maintained** and is increasingly hard to run against MapLibre (API divergence mapbox-gl v2+ vs maplibre-gl). Forks exist but are unmaintained drift. **Do not pick this for a 2026 MapLibre admin.** [foss4g-2025 talk FJYFLZ; consensus across sources]
- `maplibre-gl-draw` as a standalone first-party package does **not** exist as a maintained thing — the de-facto answer is Terra Draw (below).

### §3.2 Terra Draw — THE recommendation ⭐
- **`terra-draw`** (James Milner, OSGeo project) — current **v1.31.2** (1.x **stable**, published within hours of this research; extremely active, 966+ commits). **MIT**, **1.0k★**, TS 99.2%. Framework-agnostic core + per-library **adapters**: Leaflet v1, OpenLayers v10, **MapLibre GL JS v4/5**, Google Maps v3, Mapbox GL v3, ArcGIS JS v4. [github.com/JamesLMilner/terra-draw; npmjs.com/package/terra-draw]
- **Architecture:** central in-memory **Store** of GeoJSON Features; adapters render the store onto the host map; you drive it imperatively. Key API: `addFeatures(Feature[])` (load existing geometry — Point/LineString/Polygon whose mode is enabled), `getSnapshot()` (read all features back as GeoJSON), `draw.on("change",(ids,type,ctx)=>…)` / `"finish"` events with `ctx.origin === "api"` to distinguish programmatic vs user edits. [guides/2.STORE.md, guides/6.EVENTS.md]
- **11 modes** with editing capability matrix [guides/4.MODES.md]:
  | Mode | Output geom | Vertex drag | Midpoint insert | Coord delete | Snapping | Validation | Resize |
  |---|---|---|---|---|---|---|---|
  | Point | Point | ✔ (via Select) | — | — | — | — | — |
  | LineString | LineString | ✔ | ✔ | ✔ | coord+line | `ValidateNotSelfIntersecting` | — |
  | Polygon | Polygon | ✔ | ✔ | ✔ | coord+line + `toCustom` fn | self-intersect + min/max area | ✔ center/opposite |
  | Circle | Polygon(circular) | ✔ | — | — | — | — | — (drag) |
  | Rectangle | Polygon | ✔ | ✔ | — | — | — | ✔ |
  | Angled Rectangle | Polygon(rotatable) | ✔ | ✔ | — | — | — | ✔ |
  | Freehand | Polygon | ✔ | — | — | — | — | — |
  | Freehand LineString | LineString | ✔ | — | — | — | — | — |
  | Sector / Sensor / PolyLine | Polygon/LineString | partial-doc | — | — | — | — | — |
- **Select mode** is the universal editor: per-geometry flags for `draggable` (whole feature), draggable/deletable **coordinates**, **draggable midpoints** (mid-vertex insertion), **resizable** (`center`|`opposite`, optional fixed aspect), same-mode coord+line **snapping**, and custom `validation` fns run on `finish`/`commit`/`provisional`. `pointerDistance` (default 40px) tunes hit-buffer.
- **Gaps for Koji's needs (explicitly NOT in Terra Draw):**
  - **MultiPolygon** — core modes emit single Polygon/LineString/Point. No first-class multipolygon *drawing*; you'd compose multipolygons yourself in the store/snapshot layer.
  - **Polygon cut / merge / boolean ops** — NOT provided. Bring **Turf.js** (`turf.union`/`turf.difference`/`turf.intersect`) and write features back via `addFeatures`.
  - **Coordinate precision setting** — not documented as a knob.
  - **Freehand on touch** — unsupported.
- **React caveat (load-bearing for Koji):** Terra Draw holds raw references to MapLibre layers. Under `react-map-gl/maplibre`, when a **sibling** component (e.g. a `<Marker>` update) forces the `<Map>` to re-render, Terra Draw can **lose its layer refs** → drawings vanish + `Cannot read properties of undefined (reading 'setData')`. Issue **#197 is OPEN/unresolved**. Mitigation: own the `maplibre.Map` instance via a stable ref, memoize map children hard, keep the Terra Draw instance in a `useRef` created once in a `useEffect`, and avoid driving markers through React state that re-renders the map. This is the single biggest integration risk for an editing admin. [github.com/JamesLMilner/terra-draw/issues/197]

### §3.3 maplibre-gl-terradraw — turnkey control (optional convenience layer)
- **`@watergis/maplibre-gl-terradraw`** (Jin Igarashi / watergis) — current **v1.13.2** (2026-05-30), 94 releases, **MIT**, **156★**, TS 75% / Svelte 10%. Wraps Terra Draw as a ready MapLibre `IControl` with a button UI: `new MaplibreTerradrawControl({modes:['polygon','select','delete',…]})`. Modes exposed: point, linestring, polygon, rectangle, circle, freehand, angled-rectangle, sensor, sector, select, delete-selection, delete, download. Bundles **Turf** for live area/distance measurement. Production users: Cartes.app, Open Hinata 3, UNDP GeoHub. [github.com/watergis/maplibre-gl-terradraw; FOSS4G-2025 talks UTUW8Z/FJYFLZ]
- **No documented React wrapper** — it's a vanilla MapLibre control. In `react-map-gl` you add it via `useControl`/`map.addControl` in an effect. For a custom-styled admin you likely skip this and use **Terra Draw core directly** for full UI control; use this control only if you want the prebuilt toolbar fast.

## §4 Routes / ordered MultiPoint / waypoint editing

- **`@maplibre/maplibre-gl-directions`** — current **v0.9.1** (2025-11-27), **MIT**, **156★**, 100% TS. Click-to-add waypoints, click-to-remove, **drag waypoints**, insert waypoint by dragging the route line, alternative routes, `refreshOnMove` (live re-route while dragging), per-waypoint bearings, congestion styling. **Requires an external OSRM- or Mapbox-Directions-compatible routing backend** — it is NOT a pure client-side waypoint editor; it always re-routes through a server. Does **not** advertise arbitrary waypoint **reordering** (sequential model). [github.com/maplibre/maplibre-gl-directions]
- **`route_snapper`** (dabreegster) — draws MapLibre routes **client-side**, snapped to a street network, no remote routing API. Good if Koji wants snapped routes without a routing server. [github.com/dabreegster/route_snapper]
- **Arrowed polylines / direction glyphs:** native MapLibre, no library — a `symbol` layer with `symbol-placement:'line'`, `symbol-spacing`, and an arrow `icon-image` (or a `line` layer with `line-pattern`) over the route LineString. This is the standard pattern; no dependency.
- **Ordered MultiPoint reorder verdict:** none of the above gives drag-to-reorder of waypoint *order* out of the box. For Koji's TSP-route editing, the realistic build is: Terra Draw (or raw circle layer) for the points + own React state holding the ordered array + a `line` layer redrawn from that order + arrow `symbol` layer for direction. Reorder is your own UI (drag list / map-drag remap), not a library feature.

## §5 Verdict for a geometry-editing admin (Koji)

- **Stack:** `maplibre-gl@5` + `react-map-gl@8` (`react-map-gl/maplibre` subpath) + **`terra-draw@1`** (core, not the prebuilt control) + **Turf.js** for cut/merge/multipolygon assembly + native `symbol`/`line` layers for routes & arrows. deck.gl `MapboxOverlay`/`ScatterplotLayer` held in reserve only if point counts blow past tens-of-thousands.
- **Pros:** all MIT/BSD (no Mapbox BSL exposure — matches Koji's license caution from the bootstrap/clustering memos); native GPU point rendering retires the Pixi overlay; Terra Draw covers polygon/rectangle/circle/line/point draw+edit incl. vertex drag, midpoint insert, snapping, self-intersection guard, resize; GeoJSON in/out via `addFeatures`/`getSnapshot` maps cleanly onto Koji's geo-types/KojiGeometry domain.
- **Cons / risks:** (1) **React re-render fragility** (issue #197, OPEN) — must isolate the map instance and memoize aggressively; biggest integration hazard. (2) **No native multipolygon/cut/merge** — Turf-bolted, you own the orchestration. (3) **No drag-to-reorder waypoints** — custom UI. (4) MapLibre point perf is mid-pack vs Mapbox at 50k+ — fine for thousands, plan tiling/deck.gl for the long tail. (5) WebGPU not yet in GL-JS — don't architect around it. (6) Freehand unsupported on touch. (7) `maplibre-gl-terradraw` and `maplibre-gl-directions` are small (~156★) single-maintainer projects — bus-factor risk; Terra Draw core (1.0k★, OSGeo-backed, daily commits) is the safer dependency.

Sources: github.com/maplibre/maplibre-gl-js · github.com/visgl/react-map-gl(+releases) · github.com/visgl/react-maplibre · github.com/JamesLMilner/terra-draw (guides 2/4/6, issue #197) · npmjs.com/package/terra-draw · github.com/watergis/maplibre-gl-terradraw · github.com/maplibre/maplibre-gl-directions · github.com/dabreegster/route_snapper · maplibre.org/maplibre-gl-js/docs/guides/large-data · mdpi.com/2220-9964/14/9/336 · deck.gl/docs/developer-guide/base-maps/using-with-maplibre · maplibre.org/news (2025-09, 2025-10)


---

# 9. Research — Alternative Map Frameworks

# Alternative Web Map Frameworks for Geometry/Route Editing (current to 2026-06-19)

Raw material for refactor trace. Koji currently uses Leaflet + leaflet-geoman (per MEMORY index, `leaflet` skill, and global rules). All npm figures pulled live from registry.npmjs.org + api.npmjs.org on 2026-06-19. Hard perf numbers from a 2025 peer-reviewed benchmark (MDPI ISPRS Int. J. Geo-Inf. 14(9):336 — Leaflet 1.9.4 / Mapbox GL 3.7.0 / MapLibre GL 4.7.1 / OpenLayers 10.2.1, RTX 3070, Chrome 131, 10-run avg, 1240 runs total).

## Hard data table (live npm, 2026-06-19)

| lib | latest | released | weekly dl | license |
|---|---|---|---|---|
| `terra-draw` | 1.31.2 | 2026-06-17 | 173,538 | MIT |
| `@geoman-io/leaflet-geoman-free` | 2.19.3 | 2026-04-10 | 138,803 | MIT |
| `@deck.gl-community/editable-layers` | 9.3.7 | 2026-06-11 | 25,557 | MIT |
| `rlayers` (OL React wrapper) | 3.9.0 | 2026-02-11 | 4,193 | ISC |
| `ol` (OpenLayers core) | 10.9.0 | 2026-04-15 | 725,009 | BSD-2 |
| `maplibre-gl` | 5.24.0 | 2026-04-23 | 3,223,650 | BSD-3 |
| `leaflet` (core, **stable**) | **1.9.4** | **2023-05-18** | 5,607,511 | BSD-2 |

Note the standout: **Leaflet stable is still 1.9.4 from May 2023 — 3 years stale.** 2.0 exists only as `2.0.0-alpha.1` (alpha released 2025-08, no stable timeline). Massive download count is install-base inertia, not active core dev.

---

## 1. OpenLayers (`ol`)

**Editing capability — strongest native editing of the open-source pack, honestly assessed as best-in-class.** Three composable interaction classes, all first-party (no plugin, no fork):
- `ol/interaction/Draw` — Point, LineString, Polygon, Circle, MultiPoint/Line/Polygon, plus `geometryFunction` for arbitrary shapes (boxes, regular polygons, freehand via `freehand:true`). `appendCoordinates`, `removeLastPoint`.
- `ol/interaction/Modify` — vertex drag, **midpoint/vertex insert** (alt-click or drag a segment), vertex delete (alt-click). Operates on a `VectorSource` or `Collection`.
- `ol/interaction/Snap` — snap-to-vertex and snap-to-edge across an entire source; the canonical pattern is to add Snap **last** so it intercepts Draw/Modify pointer events. Official examples: `draw-and-modify-features`, `draw-modify-trace-snap` (the `trace` interaction follows existing geometry edges — useful for shared borders / route-on-network).
- Full multipolygon/multilinestring editing, hole editing, topology preservation via Snap.

This is the **only** option here where polygon/multipolygon/circle/route + snapping + vertex-insert is all first-party and battle-tested — geoman replicates this for Leaflet as a plugin; OL ships it in core.

**Perf at 10k+** (MDPI 2025): Canvas renderer — Leaflet & OL are fastest of all four up to 50,000 lines / 10,000 polygons. For **100,000 lines OpenLayers won outright at 1295.5 ms (~2× faster than Mapbox/MapLibre)**. Only crack: 50,000 polygons, OL 1366 ms vs Mapbox 1265 ms (first time OL lost). OL also has a separate **WebGL path** (`WebGLVectorLayer`, `WebGLPointsLayer`) for tens-of-thousands of features GPU-accelerated, but GitHub issue openlayers#15619 documents memory growth + non-smooth panning on very large `WebGLVectorLayer` sources (OL 9). So: Canvas is excellent to ~50k; WebGL extends further but with rough edges. A 2026-04 Medium post flags an OL WebGPU "pixel drift" problem — WebGPU path is immature.

**License:** BSD-2-Clause. Clean, permissive, no commercial tier. Best license story of the editing-capable options.

**React + TS:** OL core is shipped with TS types (written in JS, `.d.ts` provided). React wrappers:
- `rlayers` (3.9.0, ISC, TS-native, maintainer publicly commits to long-term maintenance) — exposes `<RLayerVector>` + `<RInteraction.RDraw>` / `RModify` / `RSnap` as declarative components. Only runtime dep is lru-cache (~8KB). Low downloads (4.2k/wk) = small community but healthy.
- Alternative: skip the wrapper, mount OL imperatively in a `useEffect` + ref (the common production pattern; React owns the DOM node, OL owns the canvas). For an editing-heavy admin this is arguably *more* robust than a declarative wrapper because edit state lives in OL's `Collection`/`VectorSource`, not React state.

**Activity:** Core `ol` 10.9.0 (2026-04-15), 725k weekly dl, continuous major releases. Very healthy.

**Verdict:** The serious upgrade target if leaving Leaflet. Editing is its home turf.

---

## 2. deck.gl + EditableGeoJsonLayer (`@deck.gl-community/editable-layers`, ex-nebula.gl)

**Lineage matters here.** Original `uber/nebula.gl` (`@nebula.gl/layers` 1.0.4) is **dead** — last publish ~3 years ago, no maintainers, repo closed to external contributions. The live successor is `@deck.gl-community/editable-layers`, **9.3.7 (2026-06-11)**, MIT, 25.5k weekly dl. Examples modernized to Vite + TS + React 19, targets **deck.gl v9**.

**Editing capability:** `EditableGeoJsonLayer` edits a GeoJSON FeatureCollection — Point/LineString/Polygon/Multi* via swappable **edit modes**: `DrawPolygonMode`, `DrawLineStringMode`, `DrawCircleByDiameterMode`/`DrawCircleFromCenterMode`, `DrawRectangleMode`, `ModifyMode` (vertex drag + **midpoint insert** + delete), `TranslateMode`, `RotateMode`, `ScaleMode`, `DuplicateMode`, plus boolean ops (`extrudeMode`, `splitPolygonMode`). Snapping exists but is weaker/less polished than OL's or geoman's — it's the historically least-loved part. Route editing = LineString modify; no network-snap/trace equivalent out of the box.

**Perf at 10k+ — this is its entire reason to exist.** deck.gl is GPU/WebGL2: ScatterplotLayer renders ~**1M points at 60 FPS**, framerate drops to 10–20 FPS approaching 10M; hard crash ~10–100M (Chrome 1GB contiguous-buffer cap). For 10k points this is overkill but trivially smooth. **Caveat:** `EditableGeoJsonLayer` is heavier than a static layer (it re-tessellates on edit, renders guide/handle sublayers) — the GPU win is for *rendering* the underlying data, not for the edit overlay itself. If Koji's actual pain is "10k+ points on screen while editing a few," deck.gl is the only option that renders the backdrop at 60 FPS.

**License:** MIT (deck.gl core Apache-2.0). No commercial tier.

**React + TS:** Best-in-class React story — `@deck.gl/react` `<DeckGL>` is a real React component; deck.gl was built React-first at Uber. TS support out of the box. Interops with MapLibre/Mapbox as a base layer via `MapboxOverlay` / interleaved rendering — **so you can keep MapLibre/Leaflet as basemap and overlay deck.gl edit layers.** This is the key interop point: deck.gl is not a basemap, it's an overlay.

**Activity:** `deck.gl-community` is explicitly **"semi-maintained"** — repo's stated mission is "preserve valuable deck.gl ecosystem code that lacks a dedicated home," and "some modules may have no dedicated maintainer, sometimes no one responds quickly to issues." Releases track deck.gl v9 cadence (overall repo at 9.3.7, Jun 2026) but editable-layers specifically rides on community goodwill. **This is the risk flag.**

**Verdict:** Pick only if huge-dataset rendering is the dominant requirement. For pure editing ergonomics it's a step down from OL, and maintenance is "semi."

---

## 3. Leaflet, modernized (stay-put options)

### 3a. leaflet-geoman (what Koji uses today)
`@geoman-io/leaflet-geoman-free` **2.19.3 (2026-04-10)**, MIT, 138.8k weekly dl, 2.4k stars, ~23 open issues. **Actively maintained** — primarily by @Falke-Design, recent work includes polygon-hole removal with min-vertex enforcement, temp-layer cleanup, improved snapping. **This is not abandonware; it's healthy.**

**Editing capability — genuinely complete:** Draw, Edit, Drag, **Cut**, **Rotate**, **Split**, **Scale**, Measure, **Snap**, Pin. Supports Markers, CircleMarkers, Polylines, Polygons, Circles, Rectangles, ImageOverlays, LayerGroups, GeoJSON, **MultiLineStrings and MultiPolygons**. Vertex add/remove/drag, snapping with config. Feature-for-feature this *matches* OL's interactions and adds split/cut/measure on top. The **free tier covers everything Koji needs**; Geoman Pro (separate commercial license, no per-seat/royalty) adds advanced UX but is not required.

**License:** MIT (free package). Pro is a separate paid SKU — irrelevant unless you opt in.

**TS:** Ships `leaflet-geoman.d.ts`. React: no official React binding — used imperatively against a Leaflet `map` instance (which is how Koji already does it).

### 3b. Leaflet core + Turf.js
Leaflet handles render/interaction; **Turf** (`@turf/*` 7.x) does the geometry math (buffer, union, difference, intersect, simplify, midpoint, nearest-point-on-line, transform-rotate/scale/translate, area, distance, boolean-point-in-polygon). Note: **Terra Draw and deck.gl-community both depend on Turf 7.2 internally** — Turf is the de-facto geometry engine regardless of which renderer you pick, so Koji can adopt it independent of the map-framework decision.

**Is Leaflet a "relic" in 2026?** Blunt answer: **the core is stale, but not broken, and for editing it's irrelevant.**
- Core stable frozen at 1.9.4 since 2023-05; 2.0 only alpha (2025-08, ESM, drops IE, Pointer Events, no global `L`). No stable timeline.
- **But for editing-heavy admin specifically, Leaflet is still fine** — the editing lives in geoman (actively maintained, 2026-04 release), not in Leaflet core. Leaflet's job is pan/zoom/DOM + Canvas vector rendering, which 1.9.4 does correctly and won't regress.
- **Perf (MDPI 2025):** Leaflet is *fastest of all four* for <1000 features (~1.15× faster than OL), competitive to 10k. **Falls apart above 50k** — joins MapLibre as slowest, ~2× the time of OL/Mapbox; 50k+ exceeds 1s render. Default PNG markers are a perf trap (10k markers ≈ 30s, glitchy) — must use `L.circleMarker`/SVG. So Leaflet is fine for thousands, weak for tens of thousands.

**Verdict on staying:** If Koji's datasets are in the thousands, not tens-of-thousands, **staying on Leaflet+geoman is defensible and lowest-risk** — geoman is healthy, editing is complete, the only debt is core staleness (mitigated by eventual 2.0 or just pinning 1.9.4 which works). The "relic" framing only bites if (a) you need 50k+ live features or (b) you want ESM/modern-bundler ergonomics that 1.9.4 lacks.

---

## 4. Commercial — Mapbox GL JS & Google Maps

### Mapbox GL JS v3
**License is the disqualifier-or-not.** Mapbox GL JS went proprietary in Dec 2020 (v2+); v3 is non-OSS, usage-billed. **Pricing 2025:** free tier 50,000 map loads/mo, then **$5 per 1,000 loads**. A "map load" = each `Map` instantiation; pan/zoom/style-toggle within a 12-hour session is free. **Editing:** no native edit interactions — you'd layer Mapbox GL Draw (`@mapbox/mapbox-gl-draw`) or Terra Draw on top. **Perf (MDPI):** GPU/WebGL; fastest at 50k polygons (1265 ms), but a **fixed ~160 ms init cost regardless of feature count** + token-auth round-trip. Loses to OL at 100k lines. **Verdict:** the licensing + per-load billing is a hard disqualifier for an open-ish admin tool when **MapLibre GL is the free BSD-3 fork of this exact codebase** (`maplibre-gl` 5.24.0, 3.2M weekly dl) — Koji should use MapLibre, never Mapbox, if going the GL route.

### Google Maps JavaScript API
**License: pure commercial, pay-per-load.** 2025 pricing (post-March-2025 restructure): Dynamic Maps (JS) **$7.00/1,000 loads** ($0.007/load to 100k, $0.0056 after), per-SKU free caps (10k Essentials / 5k Pro / 1k Enterprise events/mo), up to 80% volume discount. **Editing:** the Drawing Library (`google.maps.drawing.DrawingManager`) does polygon/polyline/circle/rectangle/marker draw; vertices are editable (`editable:true`, `draggable:true`) with midpoint insert; **no snapping, no multipolygon, no route-network trace** — weakest editing of the lot. **Verdict:** hard disqualifier — recurring per-load cost + proprietary terms + weakest editing. Only justified if you specifically need Google's basemap/Street View/Places, which a geometry-editing admin does not.

---

## 5. Newer 2025–2026 entrants

### Terra Draw (`terra-draw` 1.31.2, 2026-06-17) — **THE entrant that matters for Koji**
James Milner's library, founded 2023, hit **1.0 in 2025**, MIT, **173.5k weekly dl** (already outpaces geoman), OSGeo project. **It is library-agnostic via an adapter pattern** — one drawing/editing API that runs on **Leaflet (v1), MapLibre (v4/5), Mapbox (v3), OpenLayers (v10), Google Maps (v3), ArcGIS (v4)**. Written 99% TypeScript, fully typed. Depends on Turf 7.2 internally.

**Editing capability — broad and modern:**
- Draw modes: Point, LineString, Polygon, Circle, Rectangle, **AngledRectangle**, **Sector**, **Sensor**, Freehand (polygon), FreehandLineString, PolyLine, Marker.
- **Select mode** (one per instance): per-feature-type editing — feature drag, **vertex drag, midpoint insertion + drag, coordinate deletion**, resize (`center`/`opposite`/`center-fixed`/`opposite-fixed`), **rotation + scaling** (geodesic on globe projection; resizable currently web-mercator only).
- **Snapping** via `flags`: `toCoordinate` and `toLine` (snaps to coords/segments of same-mode geometries); custom validation hooks; self-intersection guards.
- Geodesic line/circle drawing (globe projection). Deep styling. Custom modes/adapters extensible.

**The strategic point:** Terra Draw lets Koji **decouple the editing layer from the basemap choice.** It runs on Koji's *current* Leaflet today, and if Koji later swaps the basemap to MapLibre or OL, the editing code stays identical — only the adapter import changes. This is the lowest-risk migration path: adopt Terra Draw on existing Leaflet now, keep the basemap-swap as an independent future decision.

**Gaps vs geoman:** geoman has explicit **cut / split / measure**; Terra Draw doesn't ship those as named modes (cut/split would need custom mode + Turf difference). For raw route/polygon editing + snapping, Terra Draw is at parity or ahead.

**Activity:** extremely active (1.31.2 this week, 966+ commits, 189 tags), single very-engaged maintainer + OSGeo backing. Risk = bus-factor, mitigated by MIT + clean codebase.

### Others
- **MapLibre GL JS 5.24.0** (2026-04, BSD-3, 3.2M weekly dl) — the free Mapbox fork; the default modern GL basemap. No native editing (use Terra Draw / mapbox-gl-draw on top). MapLibre v5 added globe projection, perf work. This is the natural basemap if leaving Leaflet's raster/canvas world.
- No other credible *new* React-geospatial-editing framework emerged 2025–2026 beyond the above; the action is consolidation (Terra Draw as cross-lib editor, deck.gl-community absorbing nebula.gl, MapLibre absorbing Mapbox's OSS mantle).

---

## Blunt ranking (for Koji: editing-heavy admin, currently Leaflet+geoman)

1. **Terra Draw (on current Leaflet, or on MapLibre/OL later) — best fit.** Modern, TS-native, actively developed, decouples editing from basemap so the migration is incremental and reversible. Adopt this regardless of whether you ever change the basemap. Only real gap is named cut/split/measure (geoman has them).

2. **Stay on Leaflet + leaflet-geoman — lowest risk, still valid in 2026.** geoman is healthy (2.19.3, Apr 2026) with the most complete editing feature set (cut/split/rotate/scale/measure). The only debt is stale Leaflet *core*, which doesn't affect editing. Correct default **if datasets stay in the thousands and you don't need ESM-modern tooling.** Don't migrate just because Leaflet "feels old."

3. **OpenLayers (`ol` + `rlayers` or imperative) — best if you want a real framework upgrade.** First-party Draw/Modify/Snap/trace, best open license (BSD-2), strongest canvas perf (wins 100k lines outright), WebGL path for more. Heaviest migration cost (full rewrite of map layer), but the most capable single-framework editing home long-term.

4. **deck.gl-community editable-layers — only if 10k+ live features on screen is the gating requirement.** Unmatched GPU rendering (1M pts @ 60 FPS), React-first, interops as overlay on MapLibre/Leaflet. Downgraded by "semi-maintained" status and weaker snapping/edit ergonomics. Niche pick.

5. **MapLibre GL JS (basemap) + Terra Draw/mapbox-gl-draw (editing)** — viable modern GL stack, free BSD-3, but MapLibre alone has *no* editing; it's a basemap that needs an editing lib bolted on, so it collapses into option 1's editing layer with a different basemap. Worse canvas-scale perf than OL/Leaflet below 50k per MDPI; init overhead ~350 ms.

6. **Mapbox GL JS — disqualified** by proprietary license + $5/1k-load billing when MapLibre is the free fork of the same code.

7. **Google Maps JS API — disqualified** by $7/1k-load recurring cost, proprietary terms, and the *weakest* editing (no snapping, no multipolygon, no network trace).

**One-line recommendation for the trace doc:** adopt **Terra Draw** as the editing layer on the **existing Leaflet basemap** (incremental, reversible, gains modern TS editing + snapping), keep **leaflet-geoman** only if you depend on its cut/split/measure modes, and treat a basemap swap to **OpenLayers** (best editing-framework license + perf) or **MapLibre+deck.gl** (only for 10k+ live features) as a *separate, later* decision that Terra Draw's adapter layer makes cheap.

Sources: live npm registry/api (2026-06-19); MDPI ISPRS IJGI 14(9):336 (2025) — https://www.mdpi.com/2220-9964/14/9/336 ; https://github.com/JamesLMilner/terra-draw + https://terradraw.io ; https://github.com/geoman-io/leaflet-geoman ; https://visgl.github.io/deck.gl-community/docs/modules/editable-layers + npm `@deck.gl-community/editable-layers` ; https://openlayers.org/en/latest/examples/draw-modify-trace-snap.html + GitHub openlayers#15619 ; https://github.com/mmomtchev/rlayers ; https://leafletjs.com/2025/05/18/leaflet-2.0.0-alpha.html ; https://docs.mapbox.com/mapbox-gl-js/guides/pricing ; https://mapsplatform.google.com/pricing .


---

# 10. Map Framework — Recommendation

# Koji V2 Map Framework — Recommendation

## TL;DR

**Primary: Stay on a Leaflet-class basemap, but replace `leaflet-geoman` with `terra-draw` as the editing layer.** Concretely: **`react-leaflet@4` + `leaflet@1.9.4` (BSD-2) + `terra-draw@1.31` (MIT) via its Leaflet adapter + `@turf/turf@7` (MIT)** for cut/merge/multipolygon orchestration. Keep the **Pixi overlay** for the 10k+ point layer for now (it works and is independent of the editing decision).

**Runner-up (the "real upgrade" path): `maplibre-gl@5` (BSD-3) + `react-map-gl@8` `/maplibre` subpath (MIT) + the *same* `terra-draw@1.31` + `@turf@7`,** native `circle`/`symbol` layers replacing Pixi, deck.gl held in reserve.

The single highest-leverage move is **adopting Terra Draw regardless of basemap.** Its adapter pattern decouples the editing code from the renderer, so the basemap swap (Leaflet → MapLibre/OL) becomes a *separate, reversible, later* decision — exactly the "future-proof, not urgent" framing the `/map` backport calls for. Terra Draw is the only choice that lets you de-risk both decisions independently.

---

## (1) The stack, with versions/licenses

| Layer | Pick | Version | License | Why |
|---|---|---|---|---|
| Basemap renderer | Leaflet (primary) / MapLibre GL (runner-up) | 1.9.4 / 5.24.0 | BSD-2 / BSD-3 | Both clean permissive; no Mapbox BSL exposure (matches Koji's license caution per bootstrap/clustering memos) |
| React wrapper | react-leaflet / react-map-gl `/maplibre` | 4.2.1 / 8.1.1 | MIT / MIT | react-leaflet already in tree (zero churn); react-map-gl is the canonical MapLibre React wrapper |
| **Editing** | **terra-draw** | **1.31.2** | **MIT** | Framework-agnostic adapters (Leaflet v1 + MapLibre v4/5 + OL v10); GeoJSON in/out; vertex/midpoint/snap/resize/rotate built in |
| Geometry math | @turf/turf | 7.x | MIT | union/difference/intersect/area/midpoint — the cut/merge/multipolygon engine. Already an internal dep of Terra Draw, so it's coming in regardless |
| High-count points | leaflet-pixi-overlay (keep) → native GL `circle` layer (on swap) | — | MIT / BSD-3 | Pixi stays under Leaflet; retired by a native GPU layer if/when you move to MapLibre |

**Why Terra Draw over staying on geoman:** geoman is healthy (2.19.3, Apr 2026) and feature-complete — this is *not* a "geoman is dying" argument. The argument is **portability**. geoman is welded to Leaflet; if Koji ever wants ESM-modern tooling or 50k+ live features, geoman cannot follow you to MapLibre/OL and you rewrite the entire editing layer. Terra Draw writes the editing layer *once* against a stable GeoJSON store and survives the basemap swap. For a feature being dropped-and-backported, betting on the portable layer is the correct future-proofing.

**Why not OpenLayers as primary:** OL has the best *native* editing (Draw/Modify/Snap/trace, first-party MultiPolygon) and best canvas perf (wins 100k lines outright) and the cleanest license (BSD-2). It is the strongest single-framework editing home and is the legitimate "if you're going to rewrite anyway" target. But it forces a **full rewrite of the map layer now** for a route that's being deferred — the cost lands before the value. Terra Draw-on-Leaflet captures ~80% of the modernization (TS-native editing, snapping, portability) at ~20% of the migration cost, and keeps OL open as the runner-up's runner-up. If you decide the backport *is* a full rewrite, promote OL.

---

## (2) Coverage of EACH current Koji map capability

Mapping against the inventory's geoman/arrowheads/pixi feature set:

**Geometry editing (polygon / multipolygon / circle / rectangle):**
- Draw: Terra Draw `PolygonMode`, `RectangleMode`, `CircleMode` (circle emits a circular Polygon — matches Koji's "circle drawn as route points/polygon" model). ✅ covers `drawPolygon`/`drawRectangle`/`drawCircle`.
- Edit/drag/rotate/scale: Terra Draw **Select mode** — per-feature vertex drag, **midpoint insertion**, coordinate delete, whole-feature drag, resize (`center`/`opposite`), **rotation**. ✅ covers geoman's `editMode`/`dragMode`/`rotateMode` and the per-shape `pm:edit`/`pm:dragend`/`pm:rotateend` rebinding (now a single `change`/`finish` event with `ctx.origin` to tell user-edits from API loads).
- Snapping: Terra Draw `flags: { toCoordinate, toLine }`, `pointerDistance` tunes the hit-buffer. ✅ covers geoman `snappable`/`snapIgnore`.
- Self-intersection guard: `ValidateNotSelfIntersecting` + min/max-area validation. ✅ matches/exceeds geoman.
- **MultiPolygon — GAP.** Terra Draw emits single Polygons. **You own multipolygon assembly in the store via Turf** (`turf.union` to merge, store as MultiPolygon, explode to Polygons for editing). This is the same orchestration Koji *already* does in `useShapes.combine` (turf `union`, `keepCutoutsOnMerge`) and `splitMultiPolygons`/`To MultiPolygon`/`To Polygon` — so the logic is largely a port, not net-new.
- **Cut — GAP.** geoman has `pm:cut`; Terra Draw doesn't ship a named cut mode. **Bolt with `turf.difference`** triggered from your own toolbar button → write result back via `addFeatures`. Koji's `Create Shape from Cutouts`/`getFeatureCutouts` already lives in app code, so the cut surface is partly yours today anyway.
- **Merge / combine — GAP→Turf.** Koji's `setters.combine` is *already* Turf-based, not geoman-based. Direct port. ✅ effectively.

**Route / MultiPoint / arrowhead editing:**
- Ordered MultiPoint model: keep Koji's existing model (ordered Points + `${a}__${b}` LineString links + `__forward`/`__backward`/`__multipoint_id`, `activeRoute` explode/collapse). Terra Draw draws/edits the Points; the **ordering + linking stays your React/store logic** (it already is). No library gives drag-to-reorder of TSP waypoints — neither geoman nor any alternative does today, so this is **zero regression**.
- Split line at midpoint (`splitLine`), remove-point-rebridge, reroute (`POST /jobs`): all **app-level store logic + API calls** — renderer-agnostic, carries over unchanged.
- **Arrowheads:** `leaflet-arrowheads` is Leaflet-only and **will not survive a basemap swap.** Replacement is *better* on the GL path: a native `symbol` layer with `symbol-placement:'line'` + arrow `icon-image` (MapLibre) renders direction glyphs with no dependency. On the Leaflet-primary path, keep `leaflet-arrowheads@1.4` as-is (the `getColor`-by-distance rules are app logic and port either way). ✅ covered, dependency-free on the upgrade path.

**10k+ point performance (pokestops/gyms/spawnpoints/stations):**
- **Primary (Leaflet):** keep `leaflet-pixi-overlay` + `pixi.js`. It already handles tens of thousands and is fully decoupled from the editing decision — no reason to touch it now. (MDPI: Leaflet core itself falls apart >50k, which is exactly *why* Pixi exists here; it stays.)
- **Runner-up (MapLibre):** the Pixi overlay is **retired** — points become a `GeoJSONSource` + native `circle`/`symbol` layer on the GPU texture, with built-in `cluster:true`/`clusterRadius`. MDPI ranks MapLibre mid-pack (behind Mapbox/OL, ahead of Leaflet at 50k+) — comfortably fine for low-tens-of-thousands. If counts blow past that, drop in **deck.gl `MapboxOverlay` + `ScatterplotLayer`** (1M pts @ 60 FPS) over the same MapLibre basemap, no rewrite of the rest. The Pixi `getScale`/`scaleMarkers`/per-geohash-color/`pokestopRange` logic maps onto layer `paint` expressions.

**Import overlays (Wizard / MiniMap / Shapefile / Nominatim / Golbat):**
- All import sources (`shapefile`, Nominatim `GET /nominatim`, Golbat fences, JSON) produce **GeoJSON FeatureCollections** — fully renderer-agnostic. `addFeatures(snapshot)` loads them into Terra Draw's store; `getSnapshot()` reads them back. ✅
- `MiniMap.tsx` preview (LayersControl bbox/feature) is a second small map instance — trivially a second `<MapContainer>`/`<Map>` regardless of basemap. ✅
- The `convert`/`area`/`merge-points`/S2/coverage backend calls are all API + Turf/`nodes2ts` client math — renderer-independent. ✅

---

## (3) Migration risk vs staying on Leaflet+geoman

**Cost of the primary (Leaflet + Terra Draw):** **moderate, bounded, incremental.** You keep react-leaflet, the Pixi overlay, panes, tile layer, locate/zoom controls, popups, the entire store/fetch layer, S2, geohash. You **rewrite only the geoman binding** (`Drawing.tsx` + per-shape `pm:*` rebinding in `Polygon.tsx`/`Point.tsx`) against Terra Draw's `change`/`finish` events, and **port** the already-Turf-based combine/cut/multipolygon ops to fire on Terra Draw's store. The toolbar UI (cut/merge/remove/rotate controls) becomes your own shadcn buttons driving Terra Draw modes — which you want anyway for theming (see §4).

**The one real risk to flag — Terra Draw React re-render fragility (issue #197, OPEN):** under React wrappers, a sibling re-render of the `<Map>` can make Terra Draw lose its layer refs (`Cannot read properties of undefined (reading 'setData')`). **Mitigation is well-understood:** own the map instance in a stable `useRef`, create the Terra Draw instance once in a `useEffect`, hard-memoize map children, and never drive the high-count markers through React state that re-renders the map (Koji already keeps markers in Pixi/imperative layers, not React children — so Koji is *structurally* well-positioned to dodge #197). This is the thing to prototype first in the backport.

**Cost of staying on Leaflet+geoman:** **near-zero now, higher later.** geoman is maintained and complete; there is no urgent forcing function. The debt is (a) Leaflet *core* frozen at 1.9.4 since 2023 (2.0 still alpha), and (b) geoman's Leaflet lock-in meaning any future renderer change = full editing rewrite. Staying is the correct call **if you're confident Koji never needs >50k live features and never wants off Leaflet.** Given V2 is a deliberate modernization onto shadcn/Tailwind, betting on never-moving is the riskier bet.

**Verdict:** the primary trades a moderate, *bounded* rewrite-now (one binding file + porting existing Turf logic) for portability and TS-native editing. Because the route is deferred, you spend the rewrite during the backport when you'd be touching this code anyway — the cost lands exactly when the value does, not before.

---

## (4) Tailwind / shadcn theming fit

This is where **Terra Draw decisively beats geoman**, and it's the most underrated factor for a shadcn admin:

- **geoman ships its own toolbar DOM + CSS** (`.leaflet-pm-toolbar`, injected buttons). Theming it into shadcn means fighting/overriding vendor CSS — exactly the kind of style-leak a Tailwind design system tries to eliminate. Koji *already* works around this with custom `Drawing.tsx` controls + MUI `StyledPopup`.
- **Terra Draw has NO built-in UI.** It's a headless editing engine — you call `draw.setMode('polygon')` imperatively. **You build the entire toolbar as native shadcn components** (`Button`, `Toggle`, `ToggleGroup`, `DropdownMenu`, `Tooltip`) wired to mode setters. The cut/merge/rotate/remove controls, the keyboard-shortcut map (`usePersist.kbShortcuts`), the mode toggles — all become first-class Tailwind/shadcn UI with zero vendor-CSS override. This is a clean fit, not a workaround.
- **Geometry styling** (the orange combine-select highlight, `__KOJI`/`__GOLBAT` color-by-source, distance-color rules) is set via Terra Draw's per-feature style functions / GL `paint` expressions reading feature properties — driven straight from your Tailwind CSS variables / theme tokens. The popups already need a rewrite off MUI `Paper` → shadcn `Popover`/`Card` regardless of map choice.
- On the **MapLibre runner-up**, the same holds and is even cleaner: no Leaflet control DOM at all, every control is yours.

Net: Terra Draw's headlessness is *the* reason it suits a shadcn admin better than geoman — you stop overriding a vendor toolbar and start composing your own.

---

## (5) Fallback option

**Fallback (if Terra Draw's #197 React fragility proves unworkable in the backport prototype, or you need named cut/split/measure without writing them): stay on `react-leaflet@4` + `leaflet-geoman-free@2.19` exactly as today.** It is healthy, MIT, feature-complete (cut/split/rotate/scale/measure all first-party), and lowest-risk. You absorb the shadcn-theming friction (override geoman's toolbar CSS, or hide it and drive geoman's API from your own shadcn buttons — geoman *can* be driven imperatively too) and accept the Leaflet lock-in. This is the safe harbor and requires zero migration.

**Second fallback / promote-on-rewrite: OpenLayers (`ol@10` BSD-2 + `rlayers@3` ISC, or imperative).** If the backport scope expands into a full map-layer rewrite, OL is the superior single-framework home: first-party Draw/Modify/Snap/trace + native MultiPolygon, best canvas perf, cleanest license. Terra Draw *also* has an OL v10 adapter, so even this path keeps your editing code if you adopt Terra Draw first.

---

## One-line decision

Adopt **Terra Draw now as the editing layer** (the portable, headless, shadcn-friendly bet that survives any basemap), keep **Leaflet + Pixi** as the basemap for the incremental backport, hold **MapLibre+native-GL (retiring Pixi)** as the future-proof runner-up, and keep **geoman-as-is** as the zero-risk fallback and **OpenLayers** as the promote-on-full-rewrite target.

Relevant files for the backport (all under `/Users/rin/GitHub/Koji/apps/web-client/src/`): `pages/map/interface/Drawing.tsx` (the geoman binding to rewrite), `pages/map/markers/{Polygon,Point,LineString}.tsx` (per-shape event rebinding → Terra Draw events), `hooks/{useShapes,useSyncGeojson,usePixi}.ts` (store + Turf combine/cut logic ports + Pixi layer), `services/{fetches,geoUtils}.ts` (renderer-agnostic, carry over).