# Koji V2 — Phase 0: Workspace Skeleton Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Relocate the Cargo workspace from `server/` to the repository root (`crates/` + `bins/`) with zero behavior change — everything compiles and the existing test set passes exactly as before.

**Architecture:** A virtual workspace manifest at the repo root globs `crates/*` and `bins/*`. The six library crates move verbatim into `crates/` (their `../sibling` path deps stay valid). The `koji` binary moves to `bins/koji-server/` (its one path dep, `api`, is repointed). No package renames, no `use`-statement churn — `koji-*` renaming is deferred to the phases that rewrite each crate's internals (P1+). A `[workspace.dependencies]` table is scaffolded; per-crate adoption is deferred per the spec convention.

**Tech Stack:** Rust edition 2024, Cargo virtual workspace (resolver 3), sea-orm, actix-web. Build/CI via Dockerfile (`docker.yml` builds it with root context).

**Reference:** `docs/superpowers/specs/2026-05-28-koji-v2-architecture-design.md` §3, `refactor-workspace/roadmap.md` Phase 0.

---

## File structure (before → after)

```
server/Cargo.toml              → /Cargo.toml            (rewritten: virtual workspace, no [package])
server/Cargo.lock              → /Cargo.lock            (moved; root lock)
server/.env.example            → /.env.example          (moved)
server/algorithms/             → crates/algorithms/     (verbatim)
server/api/                    → crates/api/            (verbatim)
server/macros/                 → crates/macros/         (verbatim)
server/migration/              → crates/migration/      (verbatim)
server/model/                  → crates/model/          (verbatim)
server/nominatim/              → crates/nominatim/      (verbatim)
server/src/main.rs             → bins/koji-server/src/main.rs   (moved)
(new)                          → bins/koji-server/Cargo.toml    (created)
server/target/                 → (deleted; regenerated at /target)
Dockerfile, .dockerignore, .gitignore, .vscode/launch.json  (path edits)
```

Package names are unchanged (`koji`, `algorithms`, `api`, `macros`, `migration`, `model`, `nominatim`). The produced binary stays named `koji`.

---

### Task 1: Capture the green baseline (before touching anything)

**Files:** none (read-only).

- [ ] **Step 1: Build the current tree**

Run (from repo root): `cargo build --manifest-path server/Cargo.toml --workspace`
Expected: success. (Long; run in background.)

- [ ] **Step 2: Record the current test result as the baseline**

Run: `cargo test --manifest-path server/Cargo.toml --workspace 2>&1 | tail -30`
Expected: note the pass/fail/ignored counts. This exact set must still pass after the move. (The `partition.rs` `bench_nh_*` tests skip when `points-nh.csv` is absent — that's expected and unchanged by the move.)

---

### Task 2: Create the root virtual workspace manifest

**Files:**
- Create: `Cargo.toml` (repo root)

- [ ] **Step 1: Write the root workspace manifest**

```toml
[workspace]
resolver = "3"
members = ["crates/*", "bins/*"]

[workspace.dependencies]
# Internal crates
macros = { path = "crates/macros" }
# Shared external deps — declared centrally; each crate migrates to
# `dep = { workspace = true }` when its own phase touches it (see spec §2).
log = "0.4.28"
serde = { version = "1.0.225", features = ["derive"] }
serde_json = "1.0.145"
geojson = "0.24.2"
geo = "0.31.0"
chrono = "0.4.42"
thiserror = "2.0.16"
reqwest = "0.12.23"
```

> Note: `crates/macros` does not exist yet (created in Task 3); cargo only resolves it at build time (Task 6), so writing this first is fine.

---

### Task 3: Relocate the six library crates and the binary

**Files:**
- Move: `server/{algorithms,api,macros,migration,model,nominatim}` → `crates/`
- Move: `server/src` → `bins/koji-server/src`
- Create: `bins/koji-server/Cargo.toml`

- [ ] **Step 1: Make the new directories and move the library crates**

```bash
mkdir -p crates bins/koji-server
git mv server/algorithms crates/algorithms
git mv server/api        crates/api
git mv server/macros     crates/macros
git mv server/migration  crates/migration
git mv server/model      crates/model
git mv server/nominatim  crates/nominatim
```

> No edits needed inside these six `Cargo.toml`s: their inter-crate deps use `../sibling` (e.g. `model = { path = "../model" }`), which still resolves now that they're siblings under `crates/`. `algorithms`'s `macros = { workspace = true }` is satisfied by the root manifest from Task 2.

- [ ] **Step 2: Move the binary's source**

```bash
git mv server/src bins/koji-server/src
```

- [ ] **Step 3: Create the binary's manifest** (repoints the one path dep `api`)

Create `bins/koji-server/Cargo.toml`:
```toml
[package]
name = "koji"
version = "1.5.4"
edition = "2024"
publish = false

[dependencies]
api = { path = "../../crates/api" }
dotenv = "0.15.0"
env_logger = "0.11.8"
log = "0.4.28"
tikv-jemallocator = "0.6"
```

> Package name stays `koji`, default bin from `src/main.rs` → binary named `koji` (matches the Dockerfile `cargo install` + `CMD koji`).

---

### Task 4: Move the lockfile + env example; remove the superseded manifest

**Files:**
- Move: `server/Cargo.lock` → `Cargo.lock`; `server/.env.example` → `.env.example`
- Delete: `server/Cargo.toml`, `server/target/`, empty `server/`

- [ ] **Step 1: Move lockfile + env example to root**

```bash
git mv server/Cargo.lock Cargo.lock
git mv server/.env.example .env.example
```

- [ ] **Step 2: Remove the old workspace manifest (superseded by root) + stale target**

```bash
git rm server/Cargo.toml
rm -rf server/target        # untracked / git-ignored build cache
rmdir server                # should now be empty
```

- [ ] **Step 3: Confirm `server/` is gone**

Run: `ls -d server 2>&1 || echo "server/ removed"`
Expected: `server/ removed`.

---

### Task 5: Verify the relocated workspace builds + tests green, then commit

**Files:** none (verification + commit).

- [ ] **Step 1: Build the workspace from the root**

Run: `cargo build --workspace`
Expected: success (cargo updates `Cargo.lock` for the new member paths). Run in background — this is a full rebuild.

- [ ] **Step 2: Run the test suite**

Run: `cargo test --workspace 2>&1 | tail -30`
Expected: the same pass/fail/ignored counts captured in Task 1 Step 2. No new failures.

- [ ] **Step 3: Format + lint check**

Run: `cargo fmt --check && cargo clippy --workspace --all-targets`
Expected: fmt clean; clippy no new errors (warnings unchanged from baseline are acceptable).

- [ ] **Step 4: Commit the relocation**

```bash
git add -A
git commit -m "refactor(workspace): relocate cargo workspace to repo root (crates/ + bins/)

Virtual workspace at root globs crates/* + bins/*; six library crates
move verbatim (sibling ../path deps preserved); koji bin → bins/koji-server.
Scaffolds [workspace.dependencies]. No package renames, no behavior change.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: Update the Dockerfile server build stage

**Files:**
- Modify: `Dockerfile` (the `server` stage)

- [ ] **Step 1: Replace the `COPY ./server .` + install path**

In `Dockerfile`, change the `server` stage from:
```dockerfile
FROM rust:1.93-bookworm AS server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY ./server .
RUN apt-get update && apt-get install -y
RUN cargo install --path . --locked
```
to:
```dockerfile
FROM rust:1.93-bookworm AS server
ENV PKG_CONFIG_ALLOW_CROSS=1
WORKDIR /usr/src/koji
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bins ./bins
RUN apt-get update && apt-get install -y
RUN cargo install --path bins/koji-server --locked
```

> The `or-tools`, `client`, and `runner` stages are unchanged — `COPY --from=server /usr/local/cargo/bin/koji` still works (binary name unchanged). The `or-tools` stage's `/algorithms/src/routing/plugins` is a runtime data dir, independent of the cargo move (the runtime plugins-dir relocation to `KOJI_PLUGINS_DIR` is Phase 7, out of scope here).

---

### Task 7: Update ignore files + VS Code launch path

**Files:**
- Modify: `.dockerignore`, `.gitignore`, `.vscode/launch.json`

- [ ] **Step 1: `.dockerignore` — point target ignores at the root**

Change the line `server/target` to:
```
target
**/target
```
(Leave `client/node_modules`, `dist`, `.github`, `.vscode`, etc. as-is.)

- [ ] **Step 2: `.gitignore` — update server-prefixed paths**

Replace:
```
# server
server/target
vrp_tests
server/debug_files/*
server/algorithms/src/**/plugins/**/*
!server/algorithms/src/**/plugins/.gitkeep
target/
```
with:
```
# workspace
vrp_tests
debug_files/*
crates/algorithms/src/**/plugins/**/*
!crates/algorithms/src/**/plugins/.gitkeep
target/
```

- [ ] **Step 3: `.vscode/launch.json` — fix the debug binary path**

Change `"${workspaceFolder}/server/target/debug/koji"` to `"${workspaceFolder}/target/debug/koji"`. If the same config sets `"cwd": "${workspaceFolder}/server"`, change it to `"${workspaceFolder}"`.

- [ ] **Step 4: (Optional) Refresh the stale comment in `crates/algorithms/src/clustering/partition.rs:580`**

The comment `// CSV expected at workspace root (one level up from server/).` is now inaccurate. Change to `// CSV expected at the repo root, or one level up (crate dir).`. No code change — the existence-checks at lines 581-588 already handle both locations and skip gracefully.

- [ ] **Step 5: Commit build/tooling updates**

```bash
git add Dockerfile .dockerignore .gitignore .vscode/launch.json crates/algorithms/src/clustering/partition.rs
git commit -m "chore(build): update Dockerfile + ignores + vscode for root workspace layout

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

- [ ] **Step 6: (Optional) Smoke-test the Docker build**

Run: `docker build -t koji:p0 . 2>&1 | tail -20`
Expected: build succeeds through all stages. Skip if Docker is unavailable locally; CI (`docker.yml`) will exercise it on push.

---

## Self-Review

**Spec coverage (architecture spec §3 / roadmap P0):**
- ✅ Workspace manifest at repo root (Task 2).
- ✅ Crates under `crates/`, bins under `bins/` (Tasks 3–4).
- ✅ `[workspace.dependencies]` scaffolded (Task 2); per-crate adoption deferred per convention — intentional, not a gap.
- ✅ Dockerfile + CI path updates (Task 6; CI uses root context so the Dockerfile edit suffices).
- ✅ `koji-*` rename deferred — documented refinement; later phases handle renames when they rewrite each crate.
- ✅ Zero behavior change verified by baseline diff (Task 1 vs Task 5).

**Placeholder scan:** No TBD/TODO; all commands and file contents are concrete.

**Type/path consistency:** Binary package name `koji` is consistent across `bins/koji-server/Cargo.toml`, the Dockerfile `cargo install --path bins/koji-server`, and `COPY … /cargo/bin/koji`. Inter-crate `../sibling` paths verified to remain valid under `crates/`. The only repointed dep is `koji`'s `api` → `../../crates/api`.

**Out of scope (later phases):** package `koji-*` renames; per-crate `workspace = true` dep adoption + upgrade-to-latest; runtime plugins-dir relocation (`KOJI_PLUGINS_DIR`); model split.
