# Koji User-Story Backlog

This backlog consolidates 6 mined slices into a single, deduped product backlog for Koji (main/production branch). Stories are grouped into cross-layer epics. Where the server and the client both touch a feature, it lives in one epic with the layer noted per story and in acceptance criteria.

## Personas

- **Map operator** — self-hosts Koji to manage geofences, routes, projects, and properties for their Pokemon-GO map.
- **Mapper** — draws and edits geofences and routes interactively on the map UI.
- **API consumer** — an external service or script calling the public `/api/v1` surface.
- **Scanner admin** — pushes geofences and routes from Koji into a downstream scanner DB (RDM / Golbat / Dragonite / Flygon / ReactMap) and triggers reload.
- **Self-hoster** — installs and configures Koji (Docker, env, tile servers, auth, reverse proxy).
- **Plugin author** — writes external clustering / routing / bootstrap plugins that Koji shells out to.

---

## Epic 1: Geofence Management (Server API)

### 1. Create a geofence via the admin API
**As a** Mapper **I want** to POST GeoJSON geometry with name, mode, and optional parent and properties **so that** new geographic boundaries are stored immediately in Koji.
**Acceptance criteria:**
- `POST /private/admin/geofence/` with `{name, geometry, mode, parent_id?, properties[]?}` returns the created model with `id`, `created_at`, `updated_at`.
- Name is unique across all geofences; a duplicate name returns an error.
- `mode` must be a valid `Type` enum value (e.g. `CirclePokemon`); an invalid mode returns 400.
- `parent_id`, if given, must reference an existing geofence or returns 404.
- Each property `{property_id, value}` in the array is upserted in `geofence_property`; a `name` database property is auto-tracked.

**Source:** `server/api/src/private/admin.rs` (create handler), `server/model/src/db/geofence.rs` (upsert)

### 2. Update a geofence
**As a** Mapper **I want** to PATCH a geofence's geometry, name, mode, parent, or properties **so that** I can refine it without losing related projects, routes, or property assignments.
**Acceptance criteria:**
- `PATCH /private/admin/geofence/{id}/` updates only the provided fields.
- Name uniqueness is re-validated; `updated_at` is set to now.
- If the name changes, related routes whose name matched the old name are auto-renamed.
- Properties array merges: existing updated, absent deleted, new inserted; geofence-project links survive.

**Source:** `server/api/src/private/admin.rs` (update handler), `server/model/src/db/geofence.rs` (`update_related_route_names`)

### 3. Retrieve one geofence with related data
**As a** Mapper **I want** a geofence by id or name with its projects, routes, and properties **so that** I can inspect its usage before editing.
**Acceptance criteria:**
- `GET /private/admin/geofence/{id}/` accepts numeric id or name; returns 404 if missing.
- Response includes model fields plus `projects[]`, `routes[]`, resolved `properties[]`, and parent name; raw geometry is excluded to keep the payload lean.

**Source:** `server/api/src/private/admin.rs` (get_one), `server/model/src/db/geofence.rs` (`get_one_json_with_related`)

### 4. Delete a geofence with cascade cleanup
**As a** Map operator **I want** deleting a geofence to also remove its routes and junction rows **so that** no orphaned data remains.
**Acceptance criteria:**
- `DELETE /private/admin/geofence/{id}/` returns 404 if missing, else a `rows_affected` count.
- `geofence_project`, `geofence_property`, and `route` rows for that geofence are removed via `ON DELETE CASCADE`.
- Subsequent GET returns 404.

**Source:** `server/api/src/private/admin.rs` (remove), `server/model/src/db/geofence.rs` (delete)

### 5. List geofences with pagination and filtering
**As a** Map operator **I want** a paginated, filterable, sortable geofence list **so that** I can manage a large inventory.
**Acceptance criteria:**
- Defaults: `page=0`, `per_page=25`, `sort_by=name`, `order=ASC`.
- Filters: `q` (LIKE name), `parent` (`0` = root only), `mode`, `geotype`, `project` (`0` = no projects).
- Response has `total`, `has_prev`, `has_next`, and `results[]` with related `projects/properties/routes`.
- `sort_by` supports model columns plus `projects.length` / `routes.length` / `properties.length`.

**Source:** `server/api/src/private/admin.rs` (paginate), `server/model/src/db/geofence.rs` (paginate)

### 6. Get geofence reference lists (all / parents / search)
**As an** API consumer **I want** lightweight geofence lookups **so that** dropdowns and cascades stay fast.
**Acceptance criteria:**
- `GET /private/admin/geofence/all/` returns id/name/mode/parent/geo_type, name-ordered, no geometry.
- `GET /private/admin/geofence/parent/` returns only geofences that are a parent of at least one child.
- `GET /private/admin/search/geofence/?query=` returns full models for case-insensitive `LIKE %query%`; empty query returns all.

**Source:** `server/api/src/private/admin.rs` (get_all, parent_list, search), `server/model/src/db/geofence.rs` (`get_json_cache`, `unique_parents`, `search`)

### 7. Assign a geofence's parent (hierarchy)
**As a** Mapper **I want** to set or clear a geofence's parent **so that** I can organize geofences into nested regions.
**Acceptance criteria:**
- `PATCH /private/admin/assign/geofence/parent/{id}/` with `{parent_id}`; `0` clears (NULL).
- `parent_id > 0` must reference an existing geofence (404 otherwise); self-assignment is rejected.
- A `parent` database property is auto-created in `geofence_property` if absent.

**Source:** `server/api/src/private/admin.rs` (assign), `server/model/src/db/geofence.rs` (assign)

---

## Epic 2: Route Management (Server API)

### 1. Create a route within a geofence
**As a** Mapper **I want** to POST a MultiPoint route tied to a geofence **so that** scanners have a waypoint sequence to patrol.
**Acceptance criteria:**
- `POST /private/admin/route/` with `{geofence_id, name, geometry (MultiPoint), mode, description?}` returns the model with a computed `points` count.
- `geofence_id` must reference an existing geofence (404 otherwise); invalid GeoJSON returns 400.
- `mode` must be a valid enum; name auto-prefixes to `{geofence} - {route}` when it collides with the geofence name.

**Source:** `server/api/src/private/admin.rs` (create), `server/model/src/db/route.rs` (create)

### 2. Update a route
**As a** Mapper **I want** to PATCH a route's geometry, name, mode, or description **so that** I can refine waypoints after creation.
**Acceptance criteria:**
- `PATCH /private/admin/route/{id}/` updates only provided fields; `geofence_id` is immutable.
- `points` is recomputed from the new geometry; `updated_at` set to now; invalid mode/geometry returns 400.

**Source:** `server/api/src/private/admin.rs` (update), `server/model/src/db/route.rs` (update)

### 3. Retrieve one route
**As a** Mapper **I want** route metadata by id or name **so that** I can verify the waypoint count before deployment.
**Acceptance criteria:**
- `GET /private/admin/route/{id}/` accepts id or name; 404 if missing; returns all fields except raw geometry.

**Source:** `server/api/src/private/admin.rs` (get_one), `server/model/src/db/route.rs` (`get_one_json`)

### 4. Delete a route
**As a** Map operator **I want** to delete an unused route **so that** stale routes don't accumulate.
**Acceptance criteria:**
- `DELETE /private/admin/route/{id}/` returns 404 if missing, else `rows_affected`; route is a leaf, no cascade.

**Source:** `server/api/src/private/admin.rs` (remove), `server/model/src/db/route.rs` (delete)

### 5. List routes with pagination and filtering
**As a** Map operator **I want** to filter routes by geofence, mode, and point-count range **so that** I can manage route inventory.
**Acceptance criteria:**
- Filters: `q`, `geofenceid`, `mode`, `pointsmin`/`pointsmax` (range); defaults match other resources.
- Response includes `total`, `has_prev`, `has_next`, `results[]`; geometry excluded.

**Source:** `server/api/src/private/admin.rs` (paginate), `server/model/src/db/route.rs` (paginate)

### 6. Get route reference lists (all / parents / search)
**As an** API consumer **I want** lightweight route lookups **so that** dependent selectors stay fast.
**Acceptance criteria:**
- `GET /private/admin/route/all/` returns summaries (geometry excluded, `geo_type` hardcoded `MultiPoint`).
- `GET /private/admin/route/parent/` returns geofences that own at least one route.
- `GET /private/admin/search/route/?query=` returns matching full models.

**Source:** `server/api/src/private/admin.rs` (get_all, parent_list, search), `server/model/src/db/route.rs`

---

## Epic 3: Project Management (Server API)

### 1. Create a project
**As a** Map operator **I want** to create a named project with optional scanner integration **so that** I can group geofences and configure push-to-prod.
**Acceptance criteria:**
- `POST /private/admin/project/` with `{name, description?, scanner?, api_endpoint?, api_key?}` returns the model.
- `scanner` defaults false; `api_endpoint`/`api_key` are optional downstream integration fields.

**Source:** `server/api/src/private/admin.rs` (create), `server/model/src/db/project.rs` (upsert)

### 2. Update a project
**As a** Map operator **I want** to PATCH name, description, scanner flag, or API credentials **so that** I can change integration settings.
**Acceptance criteria:**
- Only provided fields update; `updated_at` set to now; geofence links survive; toggling `scanner` enables/disables push-to-prod.

**Source:** `server/api/src/private/admin.rs` (update), `server/model/src/db/project.rs` (upsert)

### 3. Retrieve one project with linked geofences
**As a** Map operator **I want** a project with its linked geofence ids **so that** I can verify membership.
**Acceptance criteria:**
- `GET /private/admin/project/{id}/` accepts id or name; 404 if missing; returns model fields plus `geofences[]` (ids only).

**Source:** `server/api/src/private/admin.rs` (get_one), `server/model/src/db/project.rs` (`get_one_json_with_related`)

### 4. Delete a project (links only)
**As a** Map operator **I want** deleting a project to drop only its junction rows **so that** geofences survive.
**Acceptance criteria:**
- `DELETE /private/admin/project/{id}/` removes `geofence_project` rows for the project and the project row; linked geofences are NOT deleted; returns `rows_affected`.

**Source:** `server/api/src/private/admin.rs` (remove), `server/model/src/db/project.rs` (delete)

### 5. List / reference / search projects
**As a** Map operator **I want** paginated, all, and search project endpoints **so that** I can browse and select projects.
**Acceptance criteria:**
- `GET /private/admin/project/` paginates with `q`; `sort_by` supports `geofences.length`.
- `GET /private/admin/project/all/` returns full models, name-ordered.
- `GET /private/admin/search/project/?query=` returns case-insensitive matches.

**Source:** `server/api/src/private/admin.rs` (paginate, get_all, search), `server/model/src/db/project.rs`

---

## Epic 4: Geofence ↔ Project Linkage (Server API)

### 1. Link a geofence to a project
**As a** Mapper **I want** to create/update a junction record **so that** a geofence belongs to a project.
**Acceptance criteria:**
- `POST /private/geofence_project/` with `{geofence_id, project_id}` upserts (idempotent); both must exist (404 otherwise); returns the junction.

**Source:** `server/api/src/private/geofence_project.rs` (create), `server/model/src/db/geofence_project.rs` (create)

### 2. Unlink a geofence from a project
**As a** Mapper **I want** to delete a junction record **so that** I can unassign without deleting either side.
**Acceptance criteria:**
- `DELETE /private/geofence_project/` with `{geofence_id, project_id}` returns `rows_affected`; a missing junction yields 0 (no error).

**Source:** `server/api/src/private/geofence_project.rs` (remove), `server/model/src/db/geofence_project.rs` (delete)

### 3. List all junctions
**As an** API consumer **I want** every junction record **so that** I can audit or migrate assignments.
**Acceptance criteria:**
- `GET /private/geofence_project/all/` returns all `{id, geofence_id, project_id}`; no filtering/pagination.

**Source:** `server/api/src/private/geofence_project.rs` (get_all), `server/model/src/db/geofence_project.rs` (get_all)

### 4. Batch-replace links by geofence or by project
**As a** Mapper **I want** to replace all of a geofence's projects (or a project's geofences) in one call **so that** I can reassign without manual deletes.
**Acceptance criteria:**
- `PATCH /private/geofence_project/geofence/{id}/` and `.../project/{id}/` accept an id array; existing rows are deleted then new ones inserted.
- Empty array clears all links; each referenced id must exist (404 otherwise); returns 200, no payload.

**Source:** `server/api/src/private/geofence_project.rs` (update_by_id), `server/model/src/db/geofence_project.rs` (update_by_id)

---

## Epic 5: Property Template Management (Server API)

### 1. Create a property template
**As a** Map operator **I want** to define a typed property template **so that** geofences can carry validated custom metadata.
**Acceptance criteria:**
- `POST /private/admin/property/` with `{name, category, default_value?}` returns the model.
- `category` must be a valid enum (Database/Custom/Text/Number/Boolean); invalid returns 400.
- `(name, category)` is unique; Database properties are read-only (computed from geofence fields); `default_value` is the fallback when an instance value is NULL.

**Source:** `server/api/src/private/admin.rs` (create), `server/model/src/db/property.rs` (upsert)

### 2. Update a property template
**As a** Map operator **I want** to PATCH name, category, or default value **so that** I can refine a definition without recreating assignments.
**Acceptance criteria:**
- Only provided fields update; `(name, category)` uniqueness re-validated; invalid category returns 400; `updated_at` set to now. Category changes affect interpretation across all geofences using it (no cascade validation).

**Source:** `server/api/src/private/admin.rs` (update), `server/model/src/db/property.rs` (upsert)

### 3. Retrieve one property with assignments
**As a** Map operator **I want** a property plus the geofences using it **so that** I can understand its usage.
**Acceptance criteria:**
- `GET /private/admin/property/{id}/` accepts id or name; 404 if missing; returns model fields plus `geofences[]` (all `geofence_property` records).

**Source:** `server/api/src/private/admin.rs` (get_one), `server/model/src/db/property.rs` (`get_one_json`)

### 4. Delete a property template
**As a** Map operator **I want** deleting a property to drop its assignments only **so that** geofences survive.
**Acceptance criteria:**
- `DELETE /private/admin/property/{id}/` cascades `geofence_property` rows, deletes the property row, returns `rows_affected`; geofences are not deleted.

**Source:** `server/api/src/private/admin.rs` (remove), `server/model/src/db/property.rs` (delete)

### 5. List / reference / search properties
**As a** Map operator **I want** paginated, all, and search property endpoints **so that** I can manage custom fields.
**Acceptance criteria:**
- `GET /private/admin/property/` paginates with `q`; `sort_by` supports `geofences.length`.
- `GET /private/admin/property/all/` returns models name-ordered; `GET /private/admin/search/property/?query=` returns case-insensitive matches.

**Source:** `server/api/src/private/admin.rs` (paginate, get_all, search), `server/model/src/db/property.rs`

---

## Epic 6: TileServer Management (Server API)

### 1. Create a tile server
**As a** Self-hoster **I want** to register a tile-server endpoint **so that** the web UI can render those tiles.
**Acceptance criteria:**
- `POST /private/admin/tileserver/` with `{name, url}` returns the model with timestamps.

**Source:** `server/api/src/private/admin.rs` (create), `server/model/src/db/tile_server.rs` (upsert)

### 2. Update a tile server
**As a** Self-hoster **I want** to PATCH name or URL **so that** I can update sources without disruption.
**Acceptance criteria:**
- `PATCH /private/admin/tileserver/{id}/` updates only provided fields; `updated_at` set to now; no cascade.

**Source:** `server/api/src/private/admin.rs` (update), `server/model/src/db/tile_server.rs` (upsert)

### 3. Retrieve / delete a tile server
**As a** Self-hoster **I want** to read one and delete one tile server **so that** I can verify and remove configs.
**Acceptance criteria:**
- `GET /private/admin/tileserver/{id}/` accepts id or name; 404 if missing.
- `DELETE /private/admin/tileserver/{id}/` returns `rows_affected`; tile server is a leaf, no FK dependencies.

**Source:** `server/api/src/private/admin.rs` (get_one, remove), `server/model/src/db/tile_server.rs` (`get_one_json`, delete)

### 4. List / reference / search tile servers
**As a** Self-hoster **I want** paginated, all, and search tile-server endpoints **so that** I can manage multiple configs.
**Acceptance criteria:**
- `GET /private/admin/tileserver/` paginates with `q` on name; `all/` returns models name-ordered; `search/?query=` returns matches.

**Source:** `server/api/src/private/admin.rs` (paginate, get_all, search), `server/model/src/db/tile_server.rs`

---

## Epic 7: Admin Panel — Geofence UI (Client)

### 1. Browse, filter, and expand geofences
**As a** Mapper **I want** a paginated geofence table with sidebar filters and row expansion **so that** I can find geofences without loading everything.
**Acceptance criteria:**
- List columns: name, parent, mode, geo_type; pagination 25/50/100/500/1000; sortable; responsive.
- Sidebar filters (Project, Parent, Geography Type, Mode) load values dynamically and combine with AND; live name search.
- Expanding a row shows inline counts (projects/properties/routes) and the property name/value grid without reload.

**Source:** `client/src/pages/admin/geofence/GeofenceList.tsx`, `GeofenceFilter.tsx`, `GeofenceExpand.tsx`

### 2. Create a geofence in the admin panel (single + import wizard)
**As a** Mapper **I want** to create one geofence via a geometry code editor, or many via an import wizard **so that** I can author or migrate shapes.
**Acceptance criteria:**
- Create form: name (required), mode (scanner-specific), parent (optional), properties array; code editor accepts GeoJSON/WKT/etc.
- On blur, geometry is converted/validated server-side; an inline map preview renders on success; invalid geometry shows an error.
- "Create Multiple" opens the ImportWizard (file upload + preview + multi-format conversion); after import, redirects to the list with a success notice.

**Source:** `client/src/pages/admin/geofence/GeofenceCreate.tsx`, `GeofenceForm.tsx`, `CreateDialog.tsx`, `inputs/CodeInput.tsx`

### 3. Import geometry from multiple file formats (GeoJSON / WKT / CSV / shapefile)
**As a** Mapper **I want** the create form and import wizard to accept GeoJSON, WKT, CSV, and shapefile inputs **so that** I can migrate shapes from any upstream tool without manual reformatting.
**Acceptance criteria:**
- The geometry code editor and import wizard accept GeoJSON/WKT/CSV paste plus file uploads; a file-type variant is offered for each supported format (e.g. ShapeFile, JSON).
- Uploaded files are converted server-side to canonical GeoJSON, validated, and previewed on a map before save; an unsupported or malformed file shows an inline parse error.
- Multi-feature inputs are split into one geofence per feature with auto-assigned IDs.

**Source:** `client/src/pages/admin/geofence/GeofenceCreate.tsx`, `client/src/components/drawer/manage/ShapeFile.tsx`, `Json.tsx`

### 4. Edit / show a geofence with map preview
**As a** Mapper **I want** an edit form (with a Projects multi-select) and a read-only show page **so that** I can refine relationships or verify config.
**Acceptance criteria:**
- Edit form includes all create fields plus Projects; geometry code editor with live map preview; pessimistic save; stays on page after success.
- Show page displays name, mode, geo_type, properties as key/value, related projects as chips, map preview, and geometry JSON.

**Source:** `client/src/pages/admin/geofence/GeofenceEdit.tsx`, `GeofenceShow.tsx`, `GeofenceMap.tsx`

### 5. Add typed properties to a geofence
**As a** Mapper **I want** category-based property inputs on a geofence **so that** I can store metadata my scanner can consume.
**Acceptance criteria:**
- Properties is an add/remove array; each entry references a property definition (autocomplete grouped by category) plus a value.
- Value input type follows category: boolean→switch, string→text, number→number, color→picker, database→read-only.
- Default value pre-populates; removing an entry removes it from the geofence.

**Source:** `client/src/pages/admin/geofence/GeofenceForm.tsx`, `inputs/Properties.tsx`

### 6. Delete a geofence with undo
**As a** Map operator **I want** row delete with a ~5s undo window **so that** I can remove geofences without fear of permanent loss.
**Acceptance criteria:**
- Delete shows an undo toast for ~5s; undo restores; after timeout the delete commits to the server.
- (Note: Claude Preview runs offscreen `document.hidden=true`; undoable toasts won't auto-commit in preview — verify in a visible tab.)

**Source:** `client/src/pages/admin/geofence/GeofenceList.tsx`

---

## Epic 8: Admin Panel — Project / Route / Property / TileServer UI (Client)

### 1. Project CRUD in the admin panel
**As a** Map operator **I want** project list/create/edit/show/delete with scanner config **so that** I can organize geofences and target scanners.
**Acceptance criteria:**
- List columns: name, description, api_endpoint/api_key (flags), scanner, geofences count; real-time search; 25/50/100 pages.
- Create/edit forms: name, description, scanner toggle, api_endpoint, api_key, plus a grouped Geofences multi-select on edit; pessimistic save.
- Show page lists fields and related geofences as chips; delete has ~5s undo.

**Source:** `client/src/pages/admin/project/ProjectList.tsx`, `ProjectCreate.tsx`, `ProjectForm.tsx`, `ProjectEdit.tsx`, `ProjectShow.tsx`

### 2. Route CRUD in the admin panel
**As a** Mapper **I want** route list/create/edit/show/delete with map preview **so that** I can manage scanner routes per geofence.
**Acceptance criteria:**
- List with sidebar filters: Mode, Points ranges (0-1k…25k+), Geofence; live search; 25/50/100 pages.
- Create/edit: name, description, mode, geofence (required reference), read-only points count, MultiPoint code editor with marker+line map preview; pessimistic save.
- Show page renders points and geometry JSON; delete has ~5s undo.

**Source:** `client/src/pages/admin/route/RouteList.tsx`, `RouteFilter.tsx`, `RouteCreate.tsx`, `RouteForm.tsx`, `RouteEdit.tsx`, `RouteShow.tsx`

### 3. Property CRUD in the admin panel
**As a** Map operator **I want** property list/create/edit/show/delete with typed categories **so that** I can define reusable metadata fields.
**Acceptance criteria:**
- List columns: name, category, default_value, geofences count; search; 25/50/100 pages.
- Create form: name, category select (string/number/boolean/color/object/array/database); default-value input changes type by category; object/array disabled; database shows auto-populate info.
- Edit warns that changing category resets values to the new default on all geofences using it; show page includes created/updated timestamps; delete has ~5s undo.

**Source:** `client/src/pages/admin/property/PropertyList.tsx`, `PropertyCreate.tsx`, `PropertyForm.tsx`, `PropertyEdit.tsx`, `PropertyShow.tsx`

### 4. TileServer CRUD in the admin panel
**As a** Self-hoster **I want** tile-server list/create/edit/show/delete with a live tile preview **so that** I can manage map backgrounds.
**Acceptance criteria:**
- List columns: name, url; 25/50/100 pages.
- Create/edit: name, url; inline map preview renders tiles from the URL and errors if they fail; pessimistic save.
- Show page previews tiles; delete has ~5s undo.

**Source:** `client/src/pages/admin/tileserver/TileServerList.tsx`, `TileServerCreate.tsx`, `TileServerForm.tsx`, `TileServerEdit.tsx`, `TileServerShow.tsx`

---

## Epic 9: Admin Panel — Bulk Actions, Navigation & Layout (Client)

### 1. Bulk assign parent / projects (both directions)
**As a** Map operator **I want** bulk "Assign Parent", "Assign Project" (from geofences), and "Assign Geofence" (from projects) **so that** I can organize many records at once.
**Acceptance criteria:**
- "Assign Parent" dialog: parent autocomplete plus a "Remove" (id 0) option; updates all selected in parallel.
- "Assign Project"/"Assign Geofence" dialogs: multi-select with an info note that non-selected links are unassigned; PATCH to `/internal/admin/geofence_project/{geofence|project}/{id}/`.
- Success toast counts affected records; on error a failure toast shows and the list refreshes; selection and dialog clear.

**Source:** `client/src/pages/admin/actions/AssignParentFence.tsx`, `AssignProjectFence.tsx`, `GeofenceList.tsx`, `ProjectList.tsx`

### 2. Bulk delete and bulk/single export
**As a** Map operator **I want** bulk delete-with-undo and single/bulk GeoJSON export **so that** I can clean up or back up records.
**Acceptance criteria:**
- "Delete with Undo" deletes selected in parallel with a ~5s undo window, then commits; selection clears.
- Export (row, show, or bulk) fetches `/api/v1/{resource}/area/{id}` (query `name`/`parent`/`mode`) — projects use `/api/v1/geofence/feature-collection/{id}` — and opens a dialog to download a GeoJSON FeatureCollection.

**Source:** `client/src/pages/admin/actions/Export.tsx`, `GeofenceList.tsx`

### 3. Push (sync) geofences / routes / projects to scanner
**As a** Scanner Admin **I want** single and bulk "Sync" buttons on geofences, routes, and projects **so that** I can deploy to production scanners.
**Acceptance criteria:**
- Sync POSTs to `/api/v1/geofence/push/{id}`, `/api/v1/route/push/{id}`, or `/api/v1/project/push/{id}` (parallel for bulk).
- Success toast counts synced records (or names them for single); on error a failure toast shows and the list refreshes; selection clears.

**Source:** `client/src/pages/admin/actions/PushToApi.tsx`, `GeofenceList.tsx`, `RouteList.tsx`, `ProjectList.tsx`

### 4. Navigate the admin panel and toggle theme
**As a** Map operator **I want** a resource sidebar, top-bar quick links, and a theme toggle **so that** I can move between resources and contexts comfortably.
**Acceptance criteria:**
- Sidebar links Projects, Geofences, Routes, Properties, TileServers; current page highlighted; collapses on mobile.
- Top bar has Home (`/`), Map (`/map`), Info (docs in new tab), right-aligned; a light/dark theme toggle persists across reloads.

**Source:** `client/src/pages/admin/index.tsx`, `AppBar.tsx`

---

## Epic 10: Interactive Map — Drawing & Editing (Client / Geoman)

### 1. Draw polygons and route circles
**As a** Mapper **I want** to draw polygons (and cutouts/MultiPolygons) and chained route circles **so that** I can define boundaries and waypoint sequences.
**Acceptance criteria:**
- Polygon tool draws closed shapes auto-colored by ID; rectangle tool is an alternative; nesting creates MultiPolygon/cutout; Escape/Finish exits.
- Circle tool places Point waypoints, auto-linking forward/backward with line segments showing distance (m); Finish offers New Route / Cancel; points colored by route ID.

**Source:** `client/src/pages/map/interface/Drawing.tsx`

### 2. Edit, delete, cut, rotate, and merge shapes
**As a** Mapper **I want** edit/remove/cut/rotate/merge modes **so that** I can fine-tune shapes without redrawing.
**Acceptance criteria:**
- Edit mode drags vertices/points/whole shapes and updates features live (S2 coverage recalculated per dragged point).
- Removal mode batch-deletes by category (Lines/Circles/Polygons); cut mode slices a polygon with a drawn line; rotate mode spins a polygon around its center.
- Merge mode highlights selected polygons orange and combines them into one MultiPolygon (with a Merge All shortcut); cutouts kept per the "Keep Cutouts on Merge" setting.

**Source:** `client/src/pages/map/interface/Drawing.tsx`, `markers/Point.tsx`, `markers/Polygon.tsx`

### 3. Configure drawing behavior and preferences
**As a** Mapper **I want** snapping, continuous-draw, radius, route activation (hover/click), and line color settings **so that** drawing fits my workflow.
**Acceptance criteria:**
- Toggles for Snappable, Continue Drawing, Keep Cutouts on Merge; a Radius number input (disabled in S2 mode, default 70 m) sizes circles and feeds S2 coverage.
- Activate dropdown (hover/click) controls route highlighting; a line-color selector colors arrows/lines drawn afterward.

**Source:** `client/src/components/drawer/Drawing.tsx`, `client/src/pages/map/interface/Drawing.tsx`, `markers/Point.tsx`

---

## Epic 11: Interactive Map — Layers, Markers & S2 Visualization (Client)

### 1. Toggle shape and marker layers
**As a** Mapper **I want** to show/hide circles, lines, polygons, arrows, and data markers **so that** I can reduce clutter and validate placement.
**Acceptance criteria:**
- Layer toggles (Circles/Lines/Polygons/Arrows) persist to localStorage and have keyboard shortcuts.
- Marker toggles (gym/pokestop/spawnpoint/station) fetch only when enabled and the window is focused; Pixi.js renders in prod with a native-Leaflet dev fallback; query types: all/area/bound.

**Source:** `client/src/components/drawer/Layers.tsx`, `client/src/pages/map/markers/index.tsx`

### 2. Filter markers (last-seen, TTH, pokestop range, geohash color)
**As a** Mapper **I want** to filter and decorate markers **so that** I focus on recent, relevant data.
**Acceptance criteria:**
- A last-seen DateTime picker (local→UTC) filters and refreshes markers; spawnpoint TTH dropdown (All/Known/Unknown) filters by hatch status.
- A pokestop-range toggle draws non-editable 70 m circles at 20% opacity; dev-only geohash coloring colors markers by geohash at a chosen precision.

**Source:** `client/src/components/drawer/Layers.tsx`, `client/src/pages/map/markers/index.tsx`, `client/src/components/drawer/Settings.tsx`

### 3. Visualize and configure S2 cells
**As a** Mapper **I want** to display S2 cells (none/covered/all) and choose mode/levels/fill **so that** I can align geofences to S2 boundaries.
**Acceptance criteria:**
- Display dropdown none/covered/all; covered cells red with 20% fill, uncovered black 0%; dev-mode click copies cell ID.
- Mode Radius (multi-level select, e.g. 13–21) vs S2 (single level); fill simple (merged) vs all; cells fetched per current map bounds; changes update immediately.

**Source:** `client/src/pages/map/markers/S2.tsx`, `client/src/components/drawer/Layers.tsx`

---

## Epic 12: Interactive Map — Calculation, Navigation & Popups (Client)

### 1. Configure and run clustering / routing / bootstrap
**As a** Mapper **I want** to pick mode (Radius/S2), category, cluster algorithm, and sort-by, then run an Update **so that** I generate optimized geofences and routes from real data.
**Acceptance criteria:**
- Clustering tab exposes Mode, Category (gym/pokestop/spawnpoint/station/fort), Strategy, Radius, S2 level/size, cluster mode (Fast/Balanced/Better), Cluster Split Level, "Only unique", Sort By, Route Split Level, and plugin args.
- Update calls `clusteringRouting`; results render colored by cluster ID with line/arrow segments; bootstrap plugins appear alongside Radius/S2 with an args input.

**Source:** `client/src/components/drawer/Routing.tsx`, `client/src/pages/map/popups/Polygon.tsx`

### 2. Auto-calc thresholds, simplify, route index, marker scaling
**As a** Map operator **I want** per-POI max-area auto-calc thresholds and render toggles **so that** small areas calc instantly while large ones require confirmation and the map stays responsive.
**Acceptance criteria:**
- Settings has Max Area to Auto Calc (km²) for Pokestops/Gyms/Stations/Spawnpoints, persisted; geofence popups enable/disable auto-calc by threshold.
- Toggles for Simplify Polygons, Show Route Index, and Scale Markers adjust rendering.

**Source:** `client/src/components/drawer/Settings.tsx`, `client/src/pages/map/popups/Polygon.tsx`

### 3. Tune per-category Max-Area auto-calc thresholds with persistence
**As a** Map operator **I want** separate "Max Area to Auto Calc" thresholds per POI category (Pokestops, Gyms, Stations, Spawnpoints) saved across sessions **so that** dense categories calc instantly while sparse or large areas prompt before running.
**Acceptance criteria:**
- Settings exposes four independent km² inputs — one each for Pokestops, Gyms, Stations, Spawnpoints — persisted to storage and restored on reload.
- A geofence popup compares its computed area (km²) against the matching category threshold: at/under the threshold auto-calc runs immediately; over it the popup requires explicit confirmation before calculating.
- Changing a threshold takes effect on the next popup without a reload.

**Source:** `client/src/components/drawer/Settings.tsx`, `client/src/pages/map/popups/Polygon.tsx`

### 4. Navigate the map and inspect via popups
**As a** Mapper **I want** locate, URL coordinates, pan/zoom, and shape popups **so that** I can navigate and inspect coverage.
**Acceptance criteria:**
- A locate button centers on GPS (keeps zoom; error toast if denied); `/map?lat=&lon=&zoom=` initializes position; bounds/zoom persist to localStorage.
- Clicking a polygon shows area (km²), perimeter, point count, and calc buttons; clicking a point shows index/coords/geohash; markers show name/coords/group hash (dev shows geohash).

**Source:** `client/src/pages/map/interface/Locate.tsx`, `client/src/pages/map/index.tsx`, `client/src/pages/map/interface/index.tsx`, `popups/Polygon.tsx`, `popups/Point.tsx`

---

## Epic 13: Map — Import / Export & Conversion Tools (Client)

### 1. Import / export polygons and routes (GeoJSON)
**As a** Mapper **I want** to import/export polygons and routes as GeoJSON via dialogs **so that** I can back up, share, or migrate work.
**Acceptance criteria:**
- Manage tab has Export/Import for Polygons and Routes; export shows code with copy + download; import accepts paste or `.geojson/.json` upload.
- Invalid GeoJSON shows a parse error (via `safeParse`); valid features get auto IDs; route point order is preserved.

**Source:** `client/src/pages/map/index.tsx`, `client/src/components/dialogs/Polygon.tsx`, `Route.tsx`, `ImportExport.tsx`

### 2. Import from scanner / Koji DB and the import wizard
**As a** Mapper **I want** to load existing geofences from scanner or Koji DB, or run a guided wizard **so that** I can edit existing data without re-entry.
**Acceptance criteria:**
- Manage tab Instance Select loads scanner geofences (tagged `__SCANNER`, retain IDs for push-back); project + instance select loads Koji geofences (tagged `__KOJI`).
- Import Wizard walks source → name property → property retention (Combine by Name Key / Split MultiPolygons) → mode/project assignment → preview → Save to Koji.

**Source:** `client/src/components/drawer/manage/index.tsx`, `docs/app/client/getting-started/page.mdx`

### 3. Configure import-wizard property retention and MultiPolygon handling
**As a** Mapper **I want** the import wizard to expose property-retention modes (Combine by Name Key, Split MultiPolygons) across Koji-DB and Scanner-DB sources **so that** I control how duplicate names and multi-part shapes are merged or split during migration.
**Acceptance criteria:**
- The wizard offers multi-source import (Koji DB project/instance, Scanner DB instance) with the source tag retained (`__KOJI` / `__SCANNER`) for correct push-back.
- "Combine by Name Key" merges features sharing the chosen name property into one record; "Split MultiPolygons" emits one geofence per polygon part; the two modes are selectable per import.
- A preview reflects the chosen retention before Save to Koji; mode/project assignment applies to all imported features.

**Source:** `client/src/components/drawer/manage/index.tsx`, `docs/app/client/getting-started/page.mdx`

### 4. Convert and transform in the Convert dialog and Playground
**As an** API consumer **I want** to transform geofence data between formats with name rules and test it live **so that** output matches my scanner's conventions before integrating.
**Acceptance criteria:**
- Convert dialog: paste GeoJSON, pick a format (featureCollection/feature_vec/poracle/rdm/geometry/etc.), preview on a map, copy output; supports geometry-type conversion and property filtering.
- Playground (`/playground`): pick project + return type, toggle Properties (id/name/mode/parent/group/fullcoords) and Params (trimstart/trimend/alphanumeric/replace), see the live URL and JSON; a "Names Only" toggle restricts formats to featureCollection/feature_vec/poracle.

**Source:** `client/src/components/dialogs/Convert.tsx`, `client/src/pages/Convert.tsx`, `client/src/pages/Playground.tsx`

---

## Epic 14: Global UX, Auth & Landing (Client)

### 1. Log in and log out (UI)
**As a** Self-hoster **I want** a password login page and a Settings logout **so that** only authorized users access my instance.
**Acceptance criteria:**
- Login posts `{password}` to `/config/login`; correct → home, incorrect → error that clears on retype; show/hide toggle; Enter submits.
- Settings Logout clears the session and redirects to `/login`.

**Source:** `client/src/pages/Login.tsx`, `client/src/components/drawer/Settings.tsx`

### 2. Landing page and cross-section navigation
**As a** Map operator **I want** a home page linking Map/Admin/Convert/Play and an Admin button on the map **so that** I can reach all sections quickly.
**Acceptance criteria:**
- Home shows four button cards (Map, Admin, Convert, Play) with hover effects and a theme toggle.
- The map interface has a bottom-right Admin Panel button that navigates to `/admin`.

**Source:** `client/src/pages/Home.tsx`, `client/src/pages/map/interface/index.tsx`

### 3. Theme, shortcuts, tile server, drawer, notifications, network status
**As a** Mapper **I want** theming, customizable shortcuts, tile-server selection, drawer toggle, notifications, and a connectivity indicator **so that** the app fits my environment and keeps me informed.
**Acceptance criteria:**
- Theme toggle (persisted, with map base-layer swap); Keyboard Shortcuts dialog (capture/clear/reset per action); TileServer dropdown from `/internal/admin/tileserver/all/` (persisted, cycle shortcut).
- Drawer toggle (animated, mini-icons, Escape closes); notifications with severity (success auto-dismiss ~3s, warning/error persist); a network-status indicator with retry; loading screen toggle.

**Source:** `client/src/components/ThemeToggle.tsx`, `client/src/components/dialogs/Keyboard.tsx`, `client/src/components/drawer/Settings.tsx`, `client/src/components/drawer/index.tsx`, `client/src/components/notifications/General.tsx`, `NetworkStatus.tsx`

### 4. Surface development-only ("dangerous") features behind a server flag
**As a** Self-hoster **I want** dev-only map tools (e.g. S2 cell-ID click-to-copy, geohash marker coloring/precision, geohash in popups) to appear only when the server's `DANGEROUS` flag is enabled **so that** experimental controls stay hidden in production.
**Acceptance criteria:**
- The client reads the `dangerous` flag from `GET /config/` (sourced from the server `DANGEROUS` env var) and gates dev-only UI on it.
- When `dangerous` is true, S2 cell click-to-copy, geohash-based marker coloring with a precision control, and geohash fields in point/marker popups are available; when false they are hidden.
- The flag is resolved once at load and does not require a code change to toggle.

**Source:** `server/api/src/private/misc.rs`, `server/.env.example`, `client/src/pages/map/markers/S2.tsx`, `client/src/components/drawer/Settings.tsx`

### 5. Error pages and resilient failures
**As a** Mapper **I want** a 404 page, an error boundary, and clear calc/import errors **so that** failures don't crash the app.
**Acceptance criteria:**
- Unknown routes show a 404 with a Back button; an ErrorBoundary wraps the map to catch React errors and keep it interactive.
- Calculation failures surface a clear notification (e.g. "No points within geofence") and allow retry.

**Source:** `client/src/pages/Error.tsx`, `client/src/components/ErrorBoundary.tsx`

---

## Epic 15: Public API — Format Conversion & Geometry Ops (Server)

### 1. Convert area geometry to any return format
**As an** API consumer **I want** `POST /api/v1/convert/data` with a `return_type` **so that** I get my area in the exact shape my system needs.
**Acceptance criteria:**
- Accepts a GeoFormat `area` plus optional `return_type`; supports 14+ formats: SingleStruct, MultiStruct, Text (`,`/`\n`), AltText (` `/`,`), SingleArray, MultiArray, Geometry/GeometryVec, Feature/FeatureVec, FeatureCollection, Poracle/PoracleSingle, Sql.
- Coordinates trim to 6 decimals; `simplify:true` simplifies polygons (tol 0.0001); internal properties stripped; response wraps `{status, message, data}`.

**Source:** `server/api/src/public/v1/convert.rs`, `server/api/src/utils/response.rs`

### 2. Apply name-property modifiers to converted output
**As an** API consumer **I want** a full suite of name transforms via `ApiQueryArgs` **so that** exported geofence names match my scanner's naming conventions without post-processing.
**Acceptance criteria:**
- Supported modifiers include `trimstart`/`trimend` (numeric char trim), `alphanumeric`, `replace`, capitalize/case transforms, parent-name prefix/suffix, Polish→ASCII transliteration, and space/dash/underscore replacements.
- Modifiers compose deterministically in the documented order and apply across all name-bearing return types (featureCollection/feature_vec/poracle, etc.).
- With no modifiers set, names pass through unchanged; an invalid modifier value is ignored or rejected without corrupting other fields.

**Source:** `server/model/src/api/args.rs`, `server/model/src/utils/mod.rs` (`name_modifier`)

### 3. Filter conversion output with structural query flags
**As an** API consumer **I want** `excludeproperties`, `excludeparents`, `exclude` (comma-separated), `ignoremanualparent`, and `fullcoords` flags **so that** I can shape exactly which fields and relationships appear in the response.
**Acceptance criteria:**
- `excludeproperties` strips resolved properties; `excludeparents` omits parent linkage; `exclude` drops a comma-separated list of named geofences from the result.
- `ignoremanualparent` ignores a manually-assigned parent during resolution; `fullcoords` emits full-precision coordinates rather than trimmed.
- Flags accept documented aliases and combine; unset flags leave the default response shape intact.

**Source:** `server/model/src/api/args.rs`

### 4. Simplify and merge geometry
**As a** Mapper **I want** `/convert/simplify` and `/convert/merge-points` **so that** I can reduce vertices or consolidate points into a MultiPoint.
**Acceptance criteria:**
- `POST /api/v1/convert/simplify` applies Douglas-Peucker (0.0001) to (Multi)Polygons; other geometry types pass through; output trims to 6 decimals.
- `POST /api/v1/convert/merge-points` collects all Point geometries into one MultiPoint Feature (CirclePokemon type), skipping non-Points.

**Source:** `server/api/src/public/v1/convert.rs`

---

## Epic 16: Route Generation — Clustering, Bootstrap, Rerouting (Server)

### 1. Cluster points with a chosen algorithm
**As a** Mapper **I want** Fastest (UDC), Fast/Balanced (greedy), Better/Best (S2-cell), Honeycomb, S2, or a custom plugin **so that** I trade speed against coverage quality.
**Acceptance criteria:**
- Fastest uses plane-projected UDC (fast, non-optimal); Fast/Balanced use r-tree greedy over data-point + segment candidates (Balanced adds radius×2 + wiggle); Better/Best generate candidates at S2 level-22 cells via RegionCoverer + Rayon.
- Honeycomb fills a hex grid independent of data; S2 mode makes each cell a cluster; custom mode `custom(plugin)` pipes points as JSON to a plugin and returns empty on failure.
- All modes apply `min_points` filtering, dedupe via HashSet, and respect `max_clusters`.

**Source:** `server/algorithms/src/clustering/` (`fastest.rs`, `greedy.rs`, `s2.rs`, `mod.rs`, `plugin.rs`)

### 2. Bootstrap an area (Radius / S2 / custom)
**As a** Map operator **I want** to fill a geofence with circles, S2 cell centers, or a custom plugin **so that** I cover unmapped area for initial data collection.
**Acceptance criteria:**
- Radius mode lays a hex grid of circles; S2 mode emits cell centers at `s2_level` (default 15), Rayon-parallel over polygons, cells expanded 0.1° for boundaries.
- Area is resolved by instance name or parent id; optional `sort_by` routing; stats (cluster_time, total_clusters, distance) tracked; optional `save_to_db`/`save_to_scanner` with project reload.
- Custom `calculation_mode=custom(plugin)` passes the GeoJSON feature + args; failure logs and returns empty.

**Source:** `server/api/src/public/v1/calculate.rs`, `server/algorithms/src/bootstrap/` (`radius.rs`, `s2.rs`, `mod.rs`)

### 3. Reroute existing clusters without reclustering
**As a** Mapper **I want** to re-sort existing cluster points with a different strategy **so that** I iterate on route order quickly.
**Acceptance criteria:**
- Accepts clusters + optional data_points + `sort_by` + `route_split_level` + radius; applies routing then `rotate_to_best`; cluster_time skipped.
- If clusters are empty, data_points are used as centers (legacy compatibility).

**Source:** `server/api/src/public/v1/calculate.rs` (reroute), `server/algorithms/src/routing/mod.rs`

### 4. Run calculations in benchmark mode (stats only, no persistence)
**As a** Map operator **I want** a `benchmark_mode` flag that runs clustering/bootstrap and returns timing and stats only, skipping all DB and scanner saves **so that** I can compare algorithms without side effects on production data.
**Acceptance criteria:**
- When `benchmark_mode` is set, the calc runs the full pipeline but suppresses `save_to_db`/`save_to_scanner`/project reload regardless of those flags.
- The response carries stats (cluster_time, total_clusters, distance, and applicable score metrics) with the result geometry, but commits nothing.
- Default (flag unset) preserves normal save behavior.

**Source:** `server/api/src/public/v1/calculate.rs`, `server/model/src/api/args.rs`

---

## Epic 17: Routing Strategies & Cluster Tuning (Server)

### 1. Sort clusters by strategy (S2 / geohash / point-count / lat-lon / random / custom)
**As a** Mapper **I want** to order clusters by a chosen sort strategy **so that** scanner movement is efficient or load-balanced.
**Acceptance criteria:**
- S2Cell sorts by CellID (Hilbert order); GeoHash sorts 12-char encodings lexically; PointCount sorts descending by r-tree counts; LatLon sweeps N-E→S-W; Random shuffles with seed 42 (deterministic).
- Custom `Custom(plugin)` pre-sorts via S2, pipes clusters to the plugin (with optional join), and falls back to S2 order on failure; all built-ins parallelize via Rayon.

**Source:** `server/algorithms/src/routing/sorting.rs`, `routing/mod.rs`, `routing/join.rs`

### 2. Tune clustering (split level, max clusters, centering, rotate)
**As a** Map operator **I want** split-level parallelism, a max-cluster cap, SEC centering, and best-start rotation **so that** clustering respects my constraints and data.
**Acceptance criteria:**
- `cluster_split_level>0` groups points by S2 cell and clusters per cell across threads; `max_clusters` halts greedy early (unavailable in Fastest).
- `center_clusters=true` recenters via Welzl smallest-enclosing-circle (falls back on failure, tracks success/fail counts); `rotate_to_best` picks the start cluster minimizing total distance.
- Genetic post-processing exists as a disabled stub (logs a warning if enabled).

**Source:** `server/algorithms/src/clustering/greedy.rs`, `sec/mod.rs`, `sec/sec.rs`, `utils.rs`, `clustering/genetic.rs`

### 3. Reserve genetic post-processing as a documented, disabled stub
**As a** Map operator **I want** the `genetic_post_processing` option to exist as a clearly-disabled stub (server field + commented-out UI control) **so that** the future algorithm slot is reserved and visible without misleading me that it works today.
**Acceptance criteria:**
- The `genetic_post_processing` field is present in `Args`; enabling it runs no optimization and logs a warning that the feature is disabled.
- The corresponding control in the Routing drawer is commented out / inert, documented as reserved for future use.
- Toggling the flag never alters results or throws, preserving backward compatibility.

**Source:** `server/algorithms/src/clustering/genetic.rs`, `server/model/src/api/args.rs`, `client/src/components/drawer/Routing.tsx`

### 4. Compute route and area statistics
**As a** Scanner admin **I want** distance, coverage, and area metrics **so that** I can judge route quality before deployment.
**Acceptance criteria:**
- `route-stats`/`route_stats_category` return total/longest/avg distance (Haversine) and, with data points, per-cluster coverage histograms, best/worst counts, and `mygod_score = clusters*min_points + (total-covered)` (lower better).
- Data points may load from DB by area + category + last_seen + TTH; overlap dedup via HashSet; `points_covered > total` logs a warning.
- `calculate_area` sums Chamberlain-Duquette area (m²) across features.

**Source:** `server/api/src/public/v1/calculate.rs`, `server/algorithms/src/stats.rs`, `rtree/mod.rs`

---

## Epic 18: S2 Geometry & Coverage (Server API)

### 1. Compute S2 coverage (circle / grid / bounds / polygons)
**As an** API consumer **I want** S2 endpoints for circle, grid, bounds, and cell-polygon coverage **so that** I can validate and visualize S2 geofencing.
**Acceptance criteria:**
- `circle-coverage` builds a 60-point Haversine circle and recursively adds intersecting S2 neighbors (thread-safe set); `s2_grid` builds an N×N grid via 8-neighbor expansion (Chebyshev rings).
- `s2_cells` (bounds) returns exact-level cells with 4-corner coords, optional id filter, capped at 100,000; `cell_polygons` converts id strings to quad coords (Rayon, skips bad ids).

**Source:** `server/api/src/public/v1/s2.rs`, `server/algorithms/src/s2.rs`

### 2. Expand S2 grids and circle coverage via neighbor traversal
**As an** API consumer **I want** the grid and circle-coverage endpoints to expand cells through documented neighbor traversal **so that** I get predictable, gap-free S2 coverage around a seed cell or circle.
**Acceptance criteria:**
- `s2_grid` builds an N×N block by expanding from a center cell across 8-neighbor Chebyshev rings, producing a contiguous square of equal-level cells.
- `circle-coverage` seeds from a 60-point Haversine circle and recursively adds any S2 cell intersecting the circle, tracked in a thread-safe set to avoid duplicates.
- Both stop at the requested extent/level and skip malformed or out-of-range cells without aborting the whole response.

**Source:** `server/api/src/public/v1/s2.rs`, `server/algorithms/src/s2.rs`

---

## Epic 19: Scanner Integration & Push-to-Production (Server API)

### 1. Push geofences / routes / projects to the scanner and reload
**As a** Scanner admin **I want** push endpoints that upsert to the scanner DB and trigger a reload **so that** generated data is immediately live.
**Acceptance criteria:**
- `GET /api/v1/geofence/push/{id}`, `/route/push/{id}`, `/project/push/{id}` fetch the feature(s), upsert to the scanner DB, and return `{updates, inserts}`.
- ScannerType `Unown` uses `area::Query`, others use `instance::Query`; project push respects `project.scanner` (skips DB insert if false) but always sends the reload.
- The reload calls `project.api_endpoint`; an `api_key` containing `:` splits into user/pass (Basic auth for normal scanners, custom-header auth for Unown/non-scanner); errors are logged, not fatal.

**Source:** `server/api/src/public/v1/geofence.rs`, `route.rs`, `project.rs`, `server/api/src/utils/request.rs`

### 2. Detect scanner type and route to the correct controller DB
**As a** Self-hoster **I want** Koji to auto-detect the scanner backend (RDM / Hybrid / Unown) and route queries to the right database **so that** push and instance/area lookups work without me hand-wiring each backend.
**Acceptance criteria:**
- Scanner type is detected from the database struct/schema; `Unown` resolves instances via `area::Query` while RDM/Hybrid use `instance::Query`.
- Controller-routed backends prefer `CONTROLLER_DB_URL` and fall back to `SCANNER_DB_URL` when the controller DB is unset.
- Detection failure or an unknown schema is logged and degrades gracefully rather than panicking.

**Source:** `server/model/src/utils/mod.rs` (`get_database_struct`, scanner_type detection), `server/api/src/public/v1/geofence.rs`

### 3. Bulk save to scanner or Koji
**As a** Scanner admin / Mapper **I want** bulk save-scanner and save-koji endpoints **so that** I can deploy or persist many features at once.
**Acceptance criteria:**
- `POST /api/v1/geofence/save-scanner` (and route save-scanner) upserts a FeatureCollection to the scanner DB and triggers configured project reloads.
- `POST /api/v1/route/save-koji` (and geofence save-koji) upserts to the Koji DB only with no scanner API call; both log row counts and return `{updates, inserts}`.

**Source:** `server/api/src/public/v1/geofence.rs`, `route.rs`

---

## Epic 20: Public Data Queries & Instance Lookup (Server API)

### 1. Query data points (all / bound / area) by category
**As an** API consumer **I want** to fetch gyms/pokestops/spawnpoints/stations by full DB, bounds, or area **so that** I can analyze or cluster real data.
**Acceptance criteria:**
- `POST /api/v1/data/all/{category}` filters by enabled/deleted/`updated > last_seen`, capped at 2,000,000; spawnpoint accepts `tth` (All/Known/Unknown).
- `POST /api/v1/data/bound/{category}` filters by lat/lon BETWEEN bounds; `POST /api/v1/data/area/{category}` filters by point-in-polygon from an `area` or `instance` (errors if both empty).

**Source:** `server/api/src/private/points.rs`, `server/model/src/db/{gym,pokestop,spawnpoint,station}.rs`

### 2. Filter spawnpoint queries by TTH (Tried-To-Hatch) status
**As an** API consumer **I want** an optional `tth` parameter (All/Known/Unknown) on spawnpoint data queries **so that** I cluster or analyze only spawnpoints with the hatch-timing status I need.
**Acceptance criteria:**
- Spawnpoint `all`/`area`/`stats` endpoints accept `tth`: `Known` returns spawnpoints with a known despawn timer, `Unknown` those without, `All` (default) returns both.
- The filter composes with the other data filters (enabled/deleted/last_seen, area/instance) and is reflected in returned counts and stats.
- Non-spawnpoint categories ignore `tth` without error.

**Source:** `server/api/src/private/points.rs`, `server/model/src/db/spawnpoint.rs`

### 3. Aggregate area stats for a category
**As a** Scanner admin **I want** total counts for a category within an area **so that** I can size workload before clustering.
**Acceptance criteria:**
- `POST /api/v1/data/area_stats/{category}` accepts `area` or `instance`, returns a Stats object (`total`, …), logs the total; errors if both inputs empty.

**Source:** `server/api/src/private/points.rs`

### 4. List and fetch instances from Koji or scanner
**As a** Map operator / Mapper **I want** merged Koji lists, scanner instance lists, and single-feature fetch **so that** I can browse and load geometry.
**Acceptance criteria:**
- `GET /internal/routes/from_koji` merges geofences + routes into `NameTypeId[]`; `from_scanner` switches on ScannerType (Unown→`area::Query`, else `instance::Query`).
- `GET /internal/routes/one/{koji|scanner}/{id}/{instance_type}` returns a GeoJSON Feature; circle_* instance types resolve to routes, others to geofences.

**Source:** `server/api/src/private/instance.rs`

### 5. Public geofence read endpoints in many formats
**As an** API consumer **I want** `GET /api/v1/geofence/{return_type}[/{project}]` **so that** I can export geofences in the format and scope my integration needs.
**Acceptance criteria:**
- Supports 17+ return types (feature-collection, poracle, geometry, text, array, struct, sql, …) and optional project filter.
- Query flags control output: `internal`, `id`, `name`, `mode`, `geofence_id`, `parent`, `group`, `fullcoords`; aliases accepted.

**Source:** `docs/app/api-reference/endpoints/page.mdx`, `server/api/src/public/v1/geofence.rs`

---

## Epic 21: Authentication, Server Info & Place Search (Server)

### 1. Authenticate public (bearer/session) and private (session-only) routes
**As an** API consumer / Self-hoster **I want** documented auth rules **so that** I can call the right surface securely.
**Acceptance criteria:**
- `/api/v1/*` allows a logged-in session, or any request when `KOJI_SECRET` is empty, or `Authorization: Bearer {KOJI_SECRET}`; else 401 with `WWW-Authenticate`.
- `/internal/*` requires `logged_in=true` only and ignores bearer tokens.
- `POST /config/login` checks `password == KOJI_SECRET`; `GET /config/logout` clears the session and redirects.

**Source:** `server/api/src/utils/auth.rs`, `server/api/src/private/misc.rs`

### 2. Discover server config and algorithm options
**As an** API consumer / Self-hoster **I want** `/api/v1/info/`, `/api/v1/health`, and `/config/` **so that** I can monitor health and discover capabilities.
**Acceptance criteria:**
- `GET /api/v1/info/` returns `routing`/`clustering`/`bootstrap` plugin-name arrays; `GET /api/v1/health` returns 200; both are public (no auth).
- `GET /config/` returns start_lat/lon, tile_server, scanner_type, logged_in, dangerous, and the three plugin arrays from env + session.

**Source:** `server/api/src/public/v1/info.rs`, `server/api/src/private/misc.rs`

### 3. Search and reverse-geocode place names (Nominatim)
**As a** Mapper **I want** forward and reverse Nominatim geocoding **so that** I can find boundaries or label coordinates.
**Acceptance criteria:**
- `GET /config/nominatim?query=` searches OSM (address_details, dedupe, limit 50), keeps only (Multi)Polygons, logs skipped non-polygons, and returns a FeatureCollection.
- A Nominatim reverse client (lat/lon/zoom/address/name details) exists, though no public reverse endpoint is exposed yet.

**Source:** `server/api/src/private/misc.rs`, `server/nominatim/src/reverse.rs`

### 4. Maintain the Nominatim reverse-geocoding client for future use
**As a** Self-hoster **I want** the internal `nominatim::ReverseClient` (lat/lon/zoom/address/name details) kept functional and tested even though no `/config/nominatim-reverse` endpoint is exposed yet **so that** reverse labeling can be wired up later without rebuilding the client.
**Acceptance criteria:**
- `ReverseClient` accepts lat/lon plus zoom and address/name detail flags and returns a parsed reverse-geocode result.
- The client is covered by tests and compiled into the build, but no public reverse route is registered (documented as deferred).
- Exposing a future `/config/nominatim-reverse` endpoint requires only wiring the existing client, not reimplementing it.

**Source:** `server/nominatim/src/reverse.rs`

---

## Epic 22: Public API Calculation Endpoints (Server / Docs)

### 1. Bootstrap, cluster, and route via the public API
**As an** API consumer **I want** `/api/v1/calc/bootstrap`, `/calc/cluster/{category}`, and `/calc/route/{category}` **so that** I can run end-to-end optimization programmatically.
**Acceptance criteria:**
- Bootstrap accepts area (or instance/parent), radius, `calculation_mode` (radius/s2/plugin), s2_level/size, optional save flags, and a `benchmark_mode` that returns stats only.
- Cluster accepts instance/data_points, radius, min_points, `cluster_mode` (fast/balanced/better/plugin), clustering_args, split level; categories gym/pokestop/spawnpoint/fort.
- Route clusters then routes via OR-Tools TSP (default) or a `sort_by` plugin, honoring `route_split_level` and `routing_args`.

**Source:** `docs/app/api-reference/endpoints/page.mdx`, `body/page.mdx`, `server/api/src/public/v1/calculate.rs`

### 2. Save, delete, and validate via the public API
**As an** API consumer **I want** save-koji, save-scanner, delete, route-stats, and S2 endpoints **so that** I can manage and validate data over HTTP.
**Acceptance criteria:**
- `POST /api/v1/geofence/save-koji` upserts (insert-if-new, update-by-name); `save-scanner` also triggers reloads; `DELETE /api/v1/geofence/{id}` removes by numeric id.
- `POST /api/v1/calc/route-stats` returns total/longest/avg distance from clusters; `POST /api/v1/s2/*` returns circle/cell/polygon coverage and bounds cells.
- Auth header is required when `KOJI_SECRET` is set.

**Source:** `docs/app/api-reference/endpoints/page.mdx`, `server/api/src/public/v1/`

---

## Epic 23: Installation, Deployment & Configuration (Docs)

### 1. Deploy via Docker or standard install
**As a** Self-hoster **I want** Docker-Compose and standard build paths **so that** I can stand up Koji my preferred way.
**Acceptance criteria:**
- Docker: edit `SCANNER_DB_URL`/`KOJI_DB_URL`/`KOJI_SECRET`, `docker-compose pull && up -d`, reachable quickly; update via pull/down/up without data loss; ghcr.io login documented.
- Standard: install Node/Rust/curl, run `or-tools/install.sh`, build client (`yarn build`) and server (`cargo install --path .`), optional PM2; update workflow documented.

**Source:** `docs/app/setup/docker/page.mdx`, `docs/app/setup/standard/page.mdx`

### 2. Configure env vars and reverse-proxy limits
**As a** Self-hoster **I want** documented env vars and proxy settings **so that** I avoid silent misconfiguration and timeouts.
**Acceptance criteria:**
- Required (`SCANNER_DB_URL`, `KOJI_DB_URL`, `KOJI_SECRET`) and optional vars (CONTROLLER_DB_URL, MAX_CONNECTIONS, LOG_LEVEL, START_LAT/LON, NOMINATIM_URL, TILE_SERVER, DANGEROUS) documented with deprecated aliases.
- Nginx `client_max_body_size` (50 MB cap) and `proxy_*_timeout` examples given; Cloudflare's 100s limit and the direct-IP/local-API workaround warned.

**Source:** `docs/app/setup/environment/page.mdx`, `docs/app/setup/extra/page.mdx`

### 3. Tune database pooling and logging verbosity
**As a** Self-hoster **I want** clear documentation for `MAX_CONNECTIONS` and `LOG_LEVEL` **so that** I can size the connection pool to my DB and control log noise.
**Acceptance criteria:**
- `MAX_CONNECTIONS` (u32, default 100) is documented as the upper bound of the DB connection pool, with guidance on matching it to the database's own connection limit.
- `LOG_LEVEL` accepts `error`/`warn`/`info`/`debug`/`trace` (documented default and effect of each); invalid values fall back to the default.
- Both are listed in the environment reference with their types and defaults.

**Source:** `server/model/src/utils/mod.rs`, `server/.env.example`, `docs/app/setup/environment/page.mdx`

### 4. Document deprecated env-var aliases and the upgrade path
**As a** Self-hoster **I want** the deprecated env-var aliases documented **so that** I can upgrade an older config without silent breakage.
**Acceptance criteria:**
- The docs list each alias and its replacement: `DATABASE_URL`→`SCANNER_DB_URL`, `UNOWN_DB_URL`→`CONTROLLER_DB_URL`, `UNOWN_DB`→`CONTROLLER_DB_URL`.
- The backward-compatibility layer still honors the old names but logs a deprecation warning at startup, noted in the docs.
- The upgrade guide recommends migrating to the new names and explains the warning users will see until they do.

**Source:** `server/model/src/utils/mod.rs`, `docs/app/setup/environment/page.mdx`

---

## Epic 24: Getting Started & Documented Workflows (Docs)

### 1. Log in and create a distribution project
**As a** Map operator **I want** a getting-started flow for login and project creation **so that** scanners can fetch geofences via a stable named URL.
**Acceptance criteria:**
- Login uses `KOJI_SECRET` (same token for API and UI); landing page exposes Admin and Map.
- Admin → Projects → Create sets name and optional scanner flag plus a refresh endpoint; assigned geofences become available at `/api/v1/geofence/feature-collection/{project}`; only one scanner project allowed.

**Source:** `docs/app/client/getting-started/page.mdx`

### 2. Import existing geofences and run a Dragonite bootstrap loop
**As a** Mapper / Scanner admin **I want** documented import and bootstrap→sync→re-run workflows **so that** I can migrate data and discover spawnpoints.
**Acceptance criteria:**
- The import wizard ingests ReactMap areas.json, Poracle geofence.json, shapefile, or Nominatim; steps cover name property, property retention, and mode/project assignment.
- The Dragonite loop: save geofence, bootstrap (Radius/70/no-sort) with save flags, Sync, adjust workers, then re-run the Pokemon route after data accumulates and push back.

**Source:** `docs/app/client/getting-started/page.mdx`

### 3. Reference recommended map settings
**As a** Mapper **I want** four pre-tuned calculation configs **so that** I skip trial-and-error.
**Acceptance criteria:**
- Documented configs for Area Bootstrap, Spawnpoint Clustering (Balanced + TSP, min_points 3), Pokestop Quest Scan (Better + TSP, min_points 1, 80 m), Fort Scanning (S2 + TSP), each in a parameter table with a tuning tip.

**Source:** `docs/app/client/map/page.mdx`

---

## Epic 25: Scanner Backend Integrations (Docs)

### 1. Integrate with ReactMap / PoracleJS / Golbat / Dragonite / rdmGruber
**As an** API consumer **I want** per-backend config recipes **so that** each scanner fetches geofences from Koji and reloads on change.
**Acceptance criteria:**
- Each backend documents its bearer token (`KOJI_SECRET`), fetch URL (feature-collection or poracle endpoint), and Koji project refresh endpoint + API-key header (e.g. `x-golbat-secret`, `X-Poracle-Secret`, ReactMap secret, Dragonite `/reload`).
- Polygon vs MultiPolygon support is noted per backend (Dragonite down-converts; Poracle partial; rdmGruber none); RDM is flagged deprecated.
- Dragonite covers same-network and cross-network (Admin proxy + `X-Dragonite-Admin-Secret`).

**Source:** `docs/app/integrations/page.mdx`

### 2. Configure cross-network Dragonite via the Admin proxy
**As a** Scanner admin **I want** documented cross-network Dragonite setup using the Admin proxy and `X-Dragonite-Admin-Secret` header **so that** I can trigger reloads when Koji and Dragonite live on different networks.
**Acceptance criteria:**
- The docs distinguish same-network (direct `/reload`) from cross-network (Admin proxy) Dragonite setups, with the endpoint and required `X-Dragonite-Admin-Secret` header for the latter.
- Auth handling is explained: an `api_key` containing `:` splits into user/pass for Basic auth on normal scanners, while Unown/admin scenarios use a custom-header scheme.
- The recipe states which Koji project fields hold the proxy endpoint and secret.

**Source:** `docs/app/integrations/page.mdx`, `server/api/src/utils/request.rs`

### 3. Sync geofences/routes to scanner and trigger refresh (docs)
**As a** Scanner admin **I want** documented sync behavior **so that** I can deploy from Koji to production without manual DB edits.
**Acceptance criteria:**
- A scanner project carries an API endpoint + key (templates for RDM/Dragonite/Golbat); Sync saves to the scanner DB and POSTs the refresh endpoint automatically.
- Editing a geofence/route in Koji can auto-sync when the `save_to_scanner` flag is set.

**Source:** `docs/app/client/admin/page.mdx`, `docs/app/client/notes.mdx`

---

## Epic 26: Plugin System & Extensibility (Docs)

### 1. Write custom clustering / routing / bootstrap plugins
**As a** Plugin author **I want** documented stdin/stdout contracts per plugin type **so that** I can add specialized algorithms in any language.
**Acceptance criteria:**
- Clustering plugins (`clustering/src/plugins/`) read space-separated `lat,lon` pairs, get `--radius/--min_points/--max_clusters` plus `clustering_args`, and emit `lat,lng` pairs; used via `cluster_mode={plugin}`.
- Routing plugins (`routing/src/plugins/`) read cluster centers + `routing_args` and emit ordered points; used via `sort_by={plugin}`.
- Bootstrap plugins (`bootstrapping/src/plugins/`) read a GeoJSON Feature + `--radius` + `bootstrapping_args` and emit points; used via `calculation_mode={plugin}`. Supported runners: bash/node/tsx/python3/executable.

**Source:** `docs/app/plugins/page.mdx`, `docs/app/plugins/examples/page.mdx`

### 2. Formalize the plugin stdin/stdout serialization and error contract
**As a** Plugin author **I want** the per-type argument-passing and serialization contract spelled out as a testable spec (input shape, output shape, error/success criteria) **so that** I can implement and validate a plugin without reading Koji's source.
**Acceptance criteria:**
- Each plugin type's input is documented exactly: clustering/routing receive coordinate pairs on stdin plus typed flags (`--radius`, `--min_points`, `--max_clusters`, `--split-level`) and the relevant `*_args`; bootstrap receives a GeoJSON Feature plus `--radius` and args.
- Output format is documented per type (coordinate/point pairs or ordered points), and a non-zero exit, malformed output, or empty stdout is treated as failure → Koji logs the error and returns an empty result rather than crashing.
- Success criteria (well-formed parseable output) are stated so a plugin author can write a conformance test against them.

**Source:** `server/algorithms/src/clustering/plugin.rs`, `server/api/src/public/v1/calculate.rs`, `docs/app/plugins/page.mdx`

### 3. Use and debug the external TSP plugin (tsp-mt)
**As a** Self-hoster / Plugin author **I want** templates and the tsp-mt recipe **so that** I can swap in a faster/LKH router.
**Acceptance criteria:**
- Python/JS templates show arg parsing (`--radius`, `--split-level`) and stdin reading; a C++ OR-Tools example serves as a compiled reference.
- tsp-mt: clone, `cargo build --release --features fetch-lkh`, copy the binary into `routing/plugins/`, set `sort_by=tsp-mt` (or rename to `tsp` for Dragonite compatibility), optional `--lkh-exe` via `routing_args`.

**Source:** `docs/app/plugins/external/page.mdx`, `docs/app/plugins/examples/page.mdx`

---

## Epic 27: Algorithm Reference & Docs-as-Product (Docs)

### 1. Understand clustering algorithms and the mygod_score
**As a** Mapper **I want** algorithm explanations, benchmarks, and the score formula **so that** I can pick the right speed/quality tradeoff.
**Acceptance criteria:**
- Docs rank Fastest (~0.02s, lowest quality) → Fast → Balanced (recommended) → Better (~8s, S2, best) with timing and score benchmarks and rendered comparison images.
- `mygod_score = clusters*min_points + (total - covered)` is explained (lower better, excludes runtime); parameter guidance (radius, min_points, max_clusters, split level, center) provided.

**Source:** `docs/app/algorithms/clustering/page.mdx`, `docs/app/client/map/page.mdx`

### 2. Navigate, copy examples, and get help from the docs site
**As a** Self-hoster / API consumer **I want** searchable docs, copyable cURL/JS examples, an endpoint reference, and support links **so that** I can self-serve onboarding and integration.
**Acceptance criteria:**
- Nextra docs offer full-text search, a sectioned sidebar, breadcrumbs, and card links; the endpoints page tables each method/path/description with body and query-param references.
- Copy-friendly cURL/JS examples include bearer-token handling and placeholders; a Help page links GitHub Issues and Discord; a Development page documents `cargo watch`, the client `yarn dev`, and docs dev server.

**Source:** `docs/app/page.mdx`, `docs/app/api-reference/endpoints/page.mdx`, `scripts/page.mdx`, `docs/app/help/page.mdx`, `docs/app/development/page.mdx`

---

## Coverage map

| Epic | Stories | Layers |
|------|---------|--------|
| 1. Geofence Management (Server API) | 7 | server |
| 2. Route Management (Server API) | 6 | server |
| 3. Project Management (Server API) | 5 | server |
| 4. Geofence ↔ Project Linkage (Server API) | 4 | server |
| 5. Property Template Management (Server API) | 5 | server |
| 6. TileServer Management (Server API) | 4 | server |
| 7. Admin Panel — Geofence UI | 6 | client |
| 8. Admin Panel — Project/Route/Property/TileServer UI | 4 | client |
| 9. Admin Panel — Bulk Actions, Nav & Layout | 4 | client (+ server endpoints) |
| 10. Interactive Map — Drawing & Editing | 3 | client |
| 11. Interactive Map — Layers, Markers & S2 | 3 | client (+ server data) |
| 12. Interactive Map — Calc, Nav & Popups | 4 | client (+ server calc) |
| 13. Map — Import/Export & Conversion | 4 | client (+ server convert) |
| 14. Global UX, Auth & Landing | 5 | client (+ server flag) |
| 15. Public API — Format Conversion & Geometry | 4 | server |
| 16. Route Generation — Clustering/Bootstrap/Reroute | 4 | server |
| 17. Routing Strategies & Cluster Tuning | 4 | server |
| 18. S2 Geometry & Coverage | 2 | server |
| 19. Scanner Integration & Push-to-Prod | 3 | server |
| 20. Public Data Queries & Instance Lookup | 5 | server |
| 21. Auth, Server Info & Place Search | 4 | server |
| 22. Public API Calculation Endpoints | 2 | server + docs |
| 23. Installation, Deployment & Config | 4 | docs |
| 24. Getting Started & Documented Workflows | 3 | docs (+ client) |
| 25. Scanner Backend Integrations | 3 | docs (+ server) |
| 26. Plugin System & Extensibility | 3 | docs (+ server) |
| 27. Algorithm Reference & Docs-as-Product | 2 | docs |
| **Total** | **27 epics / 107 stories** | server + client + docs |