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