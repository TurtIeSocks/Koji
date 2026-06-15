# Koji Rust Relocation Map (move defs closer to use) — 2026-06-14

Distinct from `map.md` (the completed V2 restructure). Philosophy: **aggressive by-usage**
— a def lives in the lowest crate all its real consumers can reach; koji-core keeps only
truly-shared vocabulary. Scope: cross-crate + intra-crate + visibility. Verified via real
`use <crate>::Sym` imports (bare word-greps over-report — several caught & discarded, e.g.
`clean`, `migration::Mode`, `OutputConfig`→algorithms mistag).

DAG (low→high): macros · koji-core · {db,dragonite,events,plugins,scanner} · algorithms · {service,wasm} · bins.

---

## Validation + decisions (locked 2026-06-14)
Fresh-eyes adversarial validator: **A1/A2/A4 CONFIRMED**, no missed movers, `sql_raw` confirmed dead.
**A3 REVISED** — `HasLatLon` trait STAYS in koji-core: `impl HasLatLon for PointStruct` (core geometry
type) would orphan if the trait moved. Only `count_in_area`/`AreaPolygons`/`sql_raw_bbox` move; scanner
already imports `HasLatLon` from core.
Granularity: **A1 = distribute** into clustering/routing/bootstrap · **A2 = new `requests::config`** ·
optionals (A4 macro+get_category_enum, koji-service 47-visibility, dead-code verify) **all IN**.

---

## A. Cross-crate relocations (the heart of the ask)

### A1 · koji-core → algorithms  (algorithm-parameter cluster)
Consumers = algorithms + koji-wasm + koji-service; all reach `algorithms`.
koji-core's fresh `query_args` read-layer has **zero** coupling to these (verified).

| symbol | from | action | notes |
|---|---|---|---|
| `ClusteringConfig` | config.rs:15 | move | generalizes the user's example |
| `RoutingConfig` | config.rs:29 | move | the literal example in the brief |
| `BootstrapConfig` | config.rs:36 | move | |
| `S2Config` | config.rs:9 | move | embedded by Clustering/Bootstrap configs; EXT=[] travels with them |
| `CalculationMode` | calc_mode.rs | move + impls | carries Serialize/Deserialize impls |
| `ClusterMode` | cluster_mode.rs | move + impls | |
| `SortBy` | sort_by.rs | move + impls | RoutingConfig field; PartialEq/Serialize impls travel |

New home: **`algorithms/src/config.rs`** (mirror) OR distribute into existing
`clustering`/`routing`/`bootstrap` modules. ← decision §E1.

### A2 · koji-core → koji-service  (request/output-side config)
Consumers = koji-service only (verified — koji-core agent mis-tagged as algorithms).

| symbol | from | action | notes |
|---|---|---|---|
| `OutputConfig` | config.rs:50 | move | embeds `ReturnTypeArg` (travels) |
| `DevConfig` | config.rs:60 | move | dev/benchmark toggles |
| `DataFilter` | config.rs:44 | move | embeds `SpawnpointTth` → stays in core; service imports it (service→core ✓) |
| `ReturnTypeArg` | return_type.rs:4 | move | API return-type enum |
| `get_return_type` | return_type.rs:19 | move | parser for ↑ |

New home: **`koji-service/src/requests/`** (beside the calc-Args). config.rs SPLITS across
two crates — expected; different real consumers.

### A3 · koji-core → koji-scanner  (sole consumer)

| symbol | from | action | notes |
|---|---|---|---|
| `count_in_area`, `AreaPolygons` | normalize.rs | move | scanner-only consumers |
| `HasLatLon` (trait) | normalize.rs | **STAYS core** | `impl HasLatLon for PointStruct` pins it (orphan); scanner imports it from core |
| `sql_raw_bbox` | util.rs:67 | move | scanner-only SQL helper |

### A4 · koji-core → koji-db  (sole consumer — optional / lower value)

| symbol | from | action | notes |
|---|---|---|---|
| `json_related_sort` | text_utils.rs:25 | move | db-only JSON sort helper |
| `get_category_enum` | enum_map.rs:6 | move? | koji-db already wraps it; thin. OPTIONAL |
| `enum_bridge!` macro | lib.rs:51 | move? | `#[macro_export]`, db-only consumer. Fiddly. OPTIONAL |

---

## B. Stays put (DAG-pinned / internally coupled) — WHY, no action
- **geometry::*** — koji-core internal use + sibling consumers (db,dragonite,algorithms,plugins) → LCA koji-core.
- **ApiQueryArgs cluster** (ApiQueryArgs/Filters/OutputSpec/PropertySelection/FeatureRenderSpec/AdminReq*/BoundsArg/SpawnpointTth) — consumers span db+scanner+service siblings → LCA koji-core. Refactored TODAY; leave.
- **Mode, Category** — multi-sibling consumers → koji-core. **UnknownId, TrimPrecision, create_cell_map, most s2::*** — internal use / sibling consumers.
- **NameModifier, separate_by_comma, get_mode_acronym, clean** — koji-core-internal args/text layer (NameModifier fresh).

## C. Visibility tightening (intra-crate; pub → pub(crate) unless noted)
- koji-core: `sql_raw` (no caller — remove/pub(crate))
- koji-db (~12): sea-orm `Relation` enums; NameId, InsertsUpdates, VecToJson, GeofenceNoGeometry, OnlyParent, RouteNoGeometry, OnlyGeofenceId, HierarchyArgError, Basic, FullPropertyModel, parse_order
- koji-scanner (8): gym/pokestop/spawnpoint/station `Model`+`Query`
- koji-events (~6): DispatchError, DispatcherHandle, SIGNATURE_HEADER, EVENT_ID_HEADER, sign, entity Relations
- koji-dragonite (~3): ApiAreaRarePokemonMode (unused), V2Envelope, V2ApiError
- koji-plugins: MANIFEST_FILE
- nominatim: LookupQueryBuilder, ReverseQueryBuilder, Zoom, Response, unused submodules — VERIFY (client lib; may be intentional API)
- koji-service (~47): pub handlers/scope fns/module decls used only in-crate → pub(crate). High count, low risk, marginal — OPTIONAL.

## D. Dead code (VERIFY before removal — not relocation, not auto-applied)
algorithms: `sec::*`, `routing::vrp::*` (disabled), `s2::ToGeo`, `s2::Dir`, utils debug fns, `cells_to_nearest_face_edges`, `meters_to_degrees`. May be used via dyn/trait/feature/bench.

## E. Open granularity decisions (sign-off)
1. **A1 algorithms target**: new `algorithms::config` mirror **vs** distribute into clustering/routing/bootstrap.
2. **A2 service target**: new `requests::config` **vs** fold into existing `requests::groups`/`ops`.
3. **A4 optional set** (get_category_enum, enum_bridge!): include or skip.
4. **C koji-service 47-item pass**: do or skip.
5. **D dead code**: separate verify pass after, or leave.

## Rejected mis-flags (agents tagged PURE→core; rejected)
- koji-plugins encode_latlng/decode_latlng/PROTOCOL_VERSION → core: **NO** (plugin-protocol internals).
- koji-dragonite ApiArea/V2*/area DTOs → core: **NO** (integration wire types).
- koji-db sea-orm Category/Mode/JsonToModel → core: **NO** (sea-orm storage types). [N.B. the *domain* `Category` enum DID move core→db — different type.]

---

## As-built (executed 2026-06-14, branch claude/v2)

| commit | what |
|---|---|
| `99356c5` | A1 — algo configs (Clustering/Routing/Bootstrap/S2Config + Cluster/CalculationMode, SortBy) → algorithms (distributed into clustering/routing/bootstrap) |
| `bff1f41` | A2 — request/output config (OutputConfig/DevConfig/DataFilter/ReturnTypeArg/get_return_type) → koji-service::requests::config |
| `65b625c` | A3 — count_in_area/AreaPolygons/sql_raw_bbox → koji-scanner (HasLatLon stayed — orphan rule) |
| `1867656` | A4 — json_related_sort + get_category_enum (merged) + enum_bridge! macro → koji-db |
| `913f7a6` | removed dead sql_raw + orphaned feature_single_vec from koji-core |
| `fa3fbe8` | visibility: 9 internal items → pub(crate) (db/events); 5 auto-reverted (real interface leaks) |
| `7004a27` | visibility: 71 in-crate-only items → pub(crate) (koji-service) |
| `4a4d5e8` | Category domain enum → koji-db (fresh-scan-found miss; beside its sea-orm storage twin) |
| `f315a3d` | dead code: vrp.rs + genetic.rs + debug_string/get_sorted removed (~681 LOC, owner-approved) |

**Fresh-eyes scan corrected the first agents:** sec/* is ACTIVE (greedy clustering) — NOT dead; dragonite/events types mostly used; OutputConfig fields read. Prevented bad deletes.

**Left in place (ambiguous — exported, no live caller, possibly intended API; flag only):** `s2::ToGeo`, `s2::Dir`, `koji-events::DispatchError`.

**koji-core now exports only genuinely-shared vocabulary:** `Mode`, `HasLatLon`, the ApiQueryArgs read-layer, `s2::*`, text helpers (NameModifier/clean/get_mode_acronym/separate_by_comma), `UnknownId`, `geometry::*`, `TrimPrecision`.
