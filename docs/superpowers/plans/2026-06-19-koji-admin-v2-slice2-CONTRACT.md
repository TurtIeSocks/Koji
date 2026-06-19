# Koji Admin V2 — Slice 2 (Route + lighter resources) — Contract

Builds on the committed foundation. The **template is the existing code** — mirror it:
- Resource pattern: `apps/web/src/resources/geofence/{index.ts,geofence-list,geofence-edit,geofence-create,geofence-show}.tsx`
- dataProvider: `apps/web/src/data-provider.ts` (`RESOURCE_MAP`, `unwrapResponse`, segFor/itemPath)
- shell registration: `apps/web/src/App.tsx` (`<Resource {...r} group="…">`)
- backend row endpoint template: `crates/koji-service/src/internal/geofences.rs` (`list_rows`) + `internal::scope()` + `v2::geofences::internal_item_scope()`
- browser-test pattern (stub dataProvider): the geofence `*.browser.test.tsx` files

## 1. Backend — route row-list (the only backend work)

- Add `crates/koji-service/src/internal/routes.rs` `list_rows` (mirror geofences): `GET /internal/routes` →
  `{status:"ok", data: RouteRow[], meta}` honoring `page/per_page/sortBy|sort_by/order/q` + filters
  (`mode`, `geofenceid`, `pointsmin`, `pointsmax`). Reuse `koji_db::db::route::Query::paginate` (route.rs).
  `RouteRow = { id:i64, name:String, description:String|null, mode:String, geofence_id:i64|null, points:usize }`
  (`points` = coordinate count of the MultiPoint geometry — verify how route paginate exposes it; reshape).
- In `internal::scope()`: register `GET /internal/routes` → `routes::list_rows`, and replace the current
  `v2::routes::scope()` mount with a new `v2::routes::internal_item_scope()` (mirror geofences: omit ONLY the
  collection GET-list; keep collection POST + `/{id}` getOne/patch/delete + `/{id}/publish` forwarding).
- DB-gated test in a new `crates/koji-service/tests/internal_routes_rows.rs` (mirror `internal_geofences_rows.rs`):
  row shape + meta; a middle-page `has_prev` assertion is nice-to-have (already covered for geofence).

## 2. Frontend — resource registration (`App.tsx`)

`<Resource {...geofence} group="Geo"/>`, `<Resource {...route} group="Geo"/>`,
`<Resource {...project} group="Config"/>`, `<Resource {...property} group="Config"/>`,
`<Resource {...tileserver} group="Config"/>`, `<Resource {...plugins} group="Config"/>`.

`RESOURCE_MAP` (data-provider.ts) already has all six. `route` is geo (getOne returns a Feature → record map);
project/property/tileserver/plugins are plain (macro CRUD, already forward-aliased).

## 3. Frontend — resources (mirror geofence; `src/resources/<name>/`)

- **route** (geo): List `<ListLive>`+DataTable cols name/description/mode/`geofence_id`(ReferenceField→geofence)/points;
  filters live-search + mode + geofence(ReferenceInput) ; Edit/Create `<SimpleForm>` name/description/mode(SelectInput)/
  `geofence_id`(ReferenceInput→geofence, required)/geometry(**`MultiPointInput`** from `@/components/leaflet`); Show + read-only geometry field.
  Mode choices: verify the route `mode` enum (likely same canonical set as geofence — reuse `GEOFENCE_MODES` or add `ROUTE_MODES` if the enum differs).
- **project** (plain): List name/description/`golbat`(boolean)/geofences-count; Edit + `<ReferenceArrayInput source="geofences" reference="geofence"><AutocompleteArrayInput/>`; Create; Show.
- **property** (plain): List name/category/default_value/geofences-count; Edit/Create name/`category`(SelectInput choices PROPERTY_CATEGORIES)/`<PropertyValueInput source="default_value"/>`; Show.
- **tileserver** (plain): List name/url; Edit/Create name/url + a live `<BaseMap tileUrl>` preview (shadmin leaflet); Show.
- **plugins** (plain, composite id `kind:name`): List name/kind/`enabled`(boolean)/version; **Edit only** (no create/show):
  `entrypoint`/`interpreter`/`protocol` disabled TextInputs, `enabled` BooleanInput, `args_default` `<MonacoJsonInput>`, `description` TextInput.

## 4. New component — `PropertyValueInput` (`src/components/inputs/property-value-input.tsx`)

Renders the `default_value` input by the sibling `category` field (RHF `useWatch`), per the property categories:
- `boolean` → `BooleanInput` · `string` → `TextInput` · `number` → `NumberInput` · `color` → `ColorInput` (shadmin extras)
- `object` / `array` → `MonacoJsonInput` (validates JSON object/array) · `database` → disabled field + helper text "resolved from DB at runtime"
- **On category change: KEEP the current value**, show a small note ("value may not match the new category"). Non-destructive.
- Reused later by geofence's properties-array — keep it a standalone, record-context-free input keyed on a `categorySource` prop (default `"category"`).

`PROPERTY_CATEGORIES = ['boolean','string','number','object','array','database','color']` → add to `src/lib/constants.ts`.

## 5. Constraints (inherit the foundation's)

- bun; `apps/web`; `@/`; Tailwind v4; shadmin (ra-core 5.14) via the vendored components.
- dataProvider hits `/internal` only (writes forward to public CRUD). Realtime already wired (resources get `<ListLive>`/`<EditLive>`/`<ShowLive>` for free; the macro/route handlers already emit events).
- Browser tests stub the dataProvider (msw is node-only); jsdom tests use msw. Run browser suites at end of each task; manual verify via Claude Preview at the end.
- TDD, bite-sized, conventional commits, on `claude/v2`.
