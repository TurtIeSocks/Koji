# Plan 006: Split the `geofence.rs` god-file into a focused module directory

> **Executor instructions**: Follow this plan step by step. This is a **pure relocation**
> refactor — you move code, you do not change any logic. Run the build + tests after EVERY
> group you move. If a "STOP condition" occurs, stop and report. When done, update this
> plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 658d67f..HEAD -- crates/koji-db/src/db/geofence.rs`
> If the file changed, re-read it and re-derive the group boundaries before proceeding.

## Status

- **Priority**: P3
- **Effort**: L
- **Risk**: MED (large mechanical move; sea-orm derive macros + cross-method private helpers)
- **Depends on**: none, but do LAST — it touches the file other plans (003 is elsewhere; 004
  is `project.rs`) don't, yet it's the riskiest. Land 001–005 first.
- **Category**: tech-debt
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

`crates/koji-db/src/db/geofence.rs` is **1631 LOC** — ~12× the repo's file median — and its
`impl Query` block holds ~25 async methods spanning reads, pagination, writes/upserts,
project-projection, and hierarchy. Every geofence database path flows through this one file,
which makes it hard to review and risky to change. Splitting it by responsibility (no logic
change) shrinks each unit to something a reviewer can hold in their head, with the public
path `koji_db::db::geofence::Query::*` unchanged so **no caller is affected**.

## Current state

`crates/koji-db/src/db/geofence.rs` structure (line anchors from commit `658d67f`):

- **Entity / Model / Relations / ActiveModel** + struct defs + helper impls — lines ~1–567:
  the sea-orm `#[derive(...)]` entity, `impl Related<...>` (×4, lines 69–90),
  `impl ActiveModelBehavior`, `GeofenceNoGeometry` (114), `impl Model` (218, 388),
  `VecToJson` impls (412, 418), `HierarchySpec` + `impl HierarchySpec::from_args` (456–567).
- **`impl Query`** — lines ~568–1280, ~25 `pub async fn`:
  - reads: `descendants`(580), `get_one`(617), `get_one_json`(629),
    `get_one_json_with_related`(636), `get_all`(689), `get_all_json`(694),
    `get_all_koji`(704), `get_one_koji`(717), `get_all_no_fences`(726), `search`(1188).
  - list/cache: `paginate`(747), `get_json_cache`(742).
  - writes: `update_related_route_names`(845), `upsert_related_properties`(858),
    `upsert_related_projects`(906), `upsert`(921), `upsert_json_return`(960), `delete`(970),
    `upsert_from_geometry`(1000), `assign`(1196).
  - project-projection: `by_project`(1040) + the private `get_helper_maps` it calls
    (1040–1113), `project_as_feature`(1117), `project_as_koji`(1170), `by_parent_koji`(1242),
    `unique_parents`(1268).

Key Rust facts that make this safe:
- An **inherent `impl Query { ... }` block can be split across multiple files** in the same
  module. Each new file does `impl super::Query { /* moved methods */ }`.
- Moving a module from `geofence.rs` to `geofence/mod.rs` + submodules **does not change**
  the public path `koji_db::db::geofence::Query::X` as long as the items are re-exported /
  declared at the module root.
- Tests: `crates/koji-db/tests/geofence_deep_db.rs` + others exercise these methods but are
  **gated on `KOJI_DB_URL`** (they skip without a database). Compile-equivalence is the
  primary guarantee for a DB-less executor; behavior-equivalence needs the test DB.

## Commands you will need

| Purpose  | Command                                                       | Expected |
|----------|---------------------------------------------------------------|----------|
| Build    | `cargo build -p koji-db`                                       | exit 0   |
| Lint     | `cargo clippy -p koji-db --all-targets -- -D warnings`         | exit 0   |
| Tests    | `cargo test -p koji-db`                                        | all pass (DB tests skip without env) |
| DB tests | `set -a; source ./.env.test; set +a; cargo test -p koji-db`    | all pass (only if a test DB exists) |

## Scope

**In scope:**
- Convert `crates/koji-db/src/db/geofence.rs` → `crates/koji-db/src/db/geofence/` (a module
  directory: `mod.rs` + grouped submodules).
- `crates/koji-db/src/db/mod.rs` (or wherever `mod geofence;` is declared) — no change should
  be needed (the module name stays `geofence`), but verify.
- `plans/README.md` — status row.

**Out of scope (do NOT touch):**
- **Any method body / signature / logic.** This is a move only. If a method needs an edit to
  compile after moving (beyond adjusting `use`/visibility), STOP — it signals a coupling to
  untangle deliberately, not silently.
- Callers of `geofence::Query::*` anywhere in the workspace — the public path must not change.
- Other db modules (`route.rs`, `project.rs`, etc.).

## Git workflow

- Branch: `advisor/006-split-geofence`.
- One commit per moved group (so a bisect can pinpoint a regression), message style:
  `refactor(db): extract geofence <group> into geofence/<file>.rs (pure move)`.
- Do NOT push or open a PR unless instructed.

## Steps

> After EVERY step below: `cargo build -p koji-db` → exit 0, then `cargo test -p koji-db` →
> all pass. If a DB is available, run the DB-gated suite too. Do not proceed on a red build.

### Step 0: Baseline

Run `cargo test -p koji-db` (and the DB suite if available) and record that it's green
BEFORE any move. This is the oracle.

### Step 1: Create the module directory, move the entity unchanged

- Create `crates/koji-db/src/db/geofence/mod.rs`.
- Move the ENTIRE current contents of `geofence.rs` into `geofence/mod.rs` verbatim. Delete
  `geofence.rs`. (Now it's a directory module with identical content.)

**Verify**: `cargo build -p koji-db` → exit 0; `cargo test -p koji-db` → all pass. (Pure
relocation of the whole file; nothing else changed.) Commit.

### Step 2: Extract the read methods → `geofence/reads.rs`

- Create `geofence/reads.rs` starting with the `use` items the moved methods need (copy the
  relevant `use` lines from `mod.rs`; keep them in `mod.rs` too if still used there).
- Move these `impl Query` methods into an `impl super::Query { ... }` block in `reads.rs`:
  `descendants`, `get_one`, `get_one_json`, `get_one_json_with_related`, `get_all`,
  `get_all_json`, `get_all_koji`, `get_one_koji`, `get_all_no_fences`, `search`.
- In `mod.rs`, add `mod reads;`.
- Adjust visibility only if the compiler demands it (e.g. a private helper now used across
  files becomes `pub(super)`). Do NOT change any method body.

**Verify**: build + tests green. Commit.

### Step 3: Extract list/cache → `geofence/list.rs`

Move `paginate`, `get_json_cache` into `impl super::Query` in `geofence/list.rs`; `mod list;`
in `mod.rs`. Build + tests green. Commit.

### Step 4: Extract writes → `geofence/writes.rs`

Move `update_related_route_names`, `upsert_related_properties`, `upsert_related_projects`,
`upsert`, `upsert_json_return`, `delete`, `upsert_from_geometry`, `assign`. Build + tests
green. Commit.

### Step 5: Extract project-projection → `geofence/project_view.rs`

Move `by_project`, the private `get_helper_maps` it calls, `project_as_feature`,
`project_as_koji`, `by_parent_koji`, `unique_parents`. `get_helper_maps` likely needs
`pub(super)` if any other group calls it (check; if only this group uses it, keep it private
in this file). Build + tests green. Commit.

### Step 6: (Optional) Extract `HierarchySpec` → `geofence/hierarchy.rs`

If `mod.rs` is still large, move `HierarchySpec` + `impl HierarchySpec` into
`geofence/hierarchy.rs` and re-export it from `mod.rs` (`pub use hierarchy::HierarchySpec;`)
so the path `geofence::HierarchySpec` is preserved. Build + tests green. Commit.

### Step 7: Confirm the public surface is unchanged

The sea-orm entity (`Entity`/`Model`/`Column`/`Relation`/`ActiveModel`) and `pub struct Query`
stay declared in `mod.rs`. Confirm no external caller broke:

**Verify**: `cargo build --workspace` → exit 0 (this compiles every caller of
`geofence::Query::*`). `cargo test --workspace` → all pass.

## Test plan

- No new tests — this is a behavior-preserving refactor. The existing DB-gated suites
  (`geofence_deep_db.rs`, `paginate_db.rs`, etc.) are the regression net; run them green
  before (step 0) and after (step 7). With a DB, that's behavior-equivalence; without, the
  guarantee is compile-equivalence + the "pure move, no body edits" discipline.
- Verification: `cargo test --workspace` exits 0 at step 7 (and the DB suite if available).

## Done criteria

ALL must hold:

- [ ] `crates/koji-db/src/db/geofence.rs` no longer exists; `geofence/mod.rs` + the grouped
      submodules do.
- [ ] `geofence/mod.rs` is materially smaller than the original 1631 LOC (target < ~650).
- [ ] The public path is unchanged: `cargo build --workspace` exits 0 (all callers compile)
      and `grep -rn "geofence::Query::" crates bins` still resolves (no caller edited).
- [ ] No method body or signature changed — the diff is moves + `use`/visibility only.
      (`git diff` should show deletions in one place and identical insertions in another.)
- [ ] `cargo test --workspace` exits 0; with a DB, the geofence DB suites pass.
- [ ] `cargo clippy -p koji-db --all-targets -- -D warnings` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- A method requires a logic edit (not just `use`/visibility) to compile after moving — this
  means a coupling that needs a deliberate decision, not an improvised fix.
- The sea-orm derive macros fail to resolve after splitting (e.g. `Column`/`Relation` not
  found from a submodule) and the fix isn't a simple `use super::*;`.
- `cargo test --workspace` is red after any step and a single obvious `use`/visibility fix
  doesn't restore it.
- No test database is available AND the operator needs behavior-equivalence proven before
  merge — report that DB tests couldn't run.

## Maintenance notes

- Keep the sea-orm `Entity` and its derives together in `mod.rs`; sea-orm codegen expects the
  entity in one place.
- Future geofence methods should be added to the group file that matches their role, not back
  into `mod.rs`.
- A reviewer should diff with `git diff --color-moved=zebra` to confirm the change is pure
  relocation (moved lines, not rewritten ones).
- If plan 004 (project N+1) hasn't landed yet, it touches `project.rs`, not this file — no
  conflict; order doesn't matter between them.
