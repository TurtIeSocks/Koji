MDPI is hard-walled (Akamai). The benchmark's key finding (MapLibre mid-pack, Mapbox fastest at 50k+) is already captured from the search abstract. I have everything needed. Compiling the dense trace-doc material.

---

# Modern Web Map Frameworks for Geometry + Route Editing — MapLibre Ecosystem (current 2026-06-19)

## §1 Base renderer — MapLibre GL JS

- **`maplibre-gl`** — current **v5.24.0** (2026-04-23); 161 releases; **BSD-3-Clause** (the clean license, the whole reason it was forked from mapbox-gl-js v1 after Mapbox's Dec-2020 relicense to proprietary BSL). GitHub **10.9k★**, 1.1k forks, 343 open issues, actively maintained (TS 88.5% / GLSL 2.4%). [github.com/maplibre/maplibre-gl-js]
- v5.x line is the current major. Globe projection shipped; WebGPU is **roadmap, NOT released** for GL-JS as of 2026-04 — WebGPU work is landing in MapLibre **Native** (mobile/desktop) first. Treat WebGPU as "future, not bankable." [maplibre.org/news 2025-09/2025-10]
- Rendering model: GPU-accelerated vector-tile renderer (WebGL). Points/lines/polygons drawn as GPU layers off a `GeoJSONSource` or vector-tile source. Built-in **clustering** on `GeoJSONSource` (`cluster: true`, `clusterRadius` default 50, `clusterMaxZoom`). [maplibre.org/maplibre-gl-js/docs/examples/create-and-style-clusters]
- **Perf verdict for Koji's "thousands of pokestops via pixi":** a `circle` or `symbol` layer over a `GeoJSONSource` is the native, idiomatic replacement for the Pixi overlay — points become part of the GPU texture, tens of thousands render with smooth pan/zoom **without** a separate canvas. The 2025 MDPI benchmark (*Vector Data Rendering Performance Analysis of Open-Source Web Mapping Libraries*, ISPRS IJGI 14(9):336) ranks MapLibre **mid-pack**: at ≥50k points Mapbox GL JS fastest, then OpenLayers, then MapLibre, then Leaflet. So MapLibre comfortably covers low-tens-of-thousands of circles; it is *not* the absolute fastest at 50k+, but it beats Leaflet and beats a Pixi-overlay-on-Leaflet maintenance burden. [mdpi.com/2220-9964/14/9/336 — abstract only, full text Akamai-walled]
- For **>~100k–millions** of points, MapLibre's own large-data guide says: cut coordinate precision to 6 dp, chunk/stream, pre-tile with **tippecanoe**/**Martin** server-side (Martin "comfortably handles a 13GB DB"), or push to **deck.gl** hexbin/Scatterplot. Hard limit of GeoJSON-source + circle layer is not documented but clustering/`maxZoom:12`/disable-overlap are the prescribed escape hatches. [maplibre.org/maplibre-gl-js/docs/guides/large-data]

## §2 React integration

- **`@vis.gl/react-maplibre`** — the standalone repo `visgl/react-maplibre` was **ARCHIVED 2025-01-29** (read-only, 62★). Source folded into `visgl/react-map-gl`. Do **not** depend on the standalone repo's issue tracker. [github.com/visgl/react-maplibre]
- **`react-map-gl`** — current **v8.1.1** (2026-04-11), **MIT**, **8.5k★**, 1.4k forks, active. v8.0 introduced the **`react-map-gl/maplibre`** subpath import (typed for `maplibre-gl>=4`, no mapbox-gl/placeholder dep needed). v8.1 rewrote camera sync via Proxy (better terrain/non-mercator). React **>=16.3**. This is the canonical React wrapper. [github.com/visgl/react-map-gl/releases]
- Two install shapes: `npm i react-map-gl maplibre-gl` then `import {Map} from 'react-map-gl/maplibre'`, OR the legacy `@vis.gl/react-maplibre` package (MIT, same components, published from the unified repo). Prefer the `react-map-gl/maplibre` subpath going forward.
- TypeScript: first-class (wrapper is TS, maplibre-gl ships its own types). SSR/Next.js: standard caveat — `maplibre-gl` touches `window`/WebGL, so the `<Map>` must be client-only (dynamic import `{ssr:false}` or `'use client'`); no special SSR support beyond that.

## §3 Geometry EDITING libraries

### §3.1 Mapbox GL Draw — DEAD END for MapLibre
- `@mapbox/mapbox-gl-draw` is **no longer actively maintained** and is increasingly hard to run against MapLibre (API divergence mapbox-gl v2+ vs maplibre-gl). Forks exist but are unmaintained drift. **Do not pick this for a 2026 MapLibre admin.** [foss4g-2025 talk FJYFLZ; consensus across sources]
- `maplibre-gl-draw` as a standalone first-party package does **not** exist as a maintained thing — the de-facto answer is Terra Draw (below).

### §3.2 Terra Draw — THE recommendation ⭐
- **`terra-draw`** (James Milner, OSGeo project) — current **v1.31.2** (1.x **stable**, published within hours of this research; extremely active, 966+ commits). **MIT**, **1.0k★**, TS 99.2%. Framework-agnostic core + per-library **adapters**: Leaflet v1, OpenLayers v10, **MapLibre GL JS v4/5**, Google Maps v3, Mapbox GL v3, ArcGIS JS v4. [github.com/JamesLMilner/terra-draw; npmjs.com/package/terra-draw]
- **Architecture:** central in-memory **Store** of GeoJSON Features; adapters render the store onto the host map; you drive it imperatively. Key API: `addFeatures(Feature[])` (load existing geometry — Point/LineString/Polygon whose mode is enabled), `getSnapshot()` (read all features back as GeoJSON), `draw.on("change",(ids,type,ctx)=>…)` / `"finish"` events with `ctx.origin === "api"` to distinguish programmatic vs user edits. [guides/2.STORE.md, guides/6.EVENTS.md]
- **11 modes** with editing capability matrix [guides/4.MODES.md]:
  | Mode | Output geom | Vertex drag | Midpoint insert | Coord delete | Snapping | Validation | Resize |
  |---|---|---|---|---|---|---|---|
  | Point | Point | ✔ (via Select) | — | — | — | — | — |
  | LineString | LineString | ✔ | ✔ | ✔ | coord+line | `ValidateNotSelfIntersecting` | — |
  | Polygon | Polygon | ✔ | ✔ | ✔ | coord+line + `toCustom` fn | self-intersect + min/max area | ✔ center/opposite |
  | Circle | Polygon(circular) | ✔ | — | — | — | — | — (drag) |
  | Rectangle | Polygon | ✔ | ✔ | — | — | — | ✔ |
  | Angled Rectangle | Polygon(rotatable) | ✔ | ✔ | — | — | — | ✔ |
  | Freehand | Polygon | ✔ | — | — | — | — | — |
  | Freehand LineString | LineString | ✔ | — | — | — | — | — |
  | Sector / Sensor / PolyLine | Polygon/LineString | partial-doc | — | — | — | — | — |
- **Select mode** is the universal editor: per-geometry flags for `draggable` (whole feature), draggable/deletable **coordinates**, **draggable midpoints** (mid-vertex insertion), **resizable** (`center`|`opposite`, optional fixed aspect), same-mode coord+line **snapping**, and custom `validation` fns run on `finish`/`commit`/`provisional`. `pointerDistance` (default 40px) tunes hit-buffer.
- **Gaps for Koji's needs (explicitly NOT in Terra Draw):**
  - **MultiPolygon** — core modes emit single Polygon/LineString/Point. No first-class multipolygon *drawing*; you'd compose multipolygons yourself in the store/snapshot layer.
  - **Polygon cut / merge / boolean ops** — NOT provided. Bring **Turf.js** (`turf.union`/`turf.difference`/`turf.intersect`) and write features back via `addFeatures`.
  - **Coordinate precision setting** — not documented as a knob.
  - **Freehand on touch** — unsupported.
- **React caveat (load-bearing for Koji):** Terra Draw holds raw references to MapLibre layers. Under `react-map-gl/maplibre`, when a **sibling** component (e.g. a `<Marker>` update) forces the `<Map>` to re-render, Terra Draw can **lose its layer refs** → drawings vanish + `Cannot read properties of undefined (reading 'setData')`. Issue **#197 is OPEN/unresolved**. Mitigation: own the `maplibre.Map` instance via a stable ref, memoize map children hard, keep the Terra Draw instance in a `useRef` created once in a `useEffect`, and avoid driving markers through React state that re-renders the map. This is the single biggest integration risk for an editing admin. [github.com/JamesLMilner/terra-draw/issues/197]

### §3.3 maplibre-gl-terradraw — turnkey control (optional convenience layer)
- **`@watergis/maplibre-gl-terradraw`** (Jin Igarashi / watergis) — current **v1.13.2** (2026-05-30), 94 releases, **MIT**, **156★**, TS 75% / Svelte 10%. Wraps Terra Draw as a ready MapLibre `IControl` with a button UI: `new MaplibreTerradrawControl({modes:['polygon','select','delete',…]})`. Modes exposed: point, linestring, polygon, rectangle, circle, freehand, angled-rectangle, sensor, sector, select, delete-selection, delete, download. Bundles **Turf** for live area/distance measurement. Production users: Cartes.app, Open Hinata 3, UNDP GeoHub. [github.com/watergis/maplibre-gl-terradraw; FOSS4G-2025 talks UTUW8Z/FJYFLZ]
- **No documented React wrapper** — it's a vanilla MapLibre control. In `react-map-gl` you add it via `useControl`/`map.addControl` in an effect. For a custom-styled admin you likely skip this and use **Terra Draw core directly** for full UI control; use this control only if you want the prebuilt toolbar fast.

## §4 Routes / ordered MultiPoint / waypoint editing

- **`@maplibre/maplibre-gl-directions`** — current **v0.9.1** (2025-11-27), **MIT**, **156★**, 100% TS. Click-to-add waypoints, click-to-remove, **drag waypoints**, insert waypoint by dragging the route line, alternative routes, `refreshOnMove` (live re-route while dragging), per-waypoint bearings, congestion styling. **Requires an external OSRM- or Mapbox-Directions-compatible routing backend** — it is NOT a pure client-side waypoint editor; it always re-routes through a server. Does **not** advertise arbitrary waypoint **reordering** (sequential model). [github.com/maplibre/maplibre-gl-directions]
- **`route_snapper`** (dabreegster) — draws MapLibre routes **client-side**, snapped to a street network, no remote routing API. Good if Koji wants snapped routes without a routing server. [github.com/dabreegster/route_snapper]
- **Arrowed polylines / direction glyphs:** native MapLibre, no library — a `symbol` layer with `symbol-placement:'line'`, `symbol-spacing`, and an arrow `icon-image` (or a `line` layer with `line-pattern`) over the route LineString. This is the standard pattern; no dependency.
- **Ordered MultiPoint reorder verdict:** none of the above gives drag-to-reorder of waypoint *order* out of the box. For Koji's TSP-route editing, the realistic build is: Terra Draw (or raw circle layer) for the points + own React state holding the ordered array + a `line` layer redrawn from that order + arrow `symbol` layer for direction. Reorder is your own UI (drag list / map-drag remap), not a library feature.

## §5 Verdict for a geometry-editing admin (Koji)

- **Stack:** `maplibre-gl@5` + `react-map-gl@8` (`react-map-gl/maplibre` subpath) + **`terra-draw@1`** (core, not the prebuilt control) + **Turf.js** for cut/merge/multipolygon assembly + native `symbol`/`line` layers for routes & arrows. deck.gl `MapboxOverlay`/`ScatterplotLayer` held in reserve only if point counts blow past tens-of-thousands.
- **Pros:** all MIT/BSD (no Mapbox BSL exposure — matches Koji's license caution from the bootstrap/clustering memos); native GPU point rendering retires the Pixi overlay; Terra Draw covers polygon/rectangle/circle/line/point draw+edit incl. vertex drag, midpoint insert, snapping, self-intersection guard, resize; GeoJSON in/out via `addFeatures`/`getSnapshot` maps cleanly onto Koji's geo-types/KojiGeometry domain.
- **Cons / risks:** (1) **React re-render fragility** (issue #197, OPEN) — must isolate the map instance and memoize aggressively; biggest integration hazard. (2) **No native multipolygon/cut/merge** — Turf-bolted, you own the orchestration. (3) **No drag-to-reorder waypoints** — custom UI. (4) MapLibre point perf is mid-pack vs Mapbox at 50k+ — fine for thousands, plan tiling/deck.gl for the long tail. (5) WebGPU not yet in GL-JS — don't architect around it. (6) Freehand unsupported on touch. (7) `maplibre-gl-terradraw` and `maplibre-gl-directions` are small (~156★) single-maintainer projects — bus-factor risk; Terra Draw core (1.0k★, OSGeo-backed, daily commits) is the safer dependency.

Sources: github.com/maplibre/maplibre-gl-js · github.com/visgl/react-map-gl(+releases) · github.com/visgl/react-maplibre · github.com/JamesLMilner/terra-draw (guides 2/4/6, issue #197) · npmjs.com/package/terra-draw · github.com/watergis/maplibre-gl-terradraw · github.com/maplibre/maplibre-gl-directions · github.com/dabreegster/route_snapper · maplibre.org/maplibre-gl-js/docs/guides/large-data · mdpi.com/2220-9964/14/9/336 · deck.gl/docs/developer-guide/base-maps/using-with-maplibre · maplibre.org/news (2025-09, 2025-10)