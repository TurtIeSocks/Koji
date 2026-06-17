# Plan 004: Batch the project-list geofence fetch (remove the N+1)

> **Executor instructions**: Follow this plan step by step. Run every verification command
> and confirm the expected result before moving on. If a "STOP condition" occurs, stop and
> report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 658d67f..HEAD -- crates/koji-db/src/db/project.rs`
> If the file changed, compare the "Current state" excerpt against the live code; on
> mismatch, STOP.

## Status

- **Priority**: P2
- **Effort**: M
- **Risk**: MED (must preserve the exact response JSON shape — characterization test first)
- **Depends on**: none (independent of the geofence split, plan 006)
- **Category**: perf
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

`project::Query::paginate` issues **one extra geofence query per project row**: it calls
`get_related_geofences().into_json().all(db)` inside a `try_join_all` over the page's
results (`crates/koji-db/src/db/project.rs:105-110`). A page of 50 projects = 1 + 50
queries. The repo already has the batched pattern to copy (`geofence.rs:1076-1102` fetches
properties for *all* ids in a single `Column::Id.is_in(ids)` query, then groups in memory).
This plan replaces the per-row fan-out with two batched queries while keeping the response
byte-identical.

## Current state

`crates/koji-db/src/db/project.rs:84-140` — `pub async fn paginate(...)`. The N+1 (lines
105-110):

```rust
let geofences = future::try_join_all(
    results
        .iter()
        .map(|result| result.get_related_geofences().into_json().all(db)),
)
.await?;

let mut results: Vec<Json> = results
    .into_iter()
    .enumerate()
    .map(|(i, project)| {
        json!({
            "id": project.id,
            "name": project.name,
            // ... other project fields ...
            "geofences": geofences[i],   // <-- per-project Vec<Json> of its geofences
        })
    })
    .collect();
```

- `geofences[i]` is a `Vec<JsonValue>` of that project's geofences, each as its full
  column-json (`.into_json()` on the geofence entity).
- The link is the `geofence_project` junction entity. The geofence entity declares
  `impl Related<super::geofence_project::Entity> for Entity` (`geofence.rs:84`). Read
  `crates/koji-db/src/db/geofence_project.rs` to confirm its column names (expected
  `ProjectId`, `GeofenceId`) before writing the batched query.
- **Exemplar to copy** (the in-repo batched pattern) — `geofence.rs:1076-1102`:
  ```rust
  let name_map: HashMap<u32, String> = Entity::find()
      .filter(Column::Id.is_in(ids.clone()))
      .select_only().column(Column::Id).column(Column::Name)
      .into_model::<NameId>().all(db).await?
      .into_iter().map(|m| (m.id, m.name)).collect();
  // ... a second is_in(...) query, grouped into a HashMap<u32, Vec<_>> ...
  ```
- `Json` here is `serde_json::Value`; `future`, `json!`, `HashMap` are already imported in
  `project.rs` (it currently uses `future::try_join_all`). Verify imports before adding.

## Commands you will need

| Purpose | Command                                          | Expected         |
|---------|--------------------------------------------------|------------------|
| Build   | `cargo build -p koji-db`                          | exit 0           |
| Lint    | `cargo clippy -p koji-db --all-targets -- -D warnings` | exit 0     |
| Unit    | `cargo test -p koji-db`                           | all pass (DB tests skip without `KOJI_DB_URL`) |
| DB test | `set -a; source ./.env.test; set +a; cargo test -p koji-db paginate` | all pass (only if a test DB is available) |

## Scope

**In scope:**
- `crates/koji-db/src/db/project.rs` — the `paginate` function only.
- A characterization test (see Test plan) — in `crates/koji-db/tests/paginate_db.rs` if a
  test DB is available, else inline reasoning only (see STOP conditions).
- `plans/README.md` — status row.

**Out of scope (do NOT touch):**
- The response JSON shape — every key, order, and value must stay identical. This plan is a
  query-count optimization, not a contract change.
- The other `project::Query` methods and any other db module.

## Git workflow

- Branch: `advisor/004-project-n-plus-1`.
- One commit: `perf(db): batch project-list geofence fetch (was N+1)`.
- Do NOT push or open a PR unless instructed.

## Steps

### Step 0: Lock the current behavior with a characterization test (if a DB is available)

`crates/koji-db/tests/paginate_db.rs` already exists and is gated on `KOJI_DB_URL` (it
skips cleanly when unset). If you have a test database (`.env.test` present), add a test
that seeds ≥2 projects each with ≥1 geofence, calls `project::Query::paginate`, and snapshots
the exact returned JSON (the `geofences` arrays included). This is the oracle the batched
version must reproduce. Model it after the existing tests in that file.

If **no** test DB is available, you cannot run behavior-equivalence — proceed, but the
"DB test" done-criterion is satisfied by review instead, and you must be extra strict that
step 1 is a pure batching change (same shape).

**Verify**: with a DB, `set -a; source ./.env.test; set +a; cargo test -p koji-db paginate`
→ the new snapshot test passes against the CURRENT (pre-change) code.

### Step 1: Replace the per-row fan-out with two batched queries

Replace the `try_join_all` block with:

1. Collect `let project_ids: Vec<u32> = results.iter().map(|p| p.id).collect();`.
2. One query over the `geofence_project` junction filtered `ProjectId.is_in(project_ids)`
   to get `(project_id, geofence_id)` pairs.
3. One query `geofence::Entity::find().filter(geofence::Column::Id.is_in(all_geofence_ids)).into_json().all(db)`
   → build `HashMap<u32 /*geofence id*/, JsonValue>` keyed by each geofence's `id`.
4. Build `HashMap<u32 /*project id*/, Vec<JsonValue>>` by walking the pairs and pulling each
   geofence's json from the map (preserve the same ordering the per-row query produced — if
   the original ordered geofences a particular way, replicate it; the characterization test
   from step 0 will catch a mismatch).
5. In the existing `.map(|project| json!({...}))`, set `"geofences"` to
   `geofence_map.get(&project.id).cloned().unwrap_or_default()`.

Keep every other field and the overall structure identical.

**Verify**:
- `cargo build -p koji-db` → exit 0.
- With a DB: the step-0 snapshot test still passes (byte-identical output).
- `grep -n "try_join_all" crates/koji-db/src/db/project.rs` → no match inside `paginate`.

## Test plan

- Characterization/snapshot test (step 0): seed ≥2 projects with geofences, assert the full
  `paginate` JSON (including per-project `geofences` arrays) — run it green BEFORE and AFTER
  step 1 to prove equivalence.
- Pattern: the existing DB tests in `crates/koji-db/tests/paginate_db.rs` (already uses the
  `KOJI_DB_URL`-gated, panic-safe cleanup template).
- Verification (with DB): `set -a; source ./.env.test; set +a; cargo test -p koji-db paginate`
  → all pass. Without DB: `cargo test -p koji-db` compiles + unit tests pass; behavior
  equivalence is by review.

## Done criteria

ALL must hold:

- [ ] `paginate` issues a bounded number of queries (the pagination count + 2), not one per
      row: `grep -n "try_join_all" crates/koji-db/src/db/project.rs` → no match.
- [ ] Response JSON shape unchanged (snapshot test passes if a DB is available; else review-confirmed).
- [ ] `cargo build -p koji-db` exits 0; `cargo clippy -p koji-db --all-targets -- -D warnings` exits 0.
- [ ] `cargo test -p koji-db` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- You cannot reproduce the exact per-project `geofences` JSON with a batched query (e.g. the
  per-row path applies a transform you can't replicate cleanly). Do NOT ship a shape change —
  report and leave the N+1 in place.
- `geofence_project`'s columns differ from the expected `ProjectId`/`GeofenceId`.
- The live `paginate` doesn't match the "Current state" excerpt (drift).

## Maintenance notes

- The same N+1 shape may exist in other list endpoints (`route.rs`, `geofence.rs`) — not in
  this plan's scope; flag separately if confirmed.
- A reviewer should check the query count dropped (2 batched queries) AND the response is
  byte-identical — both, not just one.
- If pagination later grows joins/filters on geofence fields, revisit the batching keys.
