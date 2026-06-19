# Alternative Web Map Frameworks for Geometry/Route Editing (current to 2026-06-19)

Raw material for refactor trace. Koji currently uses Leaflet + leaflet-geoman (per MEMORY index, `leaflet` skill, and global rules). All npm figures pulled live from registry.npmjs.org + api.npmjs.org on 2026-06-19. Hard perf numbers from a 2025 peer-reviewed benchmark (MDPI ISPRS Int. J. Geo-Inf. 14(9):336 — Leaflet 1.9.4 / Mapbox GL 3.7.0 / MapLibre GL 4.7.1 / OpenLayers 10.2.1, RTX 3070, Chrome 131, 10-run avg, 1240 runs total).

## Hard data table (live npm, 2026-06-19)

| lib | latest | released | weekly dl | license |
|---|---|---|---|---|
| `terra-draw` | 1.31.2 | 2026-06-17 | 173,538 | MIT |
| `@geoman-io/leaflet-geoman-free` | 2.19.3 | 2026-04-10 | 138,803 | MIT |
| `@deck.gl-community/editable-layers` | 9.3.7 | 2026-06-11 | 25,557 | MIT |
| `rlayers` (OL React wrapper) | 3.9.0 | 2026-02-11 | 4,193 | ISC |
| `ol` (OpenLayers core) | 10.9.0 | 2026-04-15 | 725,009 | BSD-2 |
| `maplibre-gl` | 5.24.0 | 2026-04-23 | 3,223,650 | BSD-3 |
| `leaflet` (core, **stable**) | **1.9.4** | **2023-05-18** | 5,607,511 | BSD-2 |

Note the standout: **Leaflet stable is still 1.9.4 from May 2023 — 3 years stale.** 2.0 exists only as `2.0.0-alpha.1` (alpha released 2025-08, no stable timeline). Massive download count is install-base inertia, not active core dev.

---

## 1. OpenLayers (`ol`)

**Editing capability — strongest native editing of the open-source pack, honestly assessed as best-in-class.** Three composable interaction classes, all first-party (no plugin, no fork):
- `ol/interaction/Draw` — Point, LineString, Polygon, Circle, MultiPoint/Line/Polygon, plus `geometryFunction` for arbitrary shapes (boxes, regular polygons, freehand via `freehand:true`). `appendCoordinates`, `removeLastPoint`.
- `ol/interaction/Modify` — vertex drag, **midpoint/vertex insert** (alt-click or drag a segment), vertex delete (alt-click). Operates on a `VectorSource` or `Collection`.
- `ol/interaction/Snap` — snap-to-vertex and snap-to-edge across an entire source; the canonical pattern is to add Snap **last** so it intercepts Draw/Modify pointer events. Official examples: `draw-and-modify-features`, `draw-modify-trace-snap` (the `trace` interaction follows existing geometry edges — useful for shared borders / route-on-network).
- Full multipolygon/multilinestring editing, hole editing, topology preservation via Snap.

This is the **only** option here where polygon/multipolygon/circle/route + snapping + vertex-insert is all first-party and battle-tested — geoman replicates this for Leaflet as a plugin; OL ships it in core.

**Perf at 10k+** (MDPI 2025): Canvas renderer — Leaflet & OL are fastest of all four up to 50,000 lines / 10,000 polygons. For **100,000 lines OpenLayers won outright at 1295.5 ms (~2× faster than Mapbox/MapLibre)**. Only crack: 50,000 polygons, OL 1366 ms vs Mapbox 1265 ms (first time OL lost). OL also has a separate **WebGL path** (`WebGLVectorLayer`, `WebGLPointsLayer`) for tens-of-thousands of features GPU-accelerated, but GitHub issue openlayers#15619 documents memory growth + non-smooth panning on very large `WebGLVectorLayer` sources (OL 9). So: Canvas is excellent to ~50k; WebGL extends further but with rough edges. A 2026-04 Medium post flags an OL WebGPU "pixel drift" problem — WebGPU path is immature.

**License:** BSD-2-Clause. Clean, permissive, no commercial tier. Best license story of the editing-capable options.

**React + TS:** OL core is shipped with TS types (written in JS, `.d.ts` provided). React wrappers:
- `rlayers` (3.9.0, ISC, TS-native, maintainer publicly commits to long-term maintenance) — exposes `<RLayerVector>` + `<RInteraction.RDraw>` / `RModify` / `RSnap` as declarative components. Only runtime dep is lru-cache (~8KB). Low downloads (4.2k/wk) = small community but healthy.
- Alternative: skip the wrapper, mount OL imperatively in a `useEffect` + ref (the common production pattern; React owns the DOM node, OL owns the canvas). For an editing-heavy admin this is arguably *more* robust than a declarative wrapper because edit state lives in OL's `Collection`/`VectorSource`, not React state.

**Activity:** Core `ol` 10.9.0 (2026-04-15), 725k weekly dl, continuous major releases. Very healthy.

**Verdict:** The serious upgrade target if leaving Leaflet. Editing is its home turf.

---

## 2. deck.gl + EditableGeoJsonLayer (`@deck.gl-community/editable-layers`, ex-nebula.gl)

**Lineage matters here.** Original `uber/nebula.gl` (`@nebula.gl/layers` 1.0.4) is **dead** — last publish ~3 years ago, no maintainers, repo closed to external contributions. The live successor is `@deck.gl-community/editable-layers`, **9.3.7 (2026-06-11)**, MIT, 25.5k weekly dl. Examples modernized to Vite + TS + React 19, targets **deck.gl v9**.

**Editing capability:** `EditableGeoJsonLayer` edits a GeoJSON FeatureCollection — Point/LineString/Polygon/Multi* via swappable **edit modes**: `DrawPolygonMode`, `DrawLineStringMode`, `DrawCircleByDiameterMode`/`DrawCircleFromCenterMode`, `DrawRectangleMode`, `ModifyMode` (vertex drag + **midpoint insert** + delete), `TranslateMode`, `RotateMode`, `ScaleMode`, `DuplicateMode`, plus boolean ops (`extrudeMode`, `splitPolygonMode`). Snapping exists but is weaker/less polished than OL's or geoman's — it's the historically least-loved part. Route editing = LineString modify; no network-snap/trace equivalent out of the box.

**Perf at 10k+ — this is its entire reason to exist.** deck.gl is GPU/WebGL2: ScatterplotLayer renders ~**1M points at 60 FPS**, framerate drops to 10–20 FPS approaching 10M; hard crash ~10–100M (Chrome 1GB contiguous-buffer cap). For 10k points this is overkill but trivially smooth. **Caveat:** `EditableGeoJsonLayer` is heavier than a static layer (it re-tessellates on edit, renders guide/handle sublayers) — the GPU win is for *rendering* the underlying data, not for the edit overlay itself. If Koji's actual pain is "10k+ points on screen while editing a few," deck.gl is the only option that renders the backdrop at 60 FPS.

**License:** MIT (deck.gl core Apache-2.0). No commercial tier.

**React + TS:** Best-in-class React story — `@deck.gl/react` `<DeckGL>` is a real React component; deck.gl was built React-first at Uber. TS support out of the box. Interops with MapLibre/Mapbox as a base layer via `MapboxOverlay` / interleaved rendering — **so you can keep MapLibre/Leaflet as basemap and overlay deck.gl edit layers.** This is the key interop point: deck.gl is not a basemap, it's an overlay.

**Activity:** `deck.gl-community` is explicitly **"semi-maintained"** — repo's stated mission is "preserve valuable deck.gl ecosystem code that lacks a dedicated home," and "some modules may have no dedicated maintainer, sometimes no one responds quickly to issues." Releases track deck.gl v9 cadence (overall repo at 9.3.7, Jun 2026) but editable-layers specifically rides on community goodwill. **This is the risk flag.**

**Verdict:** Pick only if huge-dataset rendering is the dominant requirement. For pure editing ergonomics it's a step down from OL, and maintenance is "semi."

---

## 3. Leaflet, modernized (stay-put options)

### 3a. leaflet-geoman (what Koji uses today)
`@geoman-io/leaflet-geoman-free` **2.19.3 (2026-04-10)**, MIT, 138.8k weekly dl, 2.4k stars, ~23 open issues. **Actively maintained** — primarily by @Falke-Design, recent work includes polygon-hole removal with min-vertex enforcement, temp-layer cleanup, improved snapping. **This is not abandonware; it's healthy.**

**Editing capability — genuinely complete:** Draw, Edit, Drag, **Cut**, **Rotate**, **Split**, **Scale**, Measure, **Snap**, Pin. Supports Markers, CircleMarkers, Polylines, Polygons, Circles, Rectangles, ImageOverlays, LayerGroups, GeoJSON, **MultiLineStrings and MultiPolygons**. Vertex add/remove/drag, snapping with config. Feature-for-feature this *matches* OL's interactions and adds split/cut/measure on top. The **free tier covers everything Koji needs**; Geoman Pro (separate commercial license, no per-seat/royalty) adds advanced UX but is not required.

**License:** MIT (free package). Pro is a separate paid SKU — irrelevant unless you opt in.

**TS:** Ships `leaflet-geoman.d.ts`. React: no official React binding — used imperatively against a Leaflet `map` instance (which is how Koji already does it).

### 3b. Leaflet core + Turf.js
Leaflet handles render/interaction; **Turf** (`@turf/*` 7.x) does the geometry math (buffer, union, difference, intersect, simplify, midpoint, nearest-point-on-line, transform-rotate/scale/translate, area, distance, boolean-point-in-polygon). Note: **Terra Draw and deck.gl-community both depend on Turf 7.2 internally** — Turf is the de-facto geometry engine regardless of which renderer you pick, so Koji can adopt it independent of the map-framework decision.

**Is Leaflet a "relic" in 2026?** Blunt answer: **the core is stale, but not broken, and for editing it's irrelevant.**
- Core stable frozen at 1.9.4 since 2023-05; 2.0 only alpha (2025-08, ESM, drops IE, Pointer Events, no global `L`). No stable timeline.
- **But for editing-heavy admin specifically, Leaflet is still fine** — the editing lives in geoman (actively maintained, 2026-04 release), not in Leaflet core. Leaflet's job is pan/zoom/DOM + Canvas vector rendering, which 1.9.4 does correctly and won't regress.
- **Perf (MDPI 2025):** Leaflet is *fastest of all four* for <1000 features (~1.15× faster than OL), competitive to 10k. **Falls apart above 50k** — joins MapLibre as slowest, ~2× the time of OL/Mapbox; 50k+ exceeds 1s render. Default PNG markers are a perf trap (10k markers ≈ 30s, glitchy) — must use `L.circleMarker`/SVG. So Leaflet is fine for thousands, weak for tens of thousands.

**Verdict on staying:** If Koji's datasets are in the thousands, not tens-of-thousands, **staying on Leaflet+geoman is defensible and lowest-risk** — geoman is healthy, editing is complete, the only debt is core staleness (mitigated by eventual 2.0 or just pinning 1.9.4 which works). The "relic" framing only bites if (a) you need 50k+ live features or (b) you want ESM/modern-bundler ergonomics that 1.9.4 lacks.

---

## 4. Commercial — Mapbox GL JS & Google Maps

### Mapbox GL JS v3
**License is the disqualifier-or-not.** Mapbox GL JS went proprietary in Dec 2020 (v2+); v3 is non-OSS, usage-billed. **Pricing 2025:** free tier 50,000 map loads/mo, then **$5 per 1,000 loads**. A "map load" = each `Map` instantiation; pan/zoom/style-toggle within a 12-hour session is free. **Editing:** no native edit interactions — you'd layer Mapbox GL Draw (`@mapbox/mapbox-gl-draw`) or Terra Draw on top. **Perf (MDPI):** GPU/WebGL; fastest at 50k polygons (1265 ms), but a **fixed ~160 ms init cost regardless of feature count** + token-auth round-trip. Loses to OL at 100k lines. **Verdict:** the licensing + per-load billing is a hard disqualifier for an open-ish admin tool when **MapLibre GL is the free BSD-3 fork of this exact codebase** (`maplibre-gl` 5.24.0, 3.2M weekly dl) — Koji should use MapLibre, never Mapbox, if going the GL route.

### Google Maps JavaScript API
**License: pure commercial, pay-per-load.** 2025 pricing (post-March-2025 restructure): Dynamic Maps (JS) **$7.00/1,000 loads** ($0.007/load to 100k, $0.0056 after), per-SKU free caps (10k Essentials / 5k Pro / 1k Enterprise events/mo), up to 80% volume discount. **Editing:** the Drawing Library (`google.maps.drawing.DrawingManager`) does polygon/polyline/circle/rectangle/marker draw; vertices are editable (`editable:true`, `draggable:true`) with midpoint insert; **no snapping, no multipolygon, no route-network trace** — weakest editing of the lot. **Verdict:** hard disqualifier — recurring per-load cost + proprietary terms + weakest editing. Only justified if you specifically need Google's basemap/Street View/Places, which a geometry-editing admin does not.

---

## 5. Newer 2025–2026 entrants

### Terra Draw (`terra-draw` 1.31.2, 2026-06-17) — **THE entrant that matters for Koji**
James Milner's library, founded 2023, hit **1.0 in 2025**, MIT, **173.5k weekly dl** (already outpaces geoman), OSGeo project. **It is library-agnostic via an adapter pattern** — one drawing/editing API that runs on **Leaflet (v1), MapLibre (v4/5), Mapbox (v3), OpenLayers (v10), Google Maps (v3), ArcGIS (v4)**. Written 99% TypeScript, fully typed. Depends on Turf 7.2 internally.

**Editing capability — broad and modern:**
- Draw modes: Point, LineString, Polygon, Circle, Rectangle, **AngledRectangle**, **Sector**, **Sensor**, Freehand (polygon), FreehandLineString, PolyLine, Marker.
- **Select mode** (one per instance): per-feature-type editing — feature drag, **vertex drag, midpoint insertion + drag, coordinate deletion**, resize (`center`/`opposite`/`center-fixed`/`opposite-fixed`), **rotation + scaling** (geodesic on globe projection; resizable currently web-mercator only).
- **Snapping** via `flags`: `toCoordinate` and `toLine` (snaps to coords/segments of same-mode geometries); custom validation hooks; self-intersection guards.
- Geodesic line/circle drawing (globe projection). Deep styling. Custom modes/adapters extensible.

**The strategic point:** Terra Draw lets Koji **decouple the editing layer from the basemap choice.** It runs on Koji's *current* Leaflet today, and if Koji later swaps the basemap to MapLibre or OL, the editing code stays identical — only the adapter import changes. This is the lowest-risk migration path: adopt Terra Draw on existing Leaflet now, keep the basemap-swap as an independent future decision.

**Gaps vs geoman:** geoman has explicit **cut / split / measure**; Terra Draw doesn't ship those as named modes (cut/split would need custom mode + Turf difference). For raw route/polygon editing + snapping, Terra Draw is at parity or ahead.

**Activity:** extremely active (1.31.2 this week, 966+ commits, 189 tags), single very-engaged maintainer + OSGeo backing. Risk = bus-factor, mitigated by MIT + clean codebase.

### Others
- **MapLibre GL JS 5.24.0** (2026-04, BSD-3, 3.2M weekly dl) — the free Mapbox fork; the default modern GL basemap. No native editing (use Terra Draw / mapbox-gl-draw on top). MapLibre v5 added globe projection, perf work. This is the natural basemap if leaving Leaflet's raster/canvas world.
- No other credible *new* React-geospatial-editing framework emerged 2025–2026 beyond the above; the action is consolidation (Terra Draw as cross-lib editor, deck.gl-community absorbing nebula.gl, MapLibre absorbing Mapbox's OSS mantle).

---

## Blunt ranking (for Koji: editing-heavy admin, currently Leaflet+geoman)

1. **Terra Draw (on current Leaflet, or on MapLibre/OL later) — best fit.** Modern, TS-native, actively developed, decouples editing from basemap so the migration is incremental and reversible. Adopt this regardless of whether you ever change the basemap. Only real gap is named cut/split/measure (geoman has them).

2. **Stay on Leaflet + leaflet-geoman — lowest risk, still valid in 2026.** geoman is healthy (2.19.3, Apr 2026) with the most complete editing feature set (cut/split/rotate/scale/measure). The only debt is stale Leaflet *core*, which doesn't affect editing. Correct default **if datasets stay in the thousands and you don't need ESM-modern tooling.** Don't migrate just because Leaflet "feels old."

3. **OpenLayers (`ol` + `rlayers` or imperative) — best if you want a real framework upgrade.** First-party Draw/Modify/Snap/trace, best open license (BSD-2), strongest canvas perf (wins 100k lines outright), WebGL path for more. Heaviest migration cost (full rewrite of map layer), but the most capable single-framework editing home long-term.

4. **deck.gl-community editable-layers — only if 10k+ live features on screen is the gating requirement.** Unmatched GPU rendering (1M pts @ 60 FPS), React-first, interops as overlay on MapLibre/Leaflet. Downgraded by "semi-maintained" status and weaker snapping/edit ergonomics. Niche pick.

5. **MapLibre GL JS (basemap) + Terra Draw/mapbox-gl-draw (editing)** — viable modern GL stack, free BSD-3, but MapLibre alone has *no* editing; it's a basemap that needs an editing lib bolted on, so it collapses into option 1's editing layer with a different basemap. Worse canvas-scale perf than OL/Leaflet below 50k per MDPI; init overhead ~350 ms.

6. **Mapbox GL JS — disqualified** by proprietary license + $5/1k-load billing when MapLibre is the free fork of the same code.

7. **Google Maps JS API — disqualified** by $7/1k-load recurring cost, proprietary terms, and the *weakest* editing (no snapping, no multipolygon, no network trace).

**One-line recommendation for the trace doc:** adopt **Terra Draw** as the editing layer on the **existing Leaflet basemap** (incremental, reversible, gains modern TS editing + snapping), keep **leaflet-geoman** only if you depend on its cut/split/measure modes, and treat a basemap swap to **OpenLayers** (best editing-framework license + perf) or **MapLibre+deck.gl** (only for 10k+ live features) as a *separate, later* decision that Terra Draw's adapter layer makes cheap.

Sources: live npm registry/api (2026-06-19); MDPI ISPRS IJGI 14(9):336 (2025) — https://www.mdpi.com/2220-9964/14/9/336 ; https://github.com/JamesLMilner/terra-draw + https://terradraw.io ; https://github.com/geoman-io/leaflet-geoman ; https://visgl.github.io/deck.gl-community/docs/modules/editable-layers + npm `@deck.gl-community/editable-layers` ; https://openlayers.org/en/latest/examples/draw-modify-trace-snap.html + GitHub openlayers#15619 ; https://github.com/mmomtchev/rlayers ; https://leafletjs.com/2025/05/18/leaflet-2.0.0-alpha.html ; https://docs.mapbox.com/mapbox-gl-js/guides/pricing ; https://mapsplatform.google.com/pricing .