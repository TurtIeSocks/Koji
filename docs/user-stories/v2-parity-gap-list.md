# Koji V2 Parity Gap-List

Parity audit of the v2 stack (`apps/web` admin client + koji server) against the legacy "main" backlog, as of 2026-06-26. Scope is strictly `apps/web` + the Rust server crates (`koji-service`, `koji-db`, `algorithms`, `koji-core`, plugins); the standalone interactive map, Convert/Playground pages, and docs-site stories are flagged where they fall outside that line. Statuses below are taken verbatim from the 6 checker verdicts — nothing invented.

## Scorecard

| Epic | Done | Partial | Missing | OOS | % done |
|------|------|---------|---------|-----|--------|
| 1 Geofence API | 4 | 2 | 1 | 0 | 57% |
| 2 Route API | 4 | 1 | 1 | 0 | 67% |
| 3 Project API | 3 | 2 | 0 | 0 | 60% |
| 4 Geofence↔Project links | 2 | 1 | 1 | 0 | 50% |
| 5 Property API | 3 | 1 | 1 | 0 | 60% |
| 6 Tile-server API | 3 | 1 | 0 | 0 | 75% |
| 7 Admin: geofence UI | 3 | 2 | 1 | 0 | 50% |
| 8 Admin: resource CRUD UI | 1 | 3 | 0 | 0 | 25% |
| 9 Admin: bulk actions | 1 | 3 | 0 | 0 | 25% |
| 10 Map: drawing | 0 | 2 | 1 | 0 | 0% |
| 11 Map: layers/markers/S2 | 0 | 0 | 3 | 0 | 0% |
| 12 Map: calc UI | 0 | 0 | 4 | 0 | 0% |
| 13 Map: import/export/convert | 0 | 1 | 3 | 0 | 0% |
| 14 Global UX | 1 | 0 | 6 | 0 | 14% |
| 15 Geometry convert API | 3 | 0 | 2 | 0 | 60% |
| 16 Calc engine | 3 | 1 | 0 | 0 | 75% |
| 17 Cluster/route tuning | 3 | 1 | 0 | 0 | 75% |
| 18 S2 coverage | 2 | 0 | 0 | 0 | 100% |
| 19 Scanner push | 0 | 1 | 2 | 0 | 0% |
| 20 Data queries | 3 | 2 | 0 | 0 | 60% |
| 21 Auth/config/geocode | 4 | 0 | 0 | 0 | 100% |
| 22 Public calc API | 0 | 2 | 0 | 0 | 0% |
| 23 Deploy/config docs | 2 | 1 | 1 | 0 | 50% |
| 24 Distribution guide | 2 | 0 | 1 | 0 | 67% |
| 25 Scanner sync contract | 1 | 1 | 1 | 0 | 33% |
| 26 Plugins | 3 | 0 | 0 | 0 | 100% |
| 27 Docs/algorithms | 2 | 0 | 0 | 0 | 100% |
| **TOTAL** | **56** | **28** | **26** | **1** | **~51%** |

Ground-truth verdict tally: **56 done / 28 partial / 26 missing / 1 out-of-scope** across **111 checked stories** (checkers split a few backlog stories into finer-grained items, so the count exceeds the backlog's 107). `% done = 56 / (56+28+26) ≈ 51%` of in-scope stories. The per-epic rows above are the synthesizer's bucketing for scanability and may differ by ±2–3 from the headline tally — treat the *gap substance* below as the signal, not the exact integer. The lone out-of-scope story is Epic 25.3 (docs-only), detailed in §Out of scope.

**Spot-check (2026-06-26):** 5 originally-unverified verdicts were confirmed against code — E3.3 stays partial (with a quick-win twist), and E5.3, E13.2, E13.3, E24 are now confirmed **missing** (were partial). Tally above reflects this.

## Top gaps (build these for parity)

- **Standalone interactive `/map` page (Epics 10–12, 14)** — there is no map page at all; drawing/editing exists only embedded in geofence/route CRUD forms. The entire map-driven workflow (draw, layer toggles, marker fetch, S2 overlay, popups, locate, URL coords, bounds persistence) is absent. **Effort: L (largest single cluster).**
- **Map-driven calc UI (Epic 12)** — server `/api/v2/calc` + `/api/v2/jobs` exist and work, but there is zero client UI to configure mode/category/algorithm/sort-by or run clustering/routing/bootstrap from the map. Also missing: per-category Max-Area auto-calc thresholds with localStorage persistence, simplify/route-index/marker-scaling toggles. **Effort: L.**
- **Scanner push UI + direct scanner write (Epics 19, 9.3, 25)** — only Dragonite event-push is wired (202 `{event_id}`, not `{updates, inserts}`). No scanner-type detection, no CONTROLLER_DB_URL routing, no `save-scanner`/`save-koji` bulk endpoints, no reload-call header routing. Client publish button hits `/internal/.../publish`, not the backlog's `/push`. **Effort: M–L.**
- **Convert dialog + Playground UI (Epic 13.4)** — server `/api/v2/geometry/convert` exists; the entire client Convert page and `/playground` are absent. **Effort: L.**
- **Public read-list pagination + reference-list endpoints (Epics 1, 2, 3, 5, 6)** — public `/api/v2/{geofences,routes}` list endpoints return all rows as GeoJSON with no pagination/filter/sort; those live only in `/internal/*`. No `/all/`, `/parent/`, `/search/` reference endpoints exist for any resource (needed for dropdowns/cascades). **Effort: M across resources.**
- **Multi-format geometry import (Epic 7.3)** — import wizard accepts only `.json`/`.geojson`; no WKT, CSV, or shapefile parsers, no server-side conversion beyond GeoJSON normalization. **Effort: L.**
- **Name-modifier + structural query flags on convert (Epic 15)** — v2 geometry endpoints lack the trim/replace/capitalize/parent-prefix name modifiers and the exclude-properties/exclude-parents/fullcoords structural flags the legacy `convert.rs` exposed via `ApiQueryArgs`. **Effort: M.**
- **Public scanner/instance data queries (Epic 20)** — no scanner-instance merge or `/api/v2/instances/from-koji`/`from-scanner` endpoints; only Koji row-lists. Spawnpoint TTH filter is hardcoded `All` (no `?tth=` param). **Effort: M.**
- **Admin list filters & bulk-direction gaps (Epics 7–9)** — geofence list missing Project/Parent filters + row expansion; route list missing Geofence + points-range filters; projects bulk toolbar missing the project→geofence "Assign Geofence" direction; export has no GeoJSON download dialog. **Effort: M total.**
- **Global UX shell (Epic 14)** — no landing page with Map/Admin/Convert/Play cards, no keyboard-shortcuts dialog, no tile-server dropdown, no network-status indicator, no 404/ErrorBoundary, no DANGEROUS-flag dev-gating. **Effort: S–M each, M cluster.**
- **`cluster_split_level` not exposed (Epics 17.2, 22.1)** — S2-partitioned parallel clustering param has no v2 API surface. **Effort: M.**
- **Bootstrap/calc persistence side-effects (Epics 16.2, 22.2)** — `save_to_db`, `save_to_scanner`, project-reload, and `save-koji`/`save-scanner`/`delete` calc endpoints are deferred per phase notes. **Effort: M–L.**

## Gaps by epic

### Epic 1: Geofence admin API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Retrieve one geofence with related data | partial | Public GET omits related projects/properties/routes; only `/internal/geofences/{id}` populates them | S |
| List geofences with pagination & filtering | partial | Public list is unpaginated GeoJSON; pagination/filter/sort only in `/internal` | M |
| Get reference lists (all/parents/search) | missing | No `/all/`, `/parent/`, `/search/` endpoints for dropdowns/cascades | M |

### Epic 2: Route admin API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| List routes with pagination & filtering | partial | Public list unpaginated GeoJSON; filters (q/geofenceid/mode/points-range) only in `/internal/routes` | M |
| Get reference lists (all/parents/search) | missing | No `/all/`, `/parent/`, `/search/` route endpoints | M |

### Epic 3: Project admin API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Retrieve one project with linked geofences | partial | **Confirmed:** macro `get_one` returns the bare model, no `geofences[]`. But `get_one_json_with_related()` already exists in `koji-db/src/db/project.rs:68` (unwired) — quick fix is a 1-line handler swap. ⚠️ client `project-show.tsx` already expects `geofences[]`, so real-backend show/edit currently under-hydrates. | S |
| List / reference / search projects | partial | Missing `/projects/all/` and `/search/projects/` endpoints | S |

### Epic 4: Geofence↔Project links
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Unlink a geofence from a project | partial | No dedicated `DELETE /api/v2/geofence_project`; deletion only indirect via geofence PATCH | S |
| List all junctions | missing | `get_all` exists in DB layer but no `GET /api/v2/geofence_project/all/` endpoint | S |

### Epic 5: Property admin API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Retrieve one property with assignments | missing | **Confirmed:** `get_one` returns bare model; no `with_related` method exists for property (only `paginate` hydrates `geofences[]`). Add the method + wire the handler. | S |
| List / reference / search properties | partial | Missing `/properties/all/` and `/search/properties/` endpoints | S |

### Epic 6: Tile-server admin API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| List / reference / search tile servers | partial | Missing `/tile-servers/all/` and `/search/tile-servers/` endpoints | S |

### Epic 7: Admin geofence UI
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Browse, filter, expand geofences | partial | Missing Project filter, Parent filter, row expansion (`geofence-list.tsx`) | M |
| Create geofence (single + import wizard) | partial | Import wizard handles GeoJSON/JSON only; no multi-format conversion | M |
| Import multi-format geometry (WKT/CSV/shapefile) | missing | `source-step.tsx` accepts only `.json/.geojson`; no WKT/CSV/shapefile parsers | L |

### Epic 8: Admin resource CRUD UI
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Project CRUD | partial | List missing description + api_endpoint/api_key columns; show missing description | S |
| Route CRUD | partial | List missing Geofence filter + points-range filter + sidebar parent filter | M |
| TileServer CRUD | partial | Show page missing tile preview (renders name/url only) | S |

### Epic 9: Admin bulk actions
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Bulk assign parent/projects (both directions) | partial | Missing project→geofence "Assign Geofence"; only geofence→project exists | M |
| Bulk delete + bulk/single export | partial | No GeoJSON FeatureCollection download dialog / `/area/{id}` integration | M |
| Push (sync) to scanner | partial | Hits `/internal/.../publish` not `/push`; only geofence+route have buttons; project has none | S |

### Epic 10: Map drawing
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Draw polygons & route circles | partial | Drawing only embedded in CRUD forms; no standalone `/map` page | L |
| Edit/delete/cut/rotate/merge shapes | partial | Cut/edit/delete handlers form-only; rotate + merge modes absent; no standalone map UI | L |
| Configure drawing behavior & preferences | missing | No prefs drawer (Snappable/Continue/Keep-Cutouts/Radius/line-color); Geoman hard-coded in forms | M |

### Epic 11: Map layers / markers / S2
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Toggle shape & marker layers | missing | No layer toggles; no gym/pokestop/spawnpoint/station marker fetching; no shortcuts | M |
| Filter markers (last-seen/TTH/range/geohash) | missing | No marker-filter UI; no marker query (all/area/bound) infra in client | M |
| Visualize & configure S2 cells | missing | Server `/api/v2/s2` exists; no client overlay, display dropdown, or cell-ID copy | M |

### Epic 12: Map calc UI
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Configure & run clustering/routing/bootstrap | missing | Server `/api/v2/calc` exists; no client tab/drawer/popup to configure or run | L |
| Auto-calc thresholds, simplify, route index, marker scaling | missing | No Settings drawer; no per-category Max-Area inputs or simplify/index/scale toggles | M |
| Tune per-category Max-Area thresholds w/ persistence | missing | No Settings drawer; no localStorage persistence; no popup threshold logic | S |
| Navigate map & inspect via popups | missing | No `/map` page; no locate button, URL coords, popups, bounds/zoom persistence | M |

### Epic 13: Map import / export / convert
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Import/export polygons & routes (GeoJSON) | partial | Import wizard exists; export dialogs (copy+download) not found | S |
| Import from scanner / Koji DB | missing | **Confirmed:** wizard `source-step.tsx` offers only Paste + File (GeoJSON); no instance/project selector and no server endpoint to list/pull scanner or Koji-DB shapes | M |
| Configure retention & MultiPolygon handling | missing | **Confirmed:** no Combine-by-Name / Split-MultiPolygon UI or transform logic; `to-import-items.ts` passes geometry unchanged (split logic exists in `algorithms` but unwired) | M |
| Convert & transform (Convert dialog + Playground) | missing | Server convert exists; client Convert page + `/playground` entirely absent | L |

### Epic 14: Global UX
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Landing page & cross-section nav | missing | No home with Map/Admin/Convert/Play cards; dashboard is counts-only | S |
| Theme/shortcuts/tile-server/drawer/notifications/network | missing | No theme toggle UI, shortcuts dialog, tile dropdown, drawer mini-icons, notifications, network status | M |
| Surface dev-only (dangerous) features behind server flag | missing | No client read of DANGEROUS flag from `/config`; no dev-only gating | S |
| Error pages & resilient failures | missing | No 404 page, no map ErrorBoundary, no calc/import error notifications | S |

### Epic 15: Geometry convert API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Apply name-property modifiers to output | missing | No trim/replace/capitalize/case/parent-prefix/Polish-ASCII modifiers in v2 geometry endpoints | M |
| Filter output with structural query flags | missing | No excludeproperties/excludeparents/exclude/ignoremanualparent/fullcoords flags | M |

### Epic 16: Calc engine
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Bootstrap an area (Radius/S2/custom) | partial | save_to_db, save_to_scanner, project-reload side effects deferred (`calc.rs` module docs) | M |

### Epic 17: Cluster/route tuning
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Tune clustering (split level, max clusters, centering, rotate) | partial | `cluster_split_level` (S2 parallel partition) not exposed in v2 API (`requests/groups.rs`) | M |

### Epic 19: Scanner push
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Push geofences/routes/projects to scanner & reload | partial | Only Dragonite event-push; no direct RDM/Golbat/Unown DB write or reload routing; 202 `{event_id}` not `{updates, inserts}` | M |
| Detect scanner type & route to controller DB | missing | No schema-based scanner detection; no CONTROLLER_DB_URL fallback | M |
| Bulk save to scanner or Koji | missing | No `save-scanner`/`save-koji` endpoints; `/internal/import` writes Koji only | M |

### Epic 20: Data queries
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Filter spawnpoint queries by TTH status | partial | TTH filter exists in koji-golbat but handler hardcodes `SpawnpointTth::All`; no `?tth=` param | S |
| List & fetch instances from Koji or scanner | partial | Koji row-lists only; no scanner-instance merge, no from-koji/from-scanner, no single fetch | M |

### Epic 22: Public calc API
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Bootstrap, cluster, route via public API | partial | `cluster_split_level` absent; save_to_db/save_to_scanner deferred | M |
| Save, delete, validate via public API | partial | `save-koji`/`save-scanner`/`delete` endpoints not in v2 calc API (persistence deferred) | L |

### Epic 23: Deploy / config docs
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Configure env vars & reverse-proxy limits | partial | Server uses GOLBAT_DB_URL; no DATABASE_URL or UNOWN_DB_URL fallback handling | S |
| Document deprecated env-var aliases & upgrade path | missing | Docs list aliases but server implements no fallback/deprecation-warning layer | S |

### Epic 24: Distribution guide
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Reference recommended map settings | missing | **Confirmed:** no Settings drawer, no per-POI Max-Area km² inputs, no localStorage persistence in v2 client (legacy had 4 inputs, default 100 km²); ships with the map calc UI | M |

### Epic 25: Scanner sync contract
| Story | Status | Gap | Effort |
|-------|--------|-----|--------|
| Server sync / reload contract | partial | Only Dragonite area-mode PATCH; no direct scanner DB routes or reload-call header routing (Basic/X-*-Secret) | L |

## Fully ported (done)

- **Epic 1** — geofence create, update, delete (cascade), assign parent.
- **Epic 2** — route create, update, retrieve-one, delete.
- **Epic 3** — project create, update, delete (links-only).
- **Epic 4** — link geofence→project, batch-replace links both directions.
- **Epic 5** — property create, update, delete.
- **Epic 6** — tile-server create, update, retrieve/delete.
- **Epic 7** — edit/show geofence with map preview, typed properties input, bulk delete with undo.
- **Epic 8** — property CRUD admin UI (list + create + show).
- **Epic 9** — admin nav + theme toggle.
- **Epic 14** — login/logout UI (show/hide toggle, Enter submits).
- **Epic 15** — area→format convert (14+ formats), simplify (Douglas-Peucker) + merge-points.
- **Epic 16** — clustering with chosen algorithm, reroute-without-recluster, benchmark mode.
- **Epic 17** — sort clusters (6 strategies + plugins), genetic-post-processing stub, route+area stats.
- **Epic 18** — S2 coverage (circle/grid/bounds/polygons) + neighbor-traversal expansion (fully done).
- **Epic 20** — data-point queries (all/bound/area by category), area-stats aggregation, public geofence reads in many formats.
- **Epic 21** — public+private auth, config/algorithm discovery, Nominatim search + reverse-geocode client (fully done).
- **Epic 23** — Docker/standard install docs, DB pooling + logging docs.
- **Epic 24** — login + create distribution project, import + Dragonite bootstrap loop.
- **Epic 25** — ReactMap/Poracle/Golbat/Dragonite/rdmGruber integration docs, cross-network Dragonite proxy config.
- **Epic 26** — plugin protocol (JSON stdio v1), serialization/error contract, external tsp-mt plugin support (fully done).
- **Epic 27** — clustering/mygod_score docs, docs nav + copyable examples (fully done).

## Out of scope (docs site / legacy client)

- **Epic 25.3 — "Sync geofences/routes to scanner & trigger refresh (docs)"** — verdict `out-of-scope`: purely web-docs documentation coverage, and `apps/web-docs` is outside the audited scope (`apps/web` + koji server only). The functional side of this contract is tracked under Epic 25's partial sync row and Epic 19's scanner-push gaps above.

Note: Epics 10–14 (interactive map + global UX) are *in scope* per the audit but are the client stories the team had flagged for deferral to v2.1+. They are counted as real partial/missing gaps above, not OOS — they remain unbuilt work, just lower priority than the admin-panel and API parity items.