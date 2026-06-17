# Koji V2 — Module Assessment

Verdicts drive the old→new map. K=Keep, R=Refactor-in-place, RW=Rewrite/restructure, D=Delete, DEF=Defer.

| Module (today) | Verdict | Evidence | Tension w/ goals | Conf |
|---|---|---|---|---|
| `model/` (whole crate) | **RW** | `[COMPLEX][COUPLED]` 3 concerns fused (db+api+utils); db entities double as DTOs | Blocks #11 sep-of-concerns; #2 rebalance | High |
| `model/api/args.rs` | **RW** | `[COMPLEX]` 748 LOC / 37 fields / 6 deprecated | #12 break super-struct | High |
| `model/api/*` conversions, `GeoFormats`, `BBox` | **R→koji-core** | pure already, just mis-placed | #11 | High |
| `model/db/{geofence,route,project,property,…}` | **R→koji-db** | sea-orm entities w/ serde DTO bleed | #11 stop entity=DTO | High |
| `model/db/{gym,pokestop,spawnpoint,station}` | **R→koji-golbat** | read-only golbat access | #11 isolate golbat | High |
| `model/db/instance.rs` | **D** | RDM `instance`-table writes | #4 use API, #5 drop RDM | High |
| `model/db/area.rs` controller-write logic | **D** | direct `area`-table writes = the "rude" path | #4 | High |
| `model/db/area.rs` read-model | **R→koji-db / koji-dragonite DTO** | needed for round-trip | #8 #558 | Med |
| `model/lib.rs` `GolbatType`/`KojiDb` | **RW** | RDM/Hybrid branching; 3-conn struct | #5 #6; →2-conn | High |
| `model/utils/get_database_struct` | **RW→koji-service config** | golbat-type autodetect + deprecated env | #5 #6 managed state | High |
| `algorithms/` (clustering/routing/bootstrap/s2) | **K (logic) / R (signatures)** | `[HOT]` active; 15-primitive sigs | #13 keep logic, #12 struct inputs | High |
| `algorithms/plugin.rs` | **RW→koji-plugins** | no trait, in-source plugins, ad-hoc protocol | #14 best practices | High |
| `algorithms/stats.rs` | **K→koji-algorithms** | Stats threaded everywhere; Dragonite logs it | preserve contract | High |
| `api/` (whole crate) | **RW→koji-service + bins** | `[HOT]` 61 inconsistent routes | #7 API redesign, #3 bins | High |
| `api/.../calculate.rs` | **RW** | sync inline algorithm calls; controller writes | #1 queue, #4 events | High |
| `api/.../{geofence,route,project}.rs` push/{id} | **RW** | GET-that-mutates + direct writes | #7 verbs, #4 events | High |
| `api/private/admin.rs` generic `{resource}` | **RW→typed resources** | stringly-typed dispatch | #7 (D3 = typed) | High |
| `api/lib.rs start()` | **RW→managed AppState** | ad-hoc Data injection | #6 managed state | High |
| `nominatim/` | **K (rename koji-nominatim)** | standalone, fine | — | High |
| `macros/` | **K (rename koji-macros)** | standalone | — | High |
| `migration/` | **R→koji-migration** | add job/outbox/webhook/area_fence/dragonite_area_id; keep RDM-era migrations as history | #1 #4 #8 | High |
| `client/` | **DEF (phase 1)** | don't touch; touch-points listed in map | #10 | High |
| `or-tools/` | **K untouched** | vendored VRP solver | — | High |
| — NEW — | **koji-jobs, koji-events, koji-dragonite, koji-cli** | — | #1 #4 #3 | — |
