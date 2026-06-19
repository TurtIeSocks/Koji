# Koji Frontend V2 — Good/Bad Assessment (Phase 3)

_Verdict per module. Verdicts: **Keep / Port / Rewrite / Defer / Drop**. Confidence in brackets._

## Verdict summary

| Area | Verdict | One-line reason |
|---|---|---|
| App shell / router (`App.tsx`, `index.tsx`) | **Rewrite** [H] | New shadmin `<Admin>` shell; react-router v7 via ra-core. Config-gate auth → real `authProvider`. |
| `/admin` 6 resources | **Rewrite-on-shadmin** [H] | Logic ports cleanly; UI is MUI→shadcn. Resources map ~1:1 to shadmin `ResourceProps`. |
| `dataProvider.ts` | **Rewrite** [H] | Lossy GeoJSON→row projection + ignored sort/filter. Rebuild as ra-data adapter atop new row endpoints. |
| Geometry inputs (`CodeInput` JSON) | **Replace** [H] | shadmin ships interactive `PolygonInput`/`MultiPointInput` shape editors — big UX upgrade over raw JSON. |
| `Code.tsx` (CodeMirror JSON) | **Replace** [M] | Use shadmin Monaco JSON input; drop `@uiw/react-codemirror` + codemirror deps. |
| Color input (`react-admin-color-picker`) | **Replace** [H] | shadmin extras `ColorInput`. |
| MUI `styled/`, `dialogs/`, `notifications/`, `buttons/` | **Rewrite** [H] | MUI-coupled glue; rebuild on shadcn primitives / shadmin equivalents (sonner notifications, shadcn Dialog/Sheet). |
| `services/utils.ts` (geo helpers) | **Port** [H] | Pure, framework-neutral (turf ops, color rules, mode mapping). Port wholesale; fix 12→4 mode collapse. |
| `services/fetches.ts` | **Split** [M] | Admin CRUD → dataProvider; calc/job/geometry calls → typed client kept for map backport. |
| `assets/types.ts` domain types | **Port** [H] | Keep GeoJSON/Koji entity types; reconcile `__`-namespace + `KojiModes` 12→4 collapse vs v2 backend. |
| `/map` page + `markers/`, `popups/`, `drawer/`, `interface/` | **Defer** [H] | Backport target. Keep MUI version live until ported. The single biggest surface. |
| `useShapes` / `usePixi` / `useSyncGeojson` / `useLayers` | **Defer (port w/ map)** [H] | Map-only geometry engine + Leaflet/Pixi-coupled rendering. |
| `usePersist` / `useStatic` | **Defer (split)** [M] | Map state stays with map backport; any admin-relevant prefs (theme) move to ra-core `useStore`. |
| `useDbCache` | **Defer / re-evaluate** [M] | Cross-store cache hack; new row + choices endpoints make most of it unnecessary. |
| `useRaStore` | **Drop** [H] | Tiny admin UI-flags store; replace with ra-core `useStore` / local state. |
| `SaveToGolbat` / `getGolbatCache` | **Drop / re-evaluate** [M] | v2 gap (stubbed); golbat-direct save superseded by admin `publish` action. |
| `/play` Playground, standalone `/convert` | **Drop or fold** [L] | Dev tools; fold conversion into admin or drop. Confirm. |
| MUI + emotion + theme | **Drop** [H] | Replaced by Tailwind v4 + shadcn tokens (oklch CSS vars, light/dark). |

## Tensions with goals

- **dataProvider rewrite is the #1 build item** and the load-bearing dependency for the whole admin
  port. It's blocked on backend list/sort/filter behavior → pairs with the **internal-endpoint**
  additions (row endpoints, batch, choices, bulk-delete). Do backend + dataProvider together.
- **Geometry editing UX** improves immediately: today's admin edits geometry as a JSON blob in
  CodeMirror; shadmin's shape inputs give an interactive map editor in the form. This is a
  "looking nice" win available in the first admin pass, *without* the full `/map` backport.
- **Mode collapse (12→4)** is a latent correctness issue (`getRouteByCategory` matches on
  `raid|quest|station|pokemon`; v2 collapses modes). Resolve the canonical mode set during the port.
- **shadmin churn:** realtime + native-CSS theming + granular registry are `[Unreleased]`. **Pin a
  shadmin commit** for the Koji client; treat the contract as ra-core 5.14.

## What's genuinely good (keep the design, not the code)

- The **resource model** (project↔geofence↔property↔route references, publish action, import wizard)
  is sound — port the *behavior*, restyle the UI.
- `utils.ts` turf/geometry ops (`combineByProperty`, `splitMultiPolygons`, color rules) are reusable.
- The v2 **envelope + job queue** API design is clean; the client just needs a thin typed wrapper.
