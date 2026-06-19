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