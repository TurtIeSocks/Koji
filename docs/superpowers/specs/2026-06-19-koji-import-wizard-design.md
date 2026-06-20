# Koji Admin V2 — Import Wizard (design)

**Date:** 2026-06-19 · **Branch:** `claude/v2` · **Slice:** admin-V2, after slice 4 (geofence properties).

Bulk-create geofences/routes from uploaded/pasted/fetched geo data. A ground-up rewrite of the
legacy v1 wizard (`apps/web-client/src/components/dialogs/import/`), which was non-atomic, lied
about success, double-created on retry, mis-linked routes, and silently dropped data. This design
fixes every ranked v1 footgun (see §6) and keeps v1's one good idea: a single normalize
choke-point that all sources funnel through.

## 1. Decisions (locked in brainstorming)

| Fork | Decision |
|---|---|
| Commit path | **Atomic bulk endpoint** `POST /internal/import` — one DB tx, dedupe/upsert by name, per-feature result map, server-side route-parent resolution. |
| Input formats | **All four v1 source families**, via a pluggable source-adapter pipeline: GeoJSON file/paste · Poracle + ReactMap JSON · Remote URL · Shapefile/Golbat/Nominatim. Heavy sources land in later build-phases. |
| Name collisions | **Detect + flag in Review**, per-row action: **Skip (default)** or **Overwrite**. In-batch duplicate names are a blocking validation failure. |
| Mount | **Dedicated `/import` page** (wire ra-core `CustomRoutes`), launched from geofence + route list toolbars. |
| Draft safety | **Guard-on-leave** confirm (router `useBlocker` + `beforeunload`); state in memory for the session. localStorage persistence not in V1. |

## 2. Architecture & data flow

One pipeline, many sources. Every source normalizes to a GeoJSON `FeatureCollection`, then flows
through one shared path. **Nothing touches the DB until the single atomic commit.**

```
SOURCE ADAPTERS (pluggable, one interface)
  GeoJSON paste/file · Poracle/ReactMap · Remote URL · Shapefile · Golbat · Nominatim
        │  each: raw input -> FeatureCollection | typed ParseError
        ▼
  POST /internal/geometry/convert        (server normalizes/validates geometry)
        ▼
  CLIENT WIZARD STATE                     (one FeatureCollection, parsed ONCE)
        │  validate -> name -> assign (mode/parent/projects/on_collision) -> review
        ▼
  POST /internal/import                   (one tx; dry_run for the Review preview)
```

**Source adapter interface** (TS):
```ts
interface SourceAdapter<Input> {
  id: string;                                   // "geojson" | "poracle" | "reactmap" | "url" | ...
  parse(input: Input): Promise<FeatureCollection>;   // throws a typed ParseError on bad input
}
```
JSON adapters (GeoJSON/Poracle/ReactMap/URL) ship in phases B–C; Shapefile/Golbat/Nominatim in
D–E. Adding one never touches the pipeline.

## 3. Backend contract

Both endpoints under `/internal/*` (session-authed via the existing `public_validator`
middleware). Grounded against existing structs: `CreateGeofence`
(`crates/koji-service/src/public/v2/geofences.rs:82`), `CreateRoute` (`.../routes.rs:63`),
`geometry::convert` (`.../geometry.rs:43`), internal scope (`.../internal/mod.rs:30`).

### 3.1 `POST /internal/import` — atomic bulk commit (NEW)

Request:
```jsonc
{
  "dry_run": false,            // true = validate + detect collisions, write NOTHING
  "items": [
    {
      "kind": "geofence",      // "geofence" | "route"; defaulted from geometry type, overridable
      "name": "Downtown",
      "geometry": { /* GeoJSON, already normalized by /convert */ },
      "mode": "pokemon",       // optional
      "parent": "Region-A",    // geofence parent BY NAME (resolved server-side) | null
      "projects": [1, 4],      // project ids
      "route_parent": null,    // routes only: parent geofence BY NAME
      "on_collision": "skip"   // "skip" (default) | "overwrite"
    }
  ]
}
```

Response (identical shape for dry-run and real commit):
```jsonc
{
  "committed": true,           // false when dry_run, or when the tx rolled back
  "summary": { "create": 47, "update": 3, "skip": 2, "fail": 0 },
  "results": [
    { "index": 0, "name": "Downtown", "action": "create", "id": 91 },
    { "index": 5, "name": "Old Park", "action": "skip",   "reason": "collision, on_collision=skip" },
    { "index": 9, "name": "",         "action": "fail",   "reason": "empty name" }
  ]
}
```

**Transaction semantics (all-or-nothing on hard failure):**
1. One DB tx. Geofences upserted **first**, then routes — a route's `route_parent` resolves
   against fences created in the same batch *or* already in the DB. Kills v1's orphan-route bug.
2. `on_collision: skip` → existing row untouched, reported `skip` (NOT a failure). `overwrite` →
   upsert by name (reuses the existing `upsert_json_return` path).
3. Any **hard failure** (unresolvable parent, geometry the DB rejects, empty name, ambiguous
   parent) → the **entire tx rolls back**, `committed:false`, offending rows in `results`.
   Nothing partially written. Retry is safe — upsert/dedupe by name never double-creates.

**Dry-run is the Review step's data source.** Review calls `dry_run:true`; the server
authoritatively computes collisions + validation + counts. The user approves exactly that report;
Commit re-sends `dry_run:false` through the **same code path** → the preview cannot lie (v1's
core sin: a preview/notification decoupled from the actual write).

### 3.2 `POST /internal/geometry/convert` (+ `/simplify`) — `/internal` alias (NEW, thin)

Forward to the existing `v2::geometry` handler from the internal scope
(`internal/mod.rs`), mirroring how routes are aliased. No handler rewrite. `GeoInput` accepts
`FeatureCollection`/`Feature`/`Geometry` (geojson-only — Poracle/ReactMap coercion happens in the
client adapter *before* convert).

### 3.3 Scoped OUT of V1 (assumptions)
- Import does **not** create koji *Property* associations from arbitrary GeoJSON `properties`;
  those are read only for name extraction. (Property-row import = a later slice.)
- `parent`/`route_parent` resolve **by name**; an ambiguous name (duplicate in batch) is a
  validation failure surfaced at dry-run.
- `mode` falls back to the server default when omitted (same as single-create).

## 4. Frontend wizard

**Route:** wire ra-core `<CustomRoutes>` into `apps/web/src/App.tsx` → `/import`. Launch buttons
on the geofence + route list toolbars (reuse the `BulkActionsToolbar`/action-button pattern)
navigate there.

**No stepper primitive ships** (confirmed absent in shadmin/ra-core) → compose a tiny `<Stepper>`
from a step array (index + labels + active panel). No new dependency. Four steps:

1. **Source** — tabbed ingest, one adapter per tab. Paste/File (GeoJSON) via `MonacoJsonInput`
   (live parse, **errors shown, never silent-zeroed**) · Poracle/ReactMap (auto-detect +
   coerce → GeoJSON) · URL (dedicated field, raw fetch, loading/error/abort) ·
   Shapefile/Golbat/Nominatim (same adapter interface, phased). Valid input → `/convert` → store
   the normalized FeatureCollection. Banner shows feature count or the parse error.
2. **Map & Name** — read-only Leaflet preview (reuse `FeatureCollectionInput`'s map, bbox-fit,
   click-to-highlight) beside a naming panel: pick the name property (**derived reactively** from
   the current features) + optional `{name}`/`{index}` template with case modifier. Empty/dup
   names flagged inline, live.
3. **Assign** — one **unified** virtualized grid (`DataTable`), one row per feature — no v1
   Fences/Routes split. Columns: name (editable), kind (auto from geometry, overridable), mode,
   parent, projects, on-collision. Bulk "apply to selected" for mode/parent/projects +
   Select-All/None. Routes get a parent-fence picker (imported fence by name, or existing).
   **Hooks at component top, never inside the render-prop** (v1's rules-of-hooks bug).
4. **Review & Commit** — calls `dry_run:true`, renders the **server's authoritative report**:
   create/update/skip/fail counts, the collision list (reflecting each row's skip/overwrite),
   validation failures as **blocking** errors (Commit disabled while `fail > 0`). Commit →
   `dry_run:false` → progress + the final result map. Failures keep you on the page, listed and
   fixable.

**State model:** a local `useReducer`+context store on the page —
`{ features: NormalizedFeature[], assignments }`, **parsed once**, never re-`JSON.parse`'d per
render. Guard-on-leave via router `useBlocker` + `beforeunload`.

**Reused, not rebuilt:** `MonacoJsonInput`, Leaflet `FeatureCollectionInput`/map, `FileInput`
dropzone, `DataTable`, shadcn `Tabs`/`Badge`/`Button`. New code = pipeline glue, source adapters,
grid wiring, the `<Stepper>`, and `dataProvider.import`.

## 5. dataProvider

Add an `import` method to `apps/web/src/data-provider.ts` (custom, not a standard ra-core verb):
`import(body): Promise<ImportResult>` → `POST /internal/import`. Called with `dry_run:true` from
Review and `dry_run:false` from Commit. No `createMany` exists and none is added — `/import` is
the batch path.

## 6. v1 footgun kill-map

| # | v1 footgun (file:line) | v2 fix |
|---|---|---|
| B1 | N-request loop, hard-coded "success", dup-on-retry (`fetches.ts:629`) | atomic `/import` tx + upsert/dedupe + honest result map |
| B3 | routes orphaned/mis-linked (`SaveToKoji.tsx:26`) | parent resolved server-side in the same tx |
| B4 | commits with no preview (`Finish.tsx`) | dry-run **is** the Review screen |
| B2 | errors swallowed to console (`fetches.ts:672`) | per-row `results` + blocking validation |
| M3 | unsupported geometry silently dropped (`SaveToKoji.tsx:13`) | flagged at convert/assign, quarantined not dropped |
| M2 | malformed JSON silent-zeros (`ImportWizard.tsx:114`) | parse error surfaced in Source step |
| M5 | URL → `"null"`, arbitrary fetch (`Code.tsx:44`) | dedicated URL field, raw fetch, error/abort states |
| M4 | empty/dup/"Invalid Property" names (`PropsStep.tsx:50,83`) | live + server name validation; placeholder never coined |
| M6 | Esc/reset nukes work (`ImportWizard.tsx:85`) | dedicated page + guard-on-leave |
| M1 | sync parse/stringify freezes (`ImportWizard.tsx:37,114`) | parse once, virtualized grid, commit progress |
| M9/M8 | unguarded parse, shapefile race (`Json.tsx:33`, `ShapeFile.tsx:46`) | adapters guard parse, wait for shp+dbf, surface errors |
| M7 | frozen prop list (`PropsStep.tsx:62`) | name property derived reactively |
| m3 | hooks in render-prop (`AssignStep.tsx:325`) | hooks hoisted to top |
| m1/m2/m6 | tooltip XSS, `chartset` typo/data-URI, `__`-prefix metadata | sanitized render, Blob download, typed assignment channel |

**Kept from v1 (good bones):** live preview triad, single normalize choke-point, bulk-assign
affordances (Select-All/None, apply-to-all), name templating, virtualized grid.

## 7. Testing (TDD throughout)

- **Backend** — `/import` unit + DB-gated integration (`KOJI_DB_URL` per the test-db setup):
  dry-run↔commit parity, route-parent resolution, **rollback on hard failure**, idempotent retry
  (run twice → no dups), collision skip/overwrite.
- **Frontend unit (jsdom)** — each source adapter (parse + coerce + error cases), name
  templating/validation, reducer transitions, `dataProvider.import`.
- **Frontend browser (vitest-browser)** — Source parse-error surfaces, Assign bulk-apply, Review
  renders the dry-run report, Commit happy path (mocked DP), guard-on-leave fires.
- **Live verify** — real end-to-end import. Uses its own result UI, not undoable toasts → free of
  the Claude Preview `document.hidden`/sonner-timer trap (see memory).

## 8. Build phases (each shippable, its own plan-chunk)

- **A — Backend:** `/import` endpoint + `/internal/geometry` alias + tests.
- **B — Frontend core:** `/import` route, `<Stepper>`, Source(GeoJSON paste/file), Map&Name,
  Assign, Review&Commit, `dataProvider.import`, guard. JSON happy path live-verified end-to-end.
- **C:** Poracle/ReactMap + URL adapters.
- **D:** Shapefile adapter.
- **E:** Golbat + Nominatim adapters.

Phase A is the first writing-plans target.
