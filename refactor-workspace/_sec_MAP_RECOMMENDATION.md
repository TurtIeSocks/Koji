# Koji V2 Map Framework — Recommendation

## TL;DR

**Primary: Stay on a Leaflet-class basemap, but replace `leaflet-geoman` with `terra-draw` as the editing layer.** Concretely: **`react-leaflet@4` + `leaflet@1.9.4` (BSD-2) + `terra-draw@1.31` (MIT) via its Leaflet adapter + `@turf/turf@7` (MIT)** for cut/merge/multipolygon orchestration. Keep the **Pixi overlay** for the 10k+ point layer for now (it works and is independent of the editing decision).

**Runner-up (the "real upgrade" path): `maplibre-gl@5` (BSD-3) + `react-map-gl@8` `/maplibre` subpath (MIT) + the *same* `terra-draw@1.31` + `@turf@7`,** native `circle`/`symbol` layers replacing Pixi, deck.gl held in reserve.

The single highest-leverage move is **adopting Terra Draw regardless of basemap.** Its adapter pattern decouples the editing code from the renderer, so the basemap swap (Leaflet → MapLibre/OL) becomes a *separate, reversible, later* decision — exactly the "future-proof, not urgent" framing the `/map` backport calls for. Terra Draw is the only choice that lets you de-risk both decisions independently.

---

## (1) The stack, with versions/licenses

| Layer | Pick | Version | License | Why |
|---|---|---|---|---|
| Basemap renderer | Leaflet (primary) / MapLibre GL (runner-up) | 1.9.4 / 5.24.0 | BSD-2 / BSD-3 | Both clean permissive; no Mapbox BSL exposure (matches Koji's license caution per bootstrap/clustering memos) |
| React wrapper | react-leaflet / react-map-gl `/maplibre` | 4.2.1 / 8.1.1 | MIT / MIT | react-leaflet already in tree (zero churn); react-map-gl is the canonical MapLibre React wrapper |
| **Editing** | **terra-draw** | **1.31.2** | **MIT** | Framework-agnostic adapters (Leaflet v1 + MapLibre v4/5 + OL v10); GeoJSON in/out; vertex/midpoint/snap/resize/rotate built in |
| Geometry math | @turf/turf | 7.x | MIT | union/difference/intersect/area/midpoint — the cut/merge/multipolygon engine. Already an internal dep of Terra Draw, so it's coming in regardless |
| High-count points | leaflet-pixi-overlay (keep) → native GL `circle` layer (on swap) | — | MIT / BSD-3 | Pixi stays under Leaflet; retired by a native GPU layer if/when you move to MapLibre |

**Why Terra Draw over staying on geoman:** geoman is healthy (2.19.3, Apr 2026) and feature-complete — this is *not* a "geoman is dying" argument. The argument is **portability**. geoman is welded to Leaflet; if Koji ever wants ESM-modern tooling or 50k+ live features, geoman cannot follow you to MapLibre/OL and you rewrite the entire editing layer. Terra Draw writes the editing layer *once* against a stable GeoJSON store and survives the basemap swap. For a feature being dropped-and-backported, betting on the portable layer is the correct future-proofing.

**Why not OpenLayers as primary:** OL has the best *native* editing (Draw/Modify/Snap/trace, first-party MultiPolygon) and best canvas perf (wins 100k lines outright) and the cleanest license (BSD-2). It is the strongest single-framework editing home and is the legitimate "if you're going to rewrite anyway" target. But it forces a **full rewrite of the map layer now** for a route that's being deferred — the cost lands before the value. Terra Draw-on-Leaflet captures ~80% of the modernization (TS-native editing, snapping, portability) at ~20% of the migration cost, and keeps OL open as the runner-up's runner-up. If you decide the backport *is* a full rewrite, promote OL.

---

## (2) Coverage of EACH current Koji map capability

Mapping against the inventory's geoman/arrowheads/pixi feature set:

**Geometry editing (polygon / multipolygon / circle / rectangle):**
- Draw: Terra Draw `PolygonMode`, `RectangleMode`, `CircleMode` (circle emits a circular Polygon — matches Koji's "circle drawn as route points/polygon" model). ✅ covers `drawPolygon`/`drawRectangle`/`drawCircle`.
- Edit/drag/rotate/scale: Terra Draw **Select mode** — per-feature vertex drag, **midpoint insertion**, coordinate delete, whole-feature drag, resize (`center`/`opposite`), **rotation**. ✅ covers geoman's `editMode`/`dragMode`/`rotateMode` and the per-shape `pm:edit`/`pm:dragend`/`pm:rotateend` rebinding (now a single `change`/`finish` event with `ctx.origin` to tell user-edits from API loads).
- Snapping: Terra Draw `flags: { toCoordinate, toLine }`, `pointerDistance` tunes the hit-buffer. ✅ covers geoman `snappable`/`snapIgnore`.
- Self-intersection guard: `ValidateNotSelfIntersecting` + min/max-area validation. ✅ matches/exceeds geoman.
- **MultiPolygon — GAP.** Terra Draw emits single Polygons. **You own multipolygon assembly in the store via Turf** (`turf.union` to merge, store as MultiPolygon, explode to Polygons for editing). This is the same orchestration Koji *already* does in `useShapes.combine` (turf `union`, `keepCutoutsOnMerge`) and `splitMultiPolygons`/`To MultiPolygon`/`To Polygon` — so the logic is largely a port, not net-new.
- **Cut — GAP.** geoman has `pm:cut`; Terra Draw doesn't ship a named cut mode. **Bolt with `turf.difference`** triggered from your own toolbar button → write result back via `addFeatures`. Koji's `Create Shape from Cutouts`/`getFeatureCutouts` already lives in app code, so the cut surface is partly yours today anyway.
- **Merge / combine — GAP→Turf.** Koji's `setters.combine` is *already* Turf-based, not geoman-based. Direct port. ✅ effectively.

**Route / MultiPoint / arrowhead editing:**
- Ordered MultiPoint model: keep Koji's existing model (ordered Points + `${a}__${b}` LineString links + `__forward`/`__backward`/`__multipoint_id`, `activeRoute` explode/collapse). Terra Draw draws/edits the Points; the **ordering + linking stays your React/store logic** (it already is). No library gives drag-to-reorder of TSP waypoints — neither geoman nor any alternative does today, so this is **zero regression**.
- Split line at midpoint (`splitLine`), remove-point-rebridge, reroute (`POST /jobs`): all **app-level store logic + API calls** — renderer-agnostic, carries over unchanged.
- **Arrowheads:** `leaflet-arrowheads` is Leaflet-only and **will not survive a basemap swap.** Replacement is *better* on the GL path: a native `symbol` layer with `symbol-placement:'line'` + arrow `icon-image` (MapLibre) renders direction glyphs with no dependency. On the Leaflet-primary path, keep `leaflet-arrowheads@1.4` as-is (the `getColor`-by-distance rules are app logic and port either way). ✅ covered, dependency-free on the upgrade path.

**10k+ point performance (pokestops/gyms/spawnpoints/stations):**
- **Primary (Leaflet):** keep `leaflet-pixi-overlay` + `pixi.js`. It already handles tens of thousands and is fully decoupled from the editing decision — no reason to touch it now. (MDPI: Leaflet core itself falls apart >50k, which is exactly *why* Pixi exists here; it stays.)
- **Runner-up (MapLibre):** the Pixi overlay is **retired** — points become a `GeoJSONSource` + native `circle`/`symbol` layer on the GPU texture, with built-in `cluster:true`/`clusterRadius`. MDPI ranks MapLibre mid-pack (behind Mapbox/OL, ahead of Leaflet at 50k+) — comfortably fine for low-tens-of-thousands. If counts blow past that, drop in **deck.gl `MapboxOverlay` + `ScatterplotLayer`** (1M pts @ 60 FPS) over the same MapLibre basemap, no rewrite of the rest. The Pixi `getScale`/`scaleMarkers`/per-geohash-color/`pokestopRange` logic maps onto layer `paint` expressions.

**Import overlays (Wizard / MiniMap / Shapefile / Nominatim / Golbat):**
- All import sources (`shapefile`, Nominatim `GET /nominatim`, Golbat fences, JSON) produce **GeoJSON FeatureCollections** — fully renderer-agnostic. `addFeatures(snapshot)` loads them into Terra Draw's store; `getSnapshot()` reads them back. ✅
- `MiniMap.tsx` preview (LayersControl bbox/feature) is a second small map instance — trivially a second `<MapContainer>`/`<Map>` regardless of basemap. ✅
- The `convert`/`area`/`merge-points`/S2/coverage backend calls are all API + Turf/`nodes2ts` client math — renderer-independent. ✅

---

## (3) Migration risk vs staying on Leaflet+geoman

**Cost of the primary (Leaflet + Terra Draw):** **moderate, bounded, incremental.** You keep react-leaflet, the Pixi overlay, panes, tile layer, locate/zoom controls, popups, the entire store/fetch layer, S2, geohash. You **rewrite only the geoman binding** (`Drawing.tsx` + per-shape `pm:*` rebinding in `Polygon.tsx`/`Point.tsx`) against Terra Draw's `change`/`finish` events, and **port** the already-Turf-based combine/cut/multipolygon ops to fire on Terra Draw's store. The toolbar UI (cut/merge/remove/rotate controls) becomes your own shadcn buttons driving Terra Draw modes — which you want anyway for theming (see §4).

**The one real risk to flag — Terra Draw React re-render fragility (issue #197, OPEN):** under React wrappers, a sibling re-render of the `<Map>` can make Terra Draw lose its layer refs (`Cannot read properties of undefined (reading 'setData')`). **Mitigation is well-understood:** own the map instance in a stable `useRef`, create the Terra Draw instance once in a `useEffect`, hard-memoize map children, and never drive the high-count markers through React state that re-renders the map (Koji already keeps markers in Pixi/imperative layers, not React children — so Koji is *structurally* well-positioned to dodge #197). This is the thing to prototype first in the backport.

**Cost of staying on Leaflet+geoman:** **near-zero now, higher later.** geoman is maintained and complete; there is no urgent forcing function. The debt is (a) Leaflet *core* frozen at 1.9.4 since 2023 (2.0 still alpha), and (b) geoman's Leaflet lock-in meaning any future renderer change = full editing rewrite. Staying is the correct call **if you're confident Koji never needs >50k live features and never wants off Leaflet.** Given V2 is a deliberate modernization onto shadcn/Tailwind, betting on never-moving is the riskier bet.

**Verdict:** the primary trades a moderate, *bounded* rewrite-now (one binding file + porting existing Turf logic) for portability and TS-native editing. Because the route is deferred, you spend the rewrite during the backport when you'd be touching this code anyway — the cost lands exactly when the value does, not before.

---

## (4) Tailwind / shadcn theming fit

This is where **Terra Draw decisively beats geoman**, and it's the most underrated factor for a shadcn admin:

- **geoman ships its own toolbar DOM + CSS** (`.leaflet-pm-toolbar`, injected buttons). Theming it into shadcn means fighting/overriding vendor CSS — exactly the kind of style-leak a Tailwind design system tries to eliminate. Koji *already* works around this with custom `Drawing.tsx` controls + MUI `StyledPopup`.
- **Terra Draw has NO built-in UI.** It's a headless editing engine — you call `draw.setMode('polygon')` imperatively. **You build the entire toolbar as native shadcn components** (`Button`, `Toggle`, `ToggleGroup`, `DropdownMenu`, `Tooltip`) wired to mode setters. The cut/merge/rotate/remove controls, the keyboard-shortcut map (`usePersist.kbShortcuts`), the mode toggles — all become first-class Tailwind/shadcn UI with zero vendor-CSS override. This is a clean fit, not a workaround.
- **Geometry styling** (the orange combine-select highlight, `__KOJI`/`__GOLBAT` color-by-source, distance-color rules) is set via Terra Draw's per-feature style functions / GL `paint` expressions reading feature properties — driven straight from your Tailwind CSS variables / theme tokens. The popups already need a rewrite off MUI `Paper` → shadcn `Popover`/`Card` regardless of map choice.
- On the **MapLibre runner-up**, the same holds and is even cleaner: no Leaflet control DOM at all, every control is yours.

Net: Terra Draw's headlessness is *the* reason it suits a shadcn admin better than geoman — you stop overriding a vendor toolbar and start composing your own.

---

## (5) Fallback option

**Fallback (if Terra Draw's #197 React fragility proves unworkable in the backport prototype, or you need named cut/split/measure without writing them): stay on `react-leaflet@4` + `leaflet-geoman-free@2.19` exactly as today.** It is healthy, MIT, feature-complete (cut/split/rotate/scale/measure all first-party), and lowest-risk. You absorb the shadcn-theming friction (override geoman's toolbar CSS, or hide it and drive geoman's API from your own shadcn buttons — geoman *can* be driven imperatively too) and accept the Leaflet lock-in. This is the safe harbor and requires zero migration.

**Second fallback / promote-on-rewrite: OpenLayers (`ol@10` BSD-2 + `rlayers@3` ISC, or imperative).** If the backport scope expands into a full map-layer rewrite, OL is the superior single-framework home: first-party Draw/Modify/Snap/trace + native MultiPolygon, best canvas perf, cleanest license. Terra Draw *also* has an OL v10 adapter, so even this path keeps your editing code if you adopt Terra Draw first.

---

## One-line decision

Adopt **Terra Draw now as the editing layer** (the portable, headless, shadcn-friendly bet that survives any basemap), keep **Leaflet + Pixi** as the basemap for the incremental backport, hold **MapLibre+native-GL (retiring Pixi)** as the future-proof runner-up, and keep **geoman-as-is** as the zero-risk fallback and **OpenLayers** as the promote-on-full-rewrite target.

Relevant files for the backport (all under `/Users/rin/GitHub/Koji/apps/web-client/src/`): `pages/map/interface/Drawing.tsx` (the geoman binding to rewrite), `pages/map/markers/{Polygon,Point,LineString}.tsx` (per-shape event rebinding → Terra Draw events), `hooks/{useShapes,useSyncGeojson,usePixi}.ts` (store + Turf combine/cut logic ports + Pixi layer), `services/{fetches,geoUtils}.ts` (renderer-agnostic, carry over).