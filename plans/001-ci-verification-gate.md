# Plan 001: Add a CI verification gate (fmt + clippy + test) on push and PR

> **Executor instructions**: Follow this plan step by step. Run every verification
> command and confirm the expected result before moving on. If a "STOP condition"
> occurs, stop and report — do not improvise. When done, update this plan's row in
> `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 658d67f..HEAD -- .github/workflows`
> If `.github/workflows/` changed since this plan was written, compare the "Current
> state" excerpt against the live file before proceeding; on mismatch, STOP.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: dx
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

The only CI workflow in this repo (`.github/workflows/docker.yml`) builds and pushes a
Docker image — it never runs `cargo test`, `cargo clippy`, or `cargo fmt`. The project's
own roadmap (`refactor-workspace/roadmap.md`) states the invariant *"every commit compiles
+ passes existing tests,"* but **nothing enforces it**. Locally the suite is healthy (1221
tests pass, clippy clean), yet `rustfmt` has already drifted to 316 diffs precisely because
no gate catches it. After this plan, every push and PR is verified for formatting, lints,
and tests — regressions are caught before merge instead of by the next contributor.

## Current state

- `.github/workflows/docker.yml` — the ONLY workflow. Triggers on `push` only; a single
  `Docker` job that logs into ghcr.io and runs `docker/build-push-action`. No Rust
  verification anywhere. Excerpt:
  ```yaml
  name: Docker
  on: [push]
  jobs:
    Docker:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v3
        # ... docker login / metadata / buildx / build-push ...
  ```
- The workspace is Rust **edition 2024** (`Cargo.toml`), which needs Rust ≥ 1.85.
- Verification commands (run from repo root, verified working this session):
  - `cargo test --workspace` → all pass (DB-integration tests skip cleanly when
    `KOJI_DB_URL` is unset, so no database is needed in CI).
  - `cargo clippy --workspace --all-targets -- -D warnings` → expected clean (the roadmap
    records a prior `clippy --fix` deny-level pass).
  - `cargo fmt --all --check` → **currently reports 316 diffs** (this is why step 1 applies
    formatting first — otherwise the new gate is red on its first run).
- There is no `rust-toolchain.toml`. CI should pin the toolchain via the action.

## Commands you will need

| Purpose    | Command                                              | Expected on success |
|------------|------------------------------------------------------|---------------------|
| Format     | `cargo fmt --all`                                    | exit 0, files rewritten |
| Format chk | `cargo fmt --all --check`                            | exit 0, no diff     |
| Lint       | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0           |
| Tests      | `cargo test --workspace`                             | all pass, exit 0    |

## Scope

**In scope:**
- `cargo fmt --all` — a formatting-only commit that rewrites whitespace across the
  workspace `.rs` files (expected; no logic changes).
- `.github/workflows/ci.yml` (create).
- `plans/README.md` (status row).

**Out of scope (do NOT touch):**
- `.github/workflows/docker.yml` — it builds images; leave it as-is.
- Any source-code *logic* change. `cargo fmt` only moves whitespace; if it would change
  anything else, STOP.

## Git workflow

- Branch: `advisor/001-ci-verification-gate`.
- Two commits: (1) `style: apply rustfmt --all across workspace`, (2)
  `ci: add fmt + clippy + test verification workflow`. Conventional-commit style matches
  the repo's `git log`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Apply formatting so the new gate starts green

Run `cargo fmt --all`. This rewrites whitespace in many files — that is expected.

**Verify**:
- `cargo fmt --all --check` → exit 0 (no diffs).
- `cargo test --workspace` → all pass (formatting must not change behavior; if any test
  now fails, STOP — that means a non-whitespace change crept in).

Commit: `style: apply rustfmt --all across workspace`.

### Step 2: Add the CI workflow

Create `.github/workflows/ci.yml` exactly as below:

```yaml
name: CI
on:
  push:
  pull_request:

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - name: Cache cargo build
        uses: Swatinem/rust-cache@v2
      - name: Format check
        run: cargo fmt --all --check
      - name: Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings
      - name: Tests
        run: cargo test --workspace
```

**Verify**:
- File parses as YAML (no tabs; 2-space indent). If `actionlint` is installed,
  `actionlint .github/workflows/ci.yml` → exit 0. Otherwise inspect visually.
- The three commands in the workflow each pass locally (you already confirmed them in
  step 1 and the Commands table).

Commit: `ci: add fmt + clippy + test verification workflow`.

### Step 3 (optional): Dependency-advisory scan

`cargo-audit` is not installed in this repo and has never run. If the operator wants a
dependency-vulnerability gate, append this step to the `verify` job (start it
non-blocking so a single transitive advisory doesn't wedge every PR):

```yaml
      - name: Security audit
        uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}
        continue-on-error: true
```

Skip this step if not requested.

## Test plan

No new unit tests — this plan adds a CI gate, not code. Verification is that the three
gated commands pass locally (step 1 + step 2) and that the workflow file is valid YAML.
The first CI run on a PR is the live proof.

## Done criteria

ALL must hold:

- [ ] `.github/workflows/ci.yml` exists, triggers on `push` and `pull_request`, and runs
      fmt-check + clippy(`-D warnings`) + `cargo test --workspace`.
- [ ] `cargo fmt --all --check` exits 0.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- [ ] `cargo test --workspace` exits 0.
- [ ] `.github/workflows/docker.yml` is unchanged (`git diff 658d67f -- .github/workflows/docker.yml` is empty).
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report (do not improvise) if:

- After `cargo fmt --all`, `cargo test --workspace` fails — formatting should never change
  behavior; something is wrong.
- `cargo clippy --workspace --all-targets -- -D warnings` surfaces a large number of
  pre-existing warnings that are not trivially fixable. Report the count and the lints;
  do NOT blanket-`#[allow]` them or drop `-D warnings` to make it pass.
- The drift check shows `.github/workflows/` changed since `658d67f`.

## Maintenance notes

- If a `rust-toolchain.toml` is later added, change CI to honor it instead of
  `@stable`, so local and CI toolchains match.
- The fmt-check gate means contributors must run `cargo fmt` before pushing; consider a
  pre-commit hook as a follow-up (out of scope here).
- A reviewer should confirm the workflow ran (green check) on the PR that introduces it —
  a workflow that never triggers is a silent no-op.
