# Map edit/create pages → TabbedForm (metadata tab + full-width map tab)

**Date:** 2026-07-14
**Branch:** claude/v2
**Status:** Approved (approach), pending spec review

## Context

The six resource edit/create forms that embed a map currently render a `SimpleForm` with the metadata inputs and the map stacked vertically (the map cramped beside/below the fields). Convert each to a **`TabbedForm`**: tab 1 = the metadata inputs, tab 2 = a **full-width, viewport-height** map. `TabbedForm` already exists in the shadmin kit (`apps/web/src/components/admin/form/tabbed-form.tsx`).

**In scope (6 forms):** geofence create + edit, route create + edit, project create + edit.
**Out of scope:** the `/import` wizard (a custom 4-step flow, not a SimpleForm), the `/map` playground, all show pages.

## Locked decisions (Q&A, 2026-07-14)

| Topic | Decision |
|---|---|
| Scope | The 6 resource forms only; import wizard excluded. |
| Map tab height | Fill the viewport (map dominates tab 2, a `dvh`-relative height minus app chrome). |
| Hidden-tab fit | REQUIRED companion fix to `DeckMap` (below) — not optional. |
| Engagement | Participate. |

## The critical gotcha — maps in an initially-hidden tab

`TabbedForm.Tab` keeps **all tabs mounted**, hiding inactive ones with `display:none` (tabbed-form.tsx:149,195). So the map on tab 2 mounts inside a `display:none` (0×0) container while tab 1 is active. `DeckMap`'s fit-once logic (`deck-map.tsx`) measures the container in a layout effect and, crucially, falls back to `r.width || 800` / `r.height || 600` — so a hidden (0×0) container yields an **800×600** fit, and the fit-once guard (`if (prev) return prev`) then locks that wrong-sized camera in. When tab 2 is first shown at its real (large) size, the camera is already fixed for 800×600 → the map opens mis-zoomed. (Same failure family as the project-map null-island bug just fixed.)

**Fix (`deck-map.tsx`):** defer the initial fit until the container has a **non-zero** size.
- `measure()` no longer falls back to 800/600; it returns the real rect and only updates `sizeRef` when `w>0 && h>0`.
- Extract the fit computation into a `fit()` that reads `sizeRef` and is guarded by the existing `if (prev) return prev` (fits at most once).
- At mount: `const {w,h} = measure(); if (w>0 && h>0) fit();` — visible maps fit immediately (unchanged behavior).
- In the `ResizeObserver` callback: `const s = measure(); if (s.w>0 && s.h>0) fit();` — a map that mounted hidden fits the first time it becomes visible, at its real size.

This is a general robustness improvement (any hidden→shown map) and is verified by a `deck-map` browser test.

## Design

### Tab structure (per form)

```
<TabbedForm>
  <TabbedForm.Tab label="Details"><metadata inputs/></TabbedForm.Tab>
  <TabbedForm.Tab label="Map" contentClassName="p-0"><TheMap fill/></TabbedForm.Tab>
</TabbedForm>
```

- **Tab 1 "Details"** — the existing metadata inputs (already separated in geofence; extracted from the combined FormFields in route/project).
- **Tab 2 "Map"** — the existing map component, made to fill the tab: full width (natural, no side fields) + a viewport-relative height so it's as large as possible.
- Labels: "Details" + "Map" for all six (consistent). Keep `TabbedForm`'s defaults; if hash-router tab-path syncing (`syncWithLocation`) misbehaves for create routes, set `syncWithLocation={false}` (verify in preview).

### Per-form refactor

- **geofence-create** (`geofence-create.tsx`): `GeofenceFormFields` is already metadata-only (name/mode/parent/properties). `GeofenceCreate` becomes `TabbedForm` with tab1 = `<GeofenceFormFields/>`, tab2 = `<GeofenceMap/>`.
- **geofence-edit** (`geofence-edit.tsx`): tab1 = `<GeofenceFormFields/>` + the edit-only projects input (currently after the map), tab2 = `<GeofenceMap/>`.
- **route-create/edit**: `RouteFormFields` currently bundles metadata + `<RouteMap/>`. Split: extract `RouteMetaFields` (name/description/mode/geofence_id); create + edit render `TabbedForm` with tab1 = `<RouteMetaFields/>`, tab2 = `<RouteMap/>`. Drop the `max-w-4xl` (the map gets a full tab now).
- **project-create/edit**: `ProjectFormFields` bundles metadata (name/description/geofences `ReferenceArrayInput`) + `<ProjectFormMap/>`. Split: `ProjectMetaFields` (the three inputs) on tab1, `<ProjectFormMap/>` on tab2. Keep `ProjectFormMap`/`ProjectShowMap` wrappers as-is (the reactive `useWatch` still works — one shared form context across tabs).

### Map fill

The map components hard-code their height today (GeofenceMap/RouteMap via their inner `DeckMap`/`DeckGeoJsonInput` ~640; `ProjectGeofencesMap` 640). Give each an optional `height?: number | string` prop (default = current value, so show pages / other callers are unchanged) and pass a viewport-relative height on the Map tab (e.g. `height="calc(100dvh - <chrome>)"` or a `dvh` value — pick a value that fills below the app header + tab bar + form toolbar without page scroll; the `/map` playground already uses `100dvh` with these components as precedent). `contentClassName="p-0"` on the map tab removes the default tab padding so the map is truly full-bleed within the tab.

- `ProjectGeofencesMap` already takes no height prop → add `height` (default 640); the form tab passes the viewport height, the show page keeps 640.
- `GeofenceMap`/`RouteMap` take no props today → add an optional `height` prop threaded to their inner map, default preserving current behavior.

### Shared form context (no data-flow change)

`TabbedForm` is a single RHF form; both tabs share the context. So `RouteMap` (`useFormContext`/`useWatch`), `GeofenceMap` (`useWatch geometry`), and `ProjectFormMap` (`useWatch geofences`) keep working exactly as before — editing a field on tab 1 still drives the map on tab 2. Validation runs across all tabs (all mounted). No dataProvider/API change.

## Component structure

- `deck-map.tsx` — deferred-fit fix [shared, load-bearing].
- `components/deck/{geofence-map,route-map,project-geofences-map}.tsx` — add optional `height` prop.
- `resources/geofence/{geofence-create,geofence-edit}.tsx` — TabbedForm.
- `resources/route/{route-create,route-edit}.tsx` — extract `RouteMetaFields`, TabbedForm.
- `resources/project/{project-create,project-edit}.tsx` — extract `ProjectMetaFields`, TabbedForm.

## Testing

- `deck-map` browser test: a map mounted in a `display:none` container that is then shown fits at the real size (not the 800×600 fallback) — assert the fit only resolves once a non-zero size is observed. (Simulate: render with the container hidden, then reveal, assert the deck initial view state / that `fit` didn't lock a fallback camera.)
- Per-form browser tests: assert both tab triggers ("Details", "Map") render; the metadata inputs are on tab 1; switching to the Map tab renders the `deck-map` container; a field edited on tab 1 (e.g. `geofence_id` for route) still drives the map. Mock the heavy map internals where a full mount is unnecessary (mirror the existing resource tests' mocking).
- Run unit + browser suites at the end; verify in Claude Preview (tab switch → map fills the viewport and is correctly framed).

## Assumptions / defaults (review these)

- Tab labels "Details" / "Map" for all six.
- Map tab uses a viewport-fill height; exact `dvh`/`calc` value tuned in preview to avoid page scroll.
- `height` props default to current values so show pages and any other callers are untouched.
- `syncWithLocation` left at `TabbedForm`'s default unless it breaks under the hash router (then `false`).
- geofence-edit's projects input goes on the Details tab (it's metadata).

## Risks / notes

- The deferred-fit `DeckMap` change is load-bearing and touches every map in the app — covered by a dedicated test + the existing deck/map browser suite must stay green (regression surface: show pages, /map playground, workbenches).
- Viewport-fill height under the hash-router + form toolbar may need a small `calc` tweak to avoid double scrollbars — a preview-tuned value, flagged for verification.
