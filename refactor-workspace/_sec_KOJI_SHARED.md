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