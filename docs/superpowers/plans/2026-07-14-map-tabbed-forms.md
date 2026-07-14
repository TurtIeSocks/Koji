# Map edit/create → TabbedForm — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Convert the six map edit/create resource forms (geofence/route/project create+edit) to a `TabbedForm` — metadata on tab 1, a full-width viewport-height map on tab 2 — with a load-bearing `DeckMap` fix so maps in the initially-hidden tab frame at their real size.

**Architecture:** shadmin `TabbedForm.Tab` keeps all tabs mounted (`display:none` when inactive), so the tab-2 map mounts at 0×0. Fix `DeckMap` to defer its fit-once camera until the container has a non-zero size (fits when the tab is first shown). Each map component gains an optional `height` prop (default = current) so the Map tab can pass a viewport-fill height; show pages / other callers are untouched. `TabbedForm` is one RHF form, so the reactive maps keep working with no data-flow change.

**Tech Stack:** React 19, react-admin/shadmin-core, deck.gl v9 + maplibre, Vitest browser provider, Tailwind v4, bun.

**Spec:** `docs/superpowers/specs/2026-07-14-map-tabbed-forms-design.md`

## Global Constraints

- Scope: geofence create+edit, route create+edit, project create+edit ONLY. NOT the /import wizard, /map playground, or show pages.
- Tab labels "Details" (metadata) and "Map". Map tab: `contentClassName="p-0"` + a viewport-fill height.
- `height` props default to the component's CURRENT height so show pages and other callers don't change.
- `TabbedForm`/`TabbedForm.Tab` from `@/components/admin` (`components/admin/form/tabbed-form.tsx`). One shared RHF context across tabs.
- Frontend cmds: `cd apps/web && bun run test <file>` (unit), `bun run test:browser <file>` (Chromium), `bun run typecheck`. Browser tests clicking over the deck canvas must `import "@/index.css"`.
- Run heavy suites once per task end; background if >5s. Verify the final result in Claude Preview (tab switch → map fills viewport, correctly framed).

---

### Task 1: `DeckMap` deferred fit (fit only once container has a real size)

**Files:**
- Modify: `apps/web/src/components/deck/deck-map.tsx` (the fit-once `useState` initializer + `useLayoutEffect`/ResizeObserver, ~lines 63-108)
- Test: `apps/web/src/components/deck/deck-map.browser.test.tsx` (extend)

**Interfaces:**
- Produces: no API change. Behavior change — when `DeckMap` mounts inside a `display:none` (0×0) container, it does NOT compute its fit at a fallback size; it computes the fit-once when the container first has a non-zero size (via the ResizeObserver). Visible-at-mount maps are unchanged.

Current (approx) shape to change:
```tsx
const measure = () => {
  const r = el.getBoundingClientRect();
  sizeRef.current = { w: r.width || 800, h: r.height || 600 };  // <-- fallback hides "hidden"
  return sizeRef.current;
};
const { w, h } = measure();
setInitial((prev) => { if (prev) return prev; /* compute fit from w,h or DEFAULT_VS */ });
const ro = new ResizeObserver(measure);  // <-- never (re)computes the fit
```

- [ ] **Step 1 — Failing test.** In `deck-map.browser.test.tsx` (`import "@/index.css"`), render `<DeckMap layers={[]} fitBounds={aGeoJsonWithBounds} />` inside a wrapper `<div style={{ display: "none" }}>` (use a controllable style so the test can flip it). Mock `@deck.gl/react`'s default export (the file already does this) to capture whether `<DeckGL>` renders and with what `initialViewState`. Assert: while the wrapper is `display:none`, `DeckGL` is NOT rendered (the fit is deferred → `initial` stays null → `{initial ? <DeckGL/> : null}` renders nothing). Then flip the wrapper to `display:block` (visible, real size) and, after the ResizeObserver fires, assert `DeckGL` IS now rendered with a defined `initialViewState`. (Also keep the existing test: a DeckMap that is visible at mount renders `DeckGL` immediately.)
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser deck-map` (today the hidden map computes a fallback fit and renders DeckGL immediately → the "not rendered while hidden" assertion fails).
- [ ] **Step 3 — Implement.** Rework the effect: `measure()` returns the real rect and only writes `sizeRef` when `w>0 && h>0` (drop the `|| 800`/`|| 600` fallback). Extract a `fit()` closure that does the existing `setInitial((prev) => prev ? prev : (b ? {...DEFAULT_VS, ...boundsToViewState(b, sizeRef.current.w, sizeRef.current.h)} : DEFAULT_VS))`. At mount: `const {w,h} = measure(); if (w>0 && h>0) fit();`. ResizeObserver callback: `() => { const s = measure(); if (s.w>0 && s.h>0) fit(); }`. Keep the synchronous `useState` initializer as-is (initialViewState → set; fitBounds → null; else DEFAULT_VS). `fit()` stays idempotent via the `if (prev) return prev` guard.
- [ ] **Step 4 — Run, verify pass.** Then run the whole deck/map browser suite to confirm NO regression (every existing map fits as before): `bun run test:browser deck`. Typecheck.
- [ ] **Step 5 — Commit.** `fix(web): DeckMap defers fit until container has a real size (hidden-tab maps)`

---

### Task 2: Geofence create + edit → TabbedForm

**Files:**
- Modify: `apps/web/src/components/deck/geofence-map.tsx` (add optional `height` prop)
- Modify: `apps/web/src/resources/geofence/geofence-create.tsx`
- Modify: `apps/web/src/resources/geofence/geofence-edit.tsx`
- Test: `apps/web/src/resources/geofence/geofence-create.browser.test.tsx` (create/extend) + geofence-edit if a test exists

**Interfaces:**
- Consumes: `TabbedForm`, `TabbedForm.Tab` (`@/components/admin`); Task 1's DeckMap behavior.
- Produces: `GeofenceMap` accepts `height?: number | string` (default = its current value — find it in geofence-map.tsx, likely the `DeckGeoJsonInput`/`DeckMap` height ~640; thread `height` to that inner map, defaulting to the current constant).

- [ ] **Step 1 — Failing test.** Render `GeofenceCreate` via the resource/AdminContext harness (see an existing resource browser test for the harness; `import "@/index.css"`). Mock `@/components/deck` `GeofenceMap` to a stub (`data-testid="geofence-map-stub"`) so the test doesn't mount the deck stack. Assert: two tab triggers with names "Details" and "Map" render; the `name` input (`source="name"`) is present on the Details tab; clicking the "Map" tab reveals the `geofence-map-stub`. (Because tabs are `display:none`-hidden, both are in the DOM — assert the Map tab's panel becomes visible / not `aria-hidden` after click, and Details' inputs are under the Details panel.)
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser geofence-create`.
- [ ] **Step 3 — Implement.** `geofence-map.tsx`: add `height?: number | string` prop, thread to the inner map (default preserving current height). `geofence-create.tsx`: replace `<SimpleForm><GeofenceFormFields/><GeofenceMap/></SimpleForm>` with:
  ```tsx
  <TabbedForm>
    <TabbedForm.Tab label="Details"><GeofenceFormFields /></TabbedForm.Tab>
    <TabbedForm.Tab label="Map" contentClassName="p-0"><GeofenceMap height="calc(100dvh - 16rem)" /></TabbedForm.Tab>
  </TabbedForm>
  ```
  (Tune the `16rem` chrome offset in preview so there's no page scroll.) `geofence-edit.tsx`: same, with the edit-only projects input placed inside the Details tab alongside `<GeofenceFormFields/>` (it currently sits after the map — move it into Details).
- [ ] **Step 4 — Run, verify pass.** Typecheck. Confirm the geofence show page (which does NOT pass `height`) is unaffected — run its browser test.
- [ ] **Step 5 — Commit.** `feat(web): geofence create/edit use TabbedForm (details + full-width map)`

---

### Task 3: Route create + edit → TabbedForm

**Files:**
- Modify: `apps/web/src/components/deck/route-map.tsx` (add optional `height` prop)
- Modify: `apps/web/src/resources/route/route-create.tsx` (extract `RouteMetaFields`)
- Modify: `apps/web/src/resources/route/route-edit.tsx`
- Test: `apps/web/src/resources/route/route-create.browser.test.tsx` (create/extend)

**Interfaces:**
- Consumes: `TabbedForm`; Task 1's DeckMap behavior.
- Produces: `RouteMap` accepts `height?: number | string` (default = current, ~640); `RouteMetaFields` = the route metadata inputs (name/description/mode/geofence_id) split out of `RouteFormFields`.

- [ ] **Step 1 — Failing test.** Render `RouteCreate` (harness + `import "@/index.css"`); mock `RouteMap` to a stub. Assert: "Details" + "Map" tab triggers; `name` and the `geofence_id` reference input on the Details tab; clicking "Map" reveals the RouteMap stub.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test:browser route-create`.
- [ ] **Step 3 — Implement.** `route-map.tsx`: add `height?` prop threaded to its `DeckMap` (default preserving current 640). `route-create.tsx`: split `RouteFormFields` into `RouteMetaFields` (name/description/mode/geofence_id — everything except `<RouteMap/>`). `RouteCreate`:
  ```tsx
  <Create>
    <TabbedForm>
      <TabbedForm.Tab label="Details"><RouteMetaFields /></TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0"><RouteMap height="calc(100dvh - 16rem)" /></TabbedForm.Tab>
    </TabbedForm>
  </Create>
  ```
  Drop the `max-w-4xl`. `route-edit.tsx`: same TabbedForm structure using `RouteMetaFields` + `RouteMap`.
- [ ] **Step 4 — Run, verify pass.** Typecheck. Route show page uses `RouteShowMap` (not `RouteMap`) — unaffected, but run its test to confirm.
- [ ] **Step 5 — Commit.** `feat(web): route create/edit use TabbedForm (details + full-width calc map)`

---

### Task 4: Project create + edit → TabbedForm

**Files:**
- Modify: `apps/web/src/components/deck/project-geofences-map.tsx` (add optional `height` prop)
- Modify: `apps/web/src/resources/project/project-create.tsx` (extract `ProjectMetaFields`)
- Modify: `apps/web/src/resources/project/project-edit.tsx`
- Test: `apps/web/src/resources/project/project-form.test.tsx` (extend — it already tests `ProjectFormFields`) or a new browser test

**Interfaces:**
- Consumes: `TabbedForm`; Task 1's DeckMap behavior.
- Produces: `ProjectGeofencesMap` accepts `height?: number | string` (default 640 — it currently hardcodes 640; make it a prop). `ProjectMetaFields` = name/description/geofences `ReferenceArrayInput` split out of `ProjectFormFields`. Keep `ProjectFormMap`/`ProjectShowMap` wrappers.

- [ ] **Step 1 — Failing test.** Render `ProjectCreate` (harness); mock `ProjectGeofencesMap`/`ProjectFormMap` as in the existing project tests. Assert: "Details" + "Map" tab triggers; `name` + the geofences `ReferenceArrayInput` on Details; clicking "Map" reveals the project map stub. Keep the existing reactivity assertion (the map stub receives the current `geofences` ids) working across the tab structure.
- [ ] **Step 2 — Run, verify it fails.** `cd apps/web && bun run test project-form` (or `bun run test:browser project`).
- [ ] **Step 3 — Implement.** `project-geofences-map.tsx`: add `height?: number | string` prop (default 640) to `ProjectGeofencesMap`, thread to its `DeckMap`/placeholder height. `project-create.tsx`: split `ProjectFormFields` into `ProjectMetaFields` (name/description/geofences input) + keep `ProjectFormMap`. `ProjectCreate`:
  ```tsx
  <Create>
    <TabbedForm>
      <TabbedForm.Tab label="Details"><ProjectMetaFields /></TabbedForm.Tab>
      <TabbedForm.Tab label="Map" contentClassName="p-0"><ProjectFormMap height="calc(100dvh - 16rem)" /></TabbedForm.Tab>
    </TabbedForm>
  </Create>
  ```
  Have `ProjectFormMap` accept + forward a `height` prop to `ProjectGeofencesMap`. `project-edit.tsx`: same structure.
- [ ] **Step 4 — Run, verify pass.** Then run BOTH full suites once: `cd apps/web && bun run test` and `bun run test:browser`. Typecheck. Confirm the project SHOW page (`ProjectShowMap`, no height) still renders at 640.
- [ ] **Step 5 — Commit.** `feat(web): project create/edit use TabbedForm (details + full-width member map)`

---

## Preview verification (after Task 4)

In Claude Preview (dev server via `.claude/launch.json`): open each of the 6 pages → the form shows a "Details" tab (metadata) + a "Map" tab; the Map tab shows a full-width map that fills the viewport and is correctly framed (NOT stuck zoomed-out from a 0-size fit); editing a Details field (route's geofence_id, project's geofences) updates the Map tab. Tune the `calc(100dvh - Nrem)` offset so no double scrollbar appears. Mind the `document.hidden` preview trap for any visibility-gated timers.

## Self-Review (done)

- **Spec coverage:** DeckMap deferred-fit → T1; geofence forms → T2; route forms (+RouteMetaFields) → T3; project forms (+ProjectMetaFields) → T4; `height` props → T2/T3/T4; tab labels/`p-0`/viewport height → each form task. Import wizard/show pages excluded. ✅
- **Placeholder scan:** the `16rem` chrome offset is a preview-tuned value, flagged in each task + the verification section (not a hidden TODO). No other placeholders. ✅
- **Type consistency:** `height?: number | string` prop name consistent across GeofenceMap/RouteMap/ProjectGeofencesMap/ProjectFormMap (T2/T3/T4); `TabbedForm`/`TabbedForm.Tab` usage identical across tasks; `RouteMetaFields`/`ProjectMetaFields` defined in their own task and used only there. ✅
