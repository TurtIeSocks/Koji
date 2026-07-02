# Koji v2 Map — Deferred Backlog

Living list of deferred map follow-ups (branch `claude/v2`). Newest deferrals on top.

## Merge existing geofences (deferred 2026-07-02)

**Today:** the Merge button only unions shapes in the **draft buffer** — i.e. shapes
drawn *this session* (`handleMerge` → `mergeSelected` → turf `union` over all draft
features → one feature → Save = one geofence). Disjoint → MultiPolygon; overlapping
→ Polygon.

**Limitation:** you **can't merge two already-saved geofences**, because clicking a
geofence loads *only that one* into the editor (`editFeature` replaces the draft), so
two existing fences can never be in the draft at once. It's also all-or-nothing
(merges everything drawn, not a chosen pair). So it's really "combine what I just
drew," not a general map merge.

**Wanted:** the real thing —
- Multi-select existing geofences on the map (shift-click to accumulate a selection).
- Merge → turf `union` of the selected geometries → one geofence.
- Persist: create the merged geofence + delete the originals (or update one, delete
  the rest). Needs the racy-realtime-safe refetch (already have `refetchGeofences`).

**Touches:** a multi-select model on the base geofences layer (not just the single
`selection`); reuse `mergeSelected` (`lib/merge-polygons.ts`); a delete path in the
dataProvider (exists). Decide UX: shift-click vs a "merge mode".

---

## Other known deferrals (from the working board)

- **Save calc result as route/geofence** — calc is overlay-only; can't persist the
  computed clusters/route. *Highest value — closes the calc loop.*
- **Calc panel UX review** (deferred 2026-07-02) — owner has reservations about some
  of the calc-panel decisions (mode/area/algorithm layout, dots-vs-path per mode,
  where reroute/route-stats live, wording). Revisit the whole panel's UX in one pass.
- **Import / export GeoJSON** on the map.
- **Snapping** (`SnappableMode` wraps a draw/modify mode) · **ellipse** draw ·
  **map-side delete** of a shape.
- **Verify reroute / route-stats** end-to-end (route path now renders for both; the
  input-from-selected-route flow is still unconfirmed in the user's tab).
- **Route-mode editing** — the toolbar Save targets geofences only; a selected route
  can't be geometry-edited + saved back.
- **`HexagonLayer` density heat** (deferred from Phase 2, optional).
- **Bundle code-split** — main chunk ~2.9 MB / 854 KB gz (deck + maplibre + ra-core
  unsplit); dynamic-import the `/map` route.
- **Duplicate geofences** created by the old save-branch bug (`map-<ts>-N` in the
  admin list) — one-time cleanup.
- **`claude/v2` → `main` merge** — the owner's explicit call (shared integration
  branch, not just the map).
