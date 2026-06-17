# Koji V2 — Goals & Decisions

## Driving goals (from user)
1. Job queue for the algorithms.
2. Rebalance crates / proper Rust workspace for the **whole repo**.
3. Multiple binaries: web server + algorithms CLI.
4. Correct Dragonite integration — **use its API, stop writing its DB directly**.
5. Drop **all** RDM mentions/support.
6. Managed state for the web server.
7. Fix the awful/inconsistent APIs — holistic v2 redesign, think outside the box.
8. Prepare support for Dragonite **PR #558** (per-mode geofence overrides).
9. Better Rust practices (idiomatic, async patterns).
10. Phase 1: **don't touch the client**; note where we'll have to later.
11. Proper separation of concerns (model crate is a dumping ground).
12. Break the `Args` super-struct into well-organized, composable arg groups.
13. Algorithm *logic* mostly untouched (only easy wins); restructure their *structs/inputs*.
14. Plugin system → best practices.
15. Push/event system: "Area X updated → notify APIs Y and Z."
16. All in **one branch / one PR, many commits**.

## Locked decisions (participate-mode Q&A)
| # | Decision | Choice |
|---|---|---|
| Q1 | Job queue execution model | **Persistent DB-backed**: jobs table → worker pool → `spawn_blocking`(rayon); pollable; CLI shares the same queue |
| Q2 | API redesign strategy | **Clean `/api/v2` + `/api/v1` as thin shim** over the v2 core. Client + Dragonite ride v1 in phase 1, migrate later |
| Q3 | Workspace / crate split | **Layered ~9 principal crates**, workspace manifest at **repo root**, crates under `crates/`; `client/` + `or-tools/` non-member siblings |
| Q4 | Event push architecture | **Durable outbox + dispatcher (retry/backoff) + webhook registry**; Dragonite v2-API pusher = first built-in subscriber (replaces inline rude writes) |
| Q5 | Plugin system | **Formalized subprocess**: Rust `Plugin` trait + external plugins dir + per-plugin manifest + versioned JSON protocol + error channel + run via `spawn_blocking` |

## Constraints
- **Migration strategy:** big-bang on a single long-lived branch, many ordered commits (each should compile).
- **Breaking changes:** none to live consumers in phase 1 — `/api/v1` shim + the frozen Dragonite calc contract preserve behavior. Client untouched.
- **Team:** solo (Rin).
- **Timeline:** none hard; scope-complete over correctness/quality.
- **Don't change:** the OR-Tools VRP solver vendoring; the ORM stays sea-orm; geojson-native domain (good alignment with Dragonite #557/#558).

## Deferred to design sections (not multiple-choice; proposed for sign-off)
- Arg-group struct shapes (Clustering/Routing/Bootstrap/S2/Output/AreaInput).
- Managed `AppState` shape + how calc endpoints bridge sync-Dragonite over the async queue.
- v2 resource model + endpoint inventory + how each v1 route maps onto it.
- Jobs/outbox/webhook DB schema; per-mode-fence model for PR #558.
- Domain-enum vs sea-orm-enum mapping (break the `Type`/`Category` coupling).
- Commit/phase sequence within the branch; client touch-point inventory.
