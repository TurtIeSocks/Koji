# Overlap visualization — project maps + geofence ghost neighbors — design

**Date:** 2026-07-14
**Branch:** claude/v2
**Status:** Approved (approach), pending spec review

## Context

Two related requests, both to **visualize geofence overlap**:
1. **Project show/create/edit** get a map of the project's member geofences — read-only, hover→name, click→open that fence's edit page in a **new tab**. On create/edit the map reflects the current (form) geofence selection reactively.
2. **Geofence show/edit/create** get a toggle for read-only **"ghost" neighbor** fences — non-editable, same hover→name and click→new-tab-edit, so you can see what overlaps the fence you're working on.

Recon (`wf_7b81d4af-d8b`) + a `main`-branch check established: **no spatial/bbox geofence query exists in V1 or v2** — V1's map fetches *all* fences and culls by viewport client-side. The user has ~600 fences in prod and does not want to load all of them per dashboard visit, so both features are backed by **new scoped backend queries**, not a fetch-all.

`koji_core` already has bbox primitives (`KojiGeometry::bbox()`, `geometry/koji_bbox.rs`, `koji_output::geojson_bbox`). `buildBaseLayers` already has an unused `editingGeofenceId` → dimmed-gray "ghost" treatment. `useGeoFeatures` (fetch-all) exists but is deliberately **not** reused here (it would load all 600).

## Locked decisions (from Q&A, 2026-07-14)

| Topic | Decision |
|---|---|
| Neighbor data | New backend **`?bbox=`** filter on `GET /api/v2/geofences` (fences intersecting a box). Client sends the current fence's padded bbox. |
| Project-map data | New backend **`?ids=`** filter on the same endpoint (only the requested fences). Serves show (saved ids) + create/edit (form-selected ids, reactive). |
| bbox filter impl | **In-memory** (server loads all rows, filters by computed bbox) — no migration now. A DB-column+index migration is spun off as a **follow-up chip**, not in this spec. |
| ids filter impl | **DB-level** (`Id.is_in(ids)`) — only the requested rows are read. |
| Engagement | Participate. |

## Non-goals

- No bbox-column migration / spatial index (follow-up chip).
- No change to the retired MapIndex / `/map` playground.
- No reuse of `useGeoFeatures` fetch-all for these surfaces (defeats the 600-fence concern).
- Project map shows **only** the project's member fences (no neighbor toggle on the project map).

## Backend — two scoped params on `GET /api/v2/geofences`

File: `crates/koji-service/src/public/v2/geofences.rs` (`ReadQuery` at ~:43, `list()` at ~:140). db reads: `crates/koji-db/src/db/geofence/reads.rs`.

- Add to `ReadQuery`: `ids: Option<String>` (comma-separated fence ids) and `bbox: Option<String>` (`"minLng,minLat,maxLng,maxLat"`).
- In `list()` (order: `ids` wins if both somehow present; document that only one is expected):
  - **`ids` present** → a new db read `get_koji_by_ids(db, &[u32])` mirroring `get_all_koji` but `Entity::find().filter(Id.is_in(ids))` — only those rows read, converted to the same FeatureCollection shape. Order/format identical to the default path.
  - **`bbox` present** → `get_all_koji` (all rows), then filter the resulting features in-memory: keep those whose geometry bbox intersects the query bbox (compute via `koji_core` bbox primitives; simple `Rect` overlap test). Accepted cost: reads all rows, returns only intersecting (payload narrowed server-side). `// ponytail: in-memory bbox filter; DB bbox columns + index if fence counts grow` with the follow-up chip referenced.
  - **neither** → unchanged (`get_all_koji`).
- Response envelope, `?format=` handling, and the `properties.name` + feature `id` on each feature are unchanged (the same `to_feature` path). Confirm the default featurecollection spec includes `name` (geofence/mod.rs:186 gates on `spec.properties.name`).
- Tests: an ids-filter test (returns only requested), a bbox-filter test (returns only intersecting), and neither-param test (returns all, back-compat). Rust unit/integration tests in the koji-service test suite; reuse the existing test-DB fixture.

## Frontend — shared primitive both features reuse

Everything below is one read-only interactive geofences overlay, wired onto two surfaces.

### Shared pieces

1. **`DeckMap` gets a `getTooltip` prop** (`components/deck/deck-map.tsx`) threaded to `<DeckGL getTooltip={getTooltip}>` (deck.gl core supports it natively; not exposed today). Signature `(info: PickingInfo) => { text: string } | null`. Back-compat: omitted → undefined → no tooltip (current behavior).

2. **`openGeofenceEdit(id)` util** (`map/lib/open-geofence.ts`): opens the fence's edit page in a new tab under the hash router —
   `window.open(\`${window.location.origin}${window.location.pathname}#/geofence/${id}\`, "_blank", "noopener")`.
   (Bare `#/geofence/{id}` resolves to the edit page per ra-core's route table.)

3. **Overlay layer builder** (`map/lib/geofence-overlay.ts`): `geofenceOverlayLayer({ features, ghost, excludeId })` → a dedicated `GeoJsonLayer` (its OWN `pickable` + `onClick`, separate from `buildBaseLayers`' shared-onClick geofences layer so clicks are unambiguous). `ghost:true` → dimmed gray fill/outline (reuse `EDITING_FILL`/`EDITING_LINE` values from layers.ts); `ghost:false` → normal orange. `excludeId` drops the current fence's own feature. `onClick` → `openGeofenceEdit(feature.id ?? feature.properties.id)`. Also exports a `overlayTooltip(info)` that returns `{ text: info.object.properties.name }` when `info.layer?.id` is the overlay layer, else null.

4. **Data hooks** (extend `map/data/use-geo-features.ts` or a sibling):
   - `useGeofencesByIds(ids: (number|string)[], enabled)` → `apiV2Fetch(\`/geofences?format=featurecollection&ids=${ids.join(",")}\`)`, react-query keyed on the sorted id list; returns `FeatureCollection` (unwrap `j.data`). `enabled:false` / empty ids → skip.
   - `useGeofencesByBbox(bbox: Bounds | null, enabled)` → `apiV2Fetch(\`/geofences?format=featurecollection&bbox=${bbox.join(",")}\`)`, keyed on the bbox; skip when `bbox` null or `!enabled`.

5. **`useNeighborOverlay(geometry, currentId)` hook** (`components/deck/use-neighbor-overlay.ts`), mirroring `useMarkerOverlay`: holds a default-off `on` toggle; computes the current fence's padded bbox (`geometryBounds(geometry)` padded ~20%); `useGeofencesByBbox(bbox, on && !!geometry)`; builds `geofenceOverlayLayer({ features, ghost:true, excludeId:currentId })`; returns `{ on, setOn, layers, getTooltip, label:"Neighbors" }`. Fetch gated on `on && geometry` (no bbox → no fetch; mirrors the marker toggle's hasGeom gating).

### Consumer surfaces

**Project map (member fences — normal style, NOT ghost):**
- New `ProjectGeofencesMap({ ids })` (`components/deck/project-geofences-map.tsx`): `useGeofencesByIds(ids, ids.length > 0)` → renders the features via `geofenceOverlayLayer({ features, ghost:false })` in a `DeckMap` (height 640, `expandable`, `getTooltip`), fit-bounds to the union of member geometries. Empty state (no ids) → a small placeholder (reuse DeckGeoJsonField's emptyText style) or hide.
- **project-show** (`resources/project/project-show.tsx`): keep the existing member chips, add `<ProjectShowMap />` — a thin wrapper reading `useRecordContext().geofences` → `<ProjectGeofencesMap ids={...} />`.
- **project-create / project-edit** (`resources/project/project-create.tsx` `ProjectFormFields`): add `<ProjectFormMap />` — a thin wrapper reading `useWatch({ name: "geofences" })` (reactive to the AutocompleteArrayInput) → `<ProjectGeofencesMap ids={...} />`, rendered below the geofences input.

**Geofence neighbor overlay (ghost style):**
- **geofence-show** (`FenceMapField` in `resources/geofence/geofence-show.tsx`): `useNeighborOverlay(record.geometry, record.id)`; pass its layers + `getTooltip` into the map and render a "Show Neighbors" toggle button. Requires `DeckGeoJsonField` to accept `extraLayers?: Layer[]`, `getTooltip?`, and an extra control slot (it already renders the marker toggle as a DeckMap child + its own layers) — add these generic props (DeckGeoJsonField stays neighbor-agnostic; FenceMapField owns the neighbor hook + toggle).
- **geofence edit / create** (`GeofenceMap` in `components/deck/geofence-map.tsx`): add a 5th toggle to the existing button row (`show.neighbors`), wire `useNeighborOverlay(watchedGeometry, editingId)`, merge its layers into `contextLayers`/`buildBaseLayers`, pass `getTooltip` to the map. On create there's no `editingId` (nothing to exclude) and geometry appears once drawn (gate as in the hook). The `editingId` on edit comes from the form record / route param.

## Component structure (recommended)

- `deck-map.tsx` — `getTooltip` prop [shared].
- `map/lib/open-geofence.ts` — new-tab edit opener [shared].
- `map/lib/geofence-overlay.ts` — overlay `GeoJsonLayer` builder + tooltip fn [shared].
- `map/data/use-geo-features.ts` — add `useGeofencesByIds`, `useGeofencesByBbox` [shared].
- `components/deck/use-neighbor-overlay.ts` — neighbor toggle hook [geofence pages].
- `components/deck/project-geofences-map.tsx` — `ProjectGeofencesMap` + `ProjectShowMap` + `ProjectFormMap` wrappers [project pages].
- `components/deck/deck-geojson-field.tsx` — add `extraLayers` / `getTooltip` / extra-control slot [geofence-show].
- `components/deck/geofence-map.tsx` — 5th "neighbors" toggle [geofence edit/create].
- Edits to `resources/project/{project-show,project-create}.tsx` and `resources/geofence/geofence-show.tsx`.

## Assumptions / defaults (review these)

- **Neighbor scope** = the current fence's bbox padded ~20%, fetched once (not viewport-following); excludes the current fence's own id.
- **Neighbor toggle** default **OFF** on all three geofence pages; on create, no fetch until geometry is drawn.
- **Styling**: neighbors dimmed-gray ghosts; project member fences drawn normal (they're the content, not context).
- **Hover** = native deck.gl `getTooltip` showing `properties.name`.
- **Click** (neighbors AND project members) = `window.open` new tab to the fence's edit page.
- **Project map**: show keeps chips + adds the map; create/edit add the map below the geofences input; fit to the union of member geometries; empty-state when no members.
- **Tooltip name key** = `properties.name` (verify the default featurecollection spec includes it).
- **`ids` query cap**: a project's member count is small; no pagination needed on `?ids=`.

## Testing

- Backend: Rust tests for `?ids=` (only requested), `?bbox=` (only intersecting), neither (all — back-compat).
- Frontend: Vitest browser tests for the shared pieces — `getTooltip` threads through DeckMap; `openGeofenceEdit` builds the right URL + calls `window.open` (spy); `geofenceOverlayLayer` ghost vs normal styling + excludeId; `useNeighborOverlay` toggle default-off + gated fetch; `ProjectGeofencesMap` renders member features + fits bounds; the neighbor toggle appears on each geofence page.
- deck.gl visuals (ghost dim, tooltip render, new tab) verified in Claude Preview where a backend + data are available; the real-Chromium component tests are the functional proof otherwise (established constraint: no local koji-server with seeded 600-fence data).

## Risks / notes

- **In-memory bbox filter** reads all fence rows per neighbor request (fine ≤ thousands; the browser payload is the thing narrowed). Migration = follow-up chip.
- **`ids` fan-out avoided**: the project map is one request (`?ids=`), not N `getOne` calls.
- **New-tab pop-up**: `window.open` from a user click (the map click handler) is a genuine user gesture, so it won't be pop-up-blocked.
- **Name property**: gated on `spec.properties.name` server-side — confirm the default featurecollection path sets it, else the tooltip is blank.
