# Koji Admin V2 — Slice 3 (Geofence completion: actions & references) — Contract

Frontend-only. Builds on slices 1–2. **The template is the existing code** — mirror it:
- geofence resource: `apps/web/src/resources/geofence/*` ; project resource (for ReferenceArrayInput): `apps/web/src/resources/project/project-create.tsx` (`<ReferenceArrayInput source="geofences" reference="geofence"><AutocompleteArrayInput/>`)
- dataProvider + `internalFetch`: `apps/web/src/data-provider.ts`, `apps/web/src/lib/http.ts`
- old-admin behavior reference (do NOT copy MUI; copy the BEHAVIOR): `apps/web-client/src/pages/admin/actions/{PushToApi,AssignParentFence,AssignProjectFence}.tsx`, `geofence/GeofenceForm.tsx`

All endpoints already forward through `/internal` (no backend work). Endpoints:
- Publish: `POST /internal/geofences/{id}/publish` and `POST /internal/routes/{id}/publish` → `{...}` 202, or **422** (no linked Dragonite area) → show a notification, don't treat as a hard crash.
- Assign parent: `PATCH /internal/geofences/{id}` body `{ "parent": <geofenceId|null> }`.
- Assign projects: `PATCH /internal/geofences/{id}` body `{ "projects": [<projectId>...] }`.
- Geofence edit projects: `PATCH /internal/geofences/{id}` body includes `projects: number[]` (the ReferenceArrayInput source).

## 1. projects ReferenceArrayInput on geofence Edit

Add to the geofence Edit form (`geofence-edit.tsx` / the shared form fields used by edit — NOT create, matching the old admin where projects-assign is edit-only):
`<ReferenceArrayInput source="projects" reference="project"><AutocompleteArrayInput/></ReferenceArrayInput>`.
(Mirror project's `geofences` ref-array exactly. Keep it out of Create.) Geofence Show: add a `<ReferenceArrayField source="projects" reference="project"><SingleFieldList><ChipField source="name"/></SingleFieldList></ReferenceArrayField>`.

## 2. Publish action (geofence + route)

A `PublishButton` (single-record, for the row/edit toolbar) + a `BulkPublishButton` (bulk-action toolbar) at `apps/web/src/components/actions/publish-button.tsx`:
- Single: `internalFetch(`/${seg}/${id}/publish`, { method:"POST" })` where seg = geofences|routes (derive from `useResourceContext()` via the same RESOURCE_MAP segment mapping, or pass a `segment` prop). On 2xx → `notify("Published", {type:"info"})` + `refresh()`. On **422** → `notify(<message>, {type:"warning"})` (the record has no linked Dragonite area). On other error → `notify(error, {type:"error"})`.
- Bulk: loop the selected ids through the single publish call (Promise.allSettled), notify a summary, `unselectAll()` + `refresh()`.
- Wire `PublishButton` into geofence + route List row actions (or the edit TopToolbar) and `BulkPublishButton` into their `<DataTable>` bulk-action slot.
- Reuse ra-core hooks: `useNotify`, `useRefresh`, `useResourceContext`, `useListContext`/`useUnselectAll` (bulk), `useRecordContext` (single).

## 3. Assign-parent bulk action (geofence)

`AssignParentBulkButton` (`apps/web/src/components/actions/assign-parent-bulk.tsx`) in geofence's bulk-action toolbar:
- Opens a shadcn `<Dialog>` with a `<ReferenceInput source="parent" reference="geofence"><AutocompleteInput/></ReferenceInput>` (inside a minimal `<Form>`), plus a "Clear parent" option (sets parent=null).
- On confirm → `dataProvider.updateMany("geofence", { ids: selectedIds, data: { parent: <chosenId|null> } })` (or loop `update` if updateMany's single-body semantics don't fit — the dataProvider has `updateMany` doing per-id PATCH with the same body, which is exactly right here). Then `notify`, `unselectAll`, `refresh`, close dialog.

## 4. Assign-projects bulk action (geofence)

`AssignProjectsBulkButton` (`apps/web/src/components/actions/assign-projects-bulk.tsx`), same dialog pattern:
- `<ReferenceArrayInput source="projects" reference="project"><AutocompleteArrayInput/></ReferenceArrayInput>` to pick projects.
- On confirm → `updateMany("geofence", { ids: selectedIds, data: { projects: <chosenIds> } })` → notify/unselectAll/refresh/close.
- Note (old-admin caveat): this is a full-replace of each geofence's `projects` (destructive); label the dialog accordingly.

## 5. Constraints (inherit slices 1–2)

- bun; `apps/web`; `@/`; shadmin (ra-core 5.14) vendored. dataProvider hits `/internal` only.
- Browser tests STUB the dataProvider (msw is node-only); jsdom for pure logic. Run browser suites at end of each task; cold Chromium ~100s → background.
- Reuse the vendored shadcn `Dialog`/`Button` (`@/components/ui/*`) + ra-core `Form` (`react-hook-form` via shadmin).
- TDD, bite-sized, conventional commits, on `claude/v2`.
- **DEFERRED to a later slice:** the geofence properties array-input (category-typed per-row value via per-row property lookup) — the complex piece.
