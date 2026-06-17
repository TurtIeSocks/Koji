# Koji V2 — Phase 6: RDM / controller-write removal (design)

**Date:** 2026-05-29
**Status:** Approved-by-delegation (autonomous run). Async-review checkpoint = the **Assumptions** section. Hard-gate satisfied: P5 (events→Dragonite) is landed + E2E-verified (`d69dbe8`), so the controller-write path now has a replacement and may be removed (architecture §10, roadmap ordering rule).
**Parent:** architecture §10 ("RDM removal"); roadmap Phase 6.

## Goal
Delete the legacy RDM/Unown/Hybrid golbat abstraction and the **direct controller-DB write/read path**. `KojiDb` drops from **3 connections → 2** (`koji`, `golbat`); the old `controller` connection's role (pushing routes/geofences to the controller) is already replaced by the **Dragonite HTTP client + events** (P5). External Dragonite/golbat tables are untouched — Koji simply stops opening a controller connection.

## What goes away
- `koji_db::GolbatType` (enum + Serialize/Display/PartialEq) — single path, no auto-detect.
- `KojiDb.controller` + `KojiDb.golbat_type` fields.
- `get_database_struct`: the controller connection, the `SELECT name FROM instance` golbat-type probe, and the deprecated env fallbacks (`DATABASE_URL`→golbat, `UNOWN_DB_URL`/`UNOWN_DB`→controller). Keep only `KOJI_DB_URL` + `GOLBAT_DB_URL`.
- Controller-side entities: `koji_db::db::instance` (RDM `instance` table) and `koji_db::db::area` (controller `area` table); `RdmInstance*` + `InstanceParsing::Rdm` in `text.rs`; their `db/mod.rs` exports.
- api consumers of the above: `private/instance.rs` (the RDM/controller golbat-import internal routes) and every `conn.controller` write/read + `conn.golbat_type == GolbatType::Unown` branch in `public/v1/{calculate,route,geofence,project}.rs` and `utils/{mod,request,response}.rs`.

## Key decision — the `GolbatType::Unown` branches are TWO kinds; collapse each differently
Verified against `calculate.rs`: the `Unown` checks are **not** all controller-table selection. Two distinct kinds:

**(a) Output fence-type / mode naming** — `if Unown { circle_pokemon / CircleRaid / CirclePokemon } else { circle_smart_pokemon / CircleSmartRaid / CircleSmartPokemon }` (calculate.rs `__mode` ~L110, `enum_type` ~L210/L220). The `else` (non-Unown) arm is the **RDM** "smart" variant. **Unown is the surviving golbat family** (Koji targets the UnownHash/Dragonite stack; RDM is what P6 removes), so these collapse by **keeping the Unown arm** — emit the plain `circle_pokemon` / `CircleRaid` / `CirclePokemon` / `CircleStation` / `CircleQuest` types; the RDM `…Smart…` variants stop being produced (left in the `Type` enum, dead, until a P7 cleanup).

**(b) Controller-table selection / writes** — `if Unown { area::upsert(controller) } else { instance::upsert(controller) }` (save_to_golbat; route/geofence/project push). **Both arms hit `conn.controller`**, which is being removed → the whole operation is **deleted**:
- **`load_feature` (utils/mod.rs):** drop the entire `Err(_)` fallback. A geofence resolves from Koji's own `geofence` table or it 404s — no RDM-area / Unown-instance fallback.
- **v1 calc `save_to_golbat` (calculate.rs):** the controller-write side-effect + `update_project_api(golbat_type)` are removed. Calc still computes + returns + may `save_to_db` (Koji). "Push to golbat" is now the v2 `POST /geofences/:id/publish` (events→Dragonite, P5).
- **v1 `push_to_prod` (geofence/route/project) + `private/instance.rs`:** removed (controller writes / RDM imports with no backing once the connection is gone). The v2 publish path supersedes them.

## CalcPayload
Drop `golbat_is_unown` (it only selected the Unown vs RDM `FenceType` default for cluster output). The single canonical `FenceType` mapping is kept (the non-Unown path). `CalculateHandler` + `run_calc` updated.

## Sub-commit plan (each compiles + tests green)
1. **api: stop using `controller` / `golbat_type`.** Remove the controller fallback in `load_feature`; delete the v1 `push_to_prod` controller writes + the calc controller side-effects; delete `private/instance.rs` + its route wiring; drop `golbat_type` from `utils/{request,response}.rs`; collapse the `Unown` branches; drop `CalcPayload.golbat_is_unown`. Compiles against the still-3-conn koji-db. Green.
2. **koji-db: remove the controller conn + GolbatType + RDM entities.** `KojiDb {koji, golbat}`; `get_database_struct` keeps only KOJI/GOLBAT urls; delete `db/instance.rs` + `db/area.rs`; strip `RdmInstance*`/`InstanceParsing::Rdm` from `text.rs` + `db/mod.rs`. Green.
3. **cleanup:** `.env`/docs note that `CONTROLLER_DB_URL` + the RDM env vars are no longer read.

Order matters: api stops *referencing* the fields (step 1) before koji-db *removes* them (step 2), so no intermediate breaks.

## Verification
`cargo build` (whole workspace — package is `koji`) + `cargo test -p koji-db -p api -p koji-dragonite`. Runtime re-check against the live dev DBs + the running koji-server: v1 + v2 calc/CRUD still work with **no** `CONTROLLER_DB_URL` set; the v2 geofence publish→Dragonite path (P5) still delivers.

## Assumptions (DELEGATE CHECKPOINT)
1. **v1 `push_to_prod` (route/geofence/project) + `private/instance.rs` are removed, not re-pointed.** They were controller writes / RDM golbat-imports; the controller connection is gone and the v2 events→Dragonite publish replaces "push". v1 is best-effort legacy; Dragonite migrates to v2. (If the maintainer wants v1 push to *emit events* instead of being removed, that's a small follow-up reusing the P5 publish path.)
2. **`save_to_golbat` is dropped as a calc side-effect** (was a controller write). `save_to_db` (Koji) is unaffected.
3. **No RDM/Unown/Hybrid distinction survives anywhere** — single golbat path (golbat read via `koji-golbat`); Koji owns geofences; Dragonite is reached only over HTTP/events.
4. **External DBs are never dropped** — Koji stops opening the controller connection; it issues no `DROP` against Dragonite/golbat.
5. **`CONTROLLER_DB_URL` becomes unused** (left in `.env` harmlessly; documented as ignored). The server no longer panics if it is absent.
6. **The `area_fence` table + `geofence.dragonite_area_id`** (P3 migration) stay — they are the *new* linkage, unrelated to the removed controller `area` entity.
