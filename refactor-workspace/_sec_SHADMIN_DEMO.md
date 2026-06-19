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