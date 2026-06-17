# Koji Rust Relocation Map (move defs closer to use) — 2026-06-14

Distinct from `map.md` (the completed V2 restructure). Philosophy: **aggressive by-usage**
— a def lives in the lowest crate all its real consumers can reach; koji-core keeps only
truly-shared vocabulary. Scope: cross-crate + intra-crate + visibility. Verified via real
`use <crate>::Sym` imports (bare word-greps over-report — several caught & discarded, e.g.
`clean`, `migration::Mode`, `OutputConfig`→algorithms mistag).

DAG (low→high): macros · koji-core · {db,dragonite,events,plugins,golbat} · algorithms · {service,wasm} · bins.

---

## Validation + decisions (locked 2026-06-14)
Fresh-eyes adversarial validator: **A1/A2/A4 CONFIRMED**, no missed movers, `sql_raw` confirmed dead.
**A3 REVISED** — `HasLatLon` trait STAYS in koji-core: `impl HasLatLon for PointStruct` (core geometry
type) would orphan if the trait moved. Only `count_in_area`/`AreaPolygons`/`sql_raw_bbox` move; golbat
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

### A3 · koji-core → koji-golbat  (sole consumer)

| symbol | from | action | notes |
|---|---|---|---|
| `count_in_area`, `AreaPolygons` | normalize.rs | move | golbat-only consumers |
| `HasLatLon` (trait) | normalize.rs | **STAYS core** | `impl HasLatLon for PointStruct` pins it (orphan); golbat imports it from core |
| `sql_raw_bbox` | util.rs:67 | move | golbat-only SQL helper |

### A4 · koji-core → koji-db  (sole consumer — optional / lower value)

| symbol | from | action | notes |
|---|---|---|---|
| `json_related_sort` | text_utils.rs:25 | move | db-only JSON sort helper |
| `get_category_enum` | enum_map.rs:6 | move? | koji-db already wraps it; thin. OPTIONAL |
| `enum_bridge!` macro | lib.rs:51 | move? | `#[macro_export]`, db-only consumer. Fiddly. OPTIONAL |

---

## B. Stays put (DAG-pinned / internally coupled) — WHY, no action
- **geometry::*** — koji-core internal use + sibling consumers (db,dragonite,algorithms,plugins) → LCA koji-core.
- **ApiQueryArgs cluster** (ApiQueryArgs/Filters/OutputSpec/PropertySelection/FeatureRenderSpec/AdminReq*/BoundsArg/SpawnpointTth) — consumers span db+golbat+service siblings → LCA koji-core. Refactored TODAY; leave.
- **Mode, Category** — multi-sibling consumers → koji-core. **UnknownId, TrimPrecision, create_cell_map, most s2::*** — internal use / sibling consumers.
- **NameModifier, separate_by_comma, get_mode_acronym, clean** — koji-core-internal args/text layer (NameModifier fresh).

## C. Visibility tightening (intra-crate; pub → pub(crate) unless noted)
- koji-core: `sql_raw` (no caller — remove/pub(crate))
- koji-db (~12): sea-orm `Relation` enums; NameId, InsertsUpdates, VecToJson, GeofenceNoGeometry, OnlyParent, RouteNoGeometry, OnlyGeofenceId, HierarchyArgError, Basic, FullPropertyModel, parse_order
- koji-golbat (8): gym/pokestop/spawnpoint/station `Model`+`Query`
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
| `65b625c` | A3 — count_in_area/AreaPolygons/sql_raw_bbox → koji-golbat (HasLatLon stayed — orphan rule) |
| `1867656` | A4 — json_related_sort + get_category_enum (merged) + enum_bridge! macro → koji-db |
| `913f7a6` | removed dead sql_raw + orphaned feature_single_vec from koji-core |
| `fa3fbe8` | visibility: 9 internal items → pub(crate) (db/events); 5 auto-reverted (real interface leaks) |
| `7004a27` | visibility: 71 in-crate-only items → pub(crate) (koji-service) |
| `4a4d5e8` | Category domain enum → koji-db (fresh-scan-found miss; beside its sea-orm storage twin) |
| `f315a3d` | dead code: vrp.rs + genetic.rs + debug_string/get_sorted removed (~681 LOC, owner-approved) |

**Fresh-eyes scan corrected the first agents:** sec/* is ACTIVE (greedy clustering) — NOT dead; dragonite/events types mostly used; OutputConfig fields read. Prevented bad deletes.

**Left in place (ambiguous — exported, no live caller, possibly intended API; flag only):** `s2::ToGeo`, `s2::Dir`, `koji-events::DispatchError`.

**koji-core now exports only genuinely-shared vocabulary:** `Mode`, `HasLatLon`, the ApiQueryArgs read-layer, `s2::*`, text helpers (NameModifier/clean/get_mode_acronym/separate_by_comma), `UnknownId`, `geometry::*`, `TrimPrecision`.

---

## 2026-06-16 fresh re-scan (delta) — codebase moved since 06-14

Same brief re-run. Substantial refactoring landed since 06-14 (ApiQueryArgs **resolve layer** `88ac9bf`→`d5017ed`, greedy Better/Best→crucible + #253 axe, koji-jobs hardened, nominatim cleaned). Method: 4 parallel investigators (DAG · koji-core · mid-tier+jobs · top-tier+revalidation) → main-thread ground-truth reads → 1 **adversarial** verifier told to refute every claim. Everything makes sense.

### DAG correction (vs the 06-14 line)
`koji-jobs`, `nominatim`, `migration` are **roots** (no `koji-core` dep). `koji-jobs` consumed only by koji-service + koji-cli. `koji-wasm` (L4, core+algorithms only) sits **below** `koji-service` (L5); neither depends on the other. Verified order:
`{macros, koji-jobs, nominatim, migration} < koji-core < {koji-db, koji-dragonite, koji-events, koji-plugins, koji-golbat} < algorithms < koji-wasm < koji-service < {koji-cli, koji-server}`.

### Prior A1–A4: ALL INTACT, zero regression
12 symbols re-verified in their post-06-14 homes (ClusteringConfig/RoutingConfig/BootstrapConfig/S2Config/CalculationMode/ClusterMode/SortBy in `algorithms`; OutputConfig/DevConfig/DataFilter/ReturnTypeArg/get_return_type in `koji-service::requests::config`). No leak-back to core.

### ONE new candidate — `query_args.rs` db-read layer (core → koji-db)
`crates/koji-core/src/query_args.rs` is a grab-bag of **3 request concerns with different consumers**:

| group | symbols | real consumer | verdict |
|---|---|---|---|
| geofence render-resolve | `ApiQueryArgs` `Filters` `PropertySelection` `OutputSpec` `FeatureRenderSpec` + resolve impls | **koji-db only** (`geofence.rs` `project_as_feature`/`project_as_koji`; service = doc-comment only at `resources.rs:27`; golbat/wasm = none) | **MOVE → koji-db** *(judgment call — see caveat)* |
| admin pagination | `AdminReq` `AdminReqParsed` | **koji-db only** (`geofence`/`route`/`project`/`property`/`tile_server` reads; no service `web::Query` wiring) | **MOVE → koji-db** |
| golbat bounds | `BoundsArg` `SpawnpointTth` | golbat (`entities/spawnpoint.rs`) **+** service (`s2.rs`/`golbat_data.rs`) → multi-crate | **STAYS core** |

Coupling if moved: `impl From<&ApiQueryArgs> for NameModifier` (`text_utils.rs:104`) moves to koji-db (orphan-ok: `ApiQueryArgs` becomes db-local; `NameModifier` stays core, db imports it). `query_args.rs` would retain only `BoundsArg`+`SpawnpointTth`. ~5-6 files (core query_args/text_utils/lib + db new module/geofence imports + 2 db test import paths).

**Caveat (why not auto-applied):** freshly-authored 06-16 code (user-planned `88ac9bf`); the 06-14 pass *deliberately* kept the cluster in core as "API request vocabulary," and service-side deserialization isn't wired yet (looks mid-integration). Reversing a 2-day-old documented decision on hot code → user sign-off first.

### False positives rejected (verified — do NOT move)
- `RoutingConfig`/`BootstrapConfig`/`SortBy` — params to `routing::main`/`bootstrap::main`/`bootstrap::radius`/`bootstrap::s2` **intra-algorithms** (cross-crate import greps miss this). Stay in algorithms.
- `NameModifier` — core-internal: `FeatureRenderSpec` field + `From<&ApiQueryArgs>` impl + `lib.rs` re-export; zero external imports. Stays.
- `KojiGeojsonError` — koji-core's **own** geometry `TryFrom` impls use it (`koji_geojson.rs:26,45`); not service-only. Stays.

### Out of relocation scope (visibility/dead-code — see §C/§D above)
koji-dragonite unused pub surface (`V2Envelope`/`V2ApiError`/`V2Meta`/`parse_v2`/`parse_v2_with_meta`/`Tri`) — pub→pub(crate) candidates, not relocations.

**Bottom line: codebase is already well-organized — the 06-14 pass holds. The only fresh opportunity is the `query_args.rs` db-layer, and it's a judgment call.**

### As-built (executed 2026-06-16, user chose "Move → koji-db")

Moved `koji-core` → `koji-db`: the ApiQueryArgs resolve cluster (`ApiQueryArgs`/`Filters`/`PropertySelection`/`OutputSpec`/`FeatureRenderSpec` + resolve impls) and `AdminReq`/`AdminReqParsed`, into new `koji-db/src/query_args.rs`. Kept in core: `BoundsArg`/`SpawnpointTth` (golbat+service consumers) + the generic text helpers (`clean`/`get_mode_acronym`/`separate_by_comma`).

**Two couplings the cross-crate greds couldn't see (caught at execution):**
1. **`NameModifier` had to travel too** → new `koji-db/src/name_modifier.rs`. Its `From<&ApiQueryArgs>` builder sets *private* fields (only constructible inside the owning crate), and the From impl must live where `ApiQueryArgs` is (orphan rule). Keeping it in core would have forced a 12-arg `pub` constructor / `pub` fields. It was db-render-only anyway (sole caller `spec.name_modifier.apply()`). Its 2 private helpers (`remove_symbols`, `convert_polish_to_ascii`) + 18 tests travelled with it; `koji-core` lost its now-orphaned `regex` dep, `koji-db` gained it.
2. **`macros` `koji_resource!` emits `AdminReqParsed`** in the generated `list` handler (a `quote!` template — invisible to import greps, expands in koji-service). Retargeted `koji_core::AdminReqParsed` → `koji_db::query_args::AdminReqParsed`. Resolves fine (koji-service depends on koji-db) and is more coherent (pagination input lives with the `paginate` that consumes it).

**Verification:** `cargo check --workspace --all-targets` clean; full `cargo test --workspace` = **1221 passed, 0 failed** (DB tests skip, no env). Parity-critical `NameModifier` presence-trigger tests pass in their new db home.

`koji-core` now truly exports only shared vocabulary; `query_args.rs` in core holds only the genuinely-multi-crate `BoundsArg`/`SpawnpointTth`.
