# Plan 005: Harden the crucible clusterer against non-finite coordinates

> **Executor instructions**: Follow this plan step by step. Run every verification command
> and confirm the expected result before moving on. If a "STOP condition" occurs, stop and
> report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 658d67f..HEAD -- crates/algorithms/src/clustering/crucible/refine.rs`
> If the file changed, compare the "Current state" excerpts against the live code; on
> mismatch, STOP.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW (defensive; the comparator fallback and the clamp only change behavior on
  already-degenerate, non-finite values)
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

`crucible` is the live clustering algorithm (it runs over thousands of points). Several sort
comparators do `f64::partial_cmp(...).unwrap()`, which **panics if either value is `NaN`**
(`partial_cmp` returns `None` for NaN). NaN can arise from the local-frame projection
(`LocalFrame::to_latlng` divides by `m_per_deg_lat`/`m_per_deg_lon`, which approach zero at
extreme latitudes). A panic mid-clustering kills the job. This plan makes the comparators
NaN-safe and floors the projection divisors so degenerate input degrades gracefully instead
of crashing. For normal Pokémon-GO coordinates nothing changes.

## Current state

`crates/algorithms/src/clustering/crucible/refine.rs` — four panicking comparators
(confirmed at these lines; if line numbers drifted, the *pattern* is the anchor):

- `:550` — `db.partial_cmp(&da).unwrap()` (vertex-pool sort).
- `:791` — `a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1))`.
- `:893` — `a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1))`.
- `:1068` — `a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1))`.

The local frame (`refine.rs:55-66`):

```rust
fn to_xy(&self, p: PointArray) -> [f64; 2] {
    [ (p[1] - self.origin[1]) * self.m_per_deg_lon,
      (p[0] - self.origin[0]) * self.m_per_deg_lat ]
}
fn to_latlng(&self, xy: [f64; 2]) -> PointArray {
    [ self.origin[0] + xy[1] / self.m_per_deg_lat,
      self.origin[1] + xy[0] / self.m_per_deg_lon ]
}
```

`m_per_deg_lat` and `m_per_deg_lon` are fields on the `LocalFrame` struct, computed where the
frame is constructed. Find that construction site: `grep -n "m_per_deg_lat" crates/algorithms/src/clustering/crucible/refine.rs`
(there will be an assignment like `m_per_deg_lat: <expr>` in the constructor). The fix
clamps them there.

## Commands you will need

| Purpose | Command                                                     | Expected         |
|---------|------------------------------------------------------------|------------------|
| Build   | `cargo build -p algorithms`                                | exit 0           |
| Lint    | `cargo clippy -p algorithms --all-targets -- -D warnings`  | exit 0           |
| Tests   | `cargo test -p algorithms crucible`                        | all pass, exit 0 |

## Scope

**In scope:**
- `crates/algorithms/src/clustering/crucible/refine.rs` — the four `partial_cmp().unwrap()`
  comparators and the `m_per_deg_*` clamp at the `LocalFrame` constructor.
- A regression test (see Test plan).
- `plans/README.md` — status row.

**Out of scope (do NOT touch):**
- Clustering *logic* — do not change ordering semantics for finite inputs. The fallback only
  fires on NaN; the clamp only fires at degenerate (~pole) latitudes.
- Other crucible files (`mod.rs`, etc.) and `greedy.rs`.

## Git workflow

- Branch: `advisor/005-crucible-non-finite`.
- One commit: `fix(algorithms): NaN-safe crucible comparators + clamp local-frame scale`.
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Make the four comparators NaN-safe

At each of the four sites, replace `.partial_cmp(&x).unwrap()` with
`.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal)`. Keep any `.then(...)` tie-breaker
exactly as-is. Example for `:791`/`:893`/`:1068`:

```rust
a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal).then(a.1.cmp(&b.1))
```

and for `:550`:

```rust
db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
```

Confirm you found ALL of them: `grep -n "partial_cmp" crates/algorithms/src/clustering/crucible/refine.rs`
must show no remaining `.unwrap()` directly after a `partial_cmp(...)`.

**Verify**: `cargo build -p algorithms` → exit 0;
`grep -n "partial_cmp(.*).unwrap()" crates/algorithms/src/clustering/crucible/refine.rs` → no matches.

### Step 2: Floor the local-frame scale factors

At the `LocalFrame` constructor (found via the grep above), clamp each scale factor away
from zero so `to_latlng`'s division can never produce a non-finite value:

```rust
// 1 meter/degree is far below any real value (~111_000 at the equator, ~hundreds near the
// poles); this floor only triggers on degenerate near-pole inputs and prevents NaN/inf.
m_per_deg_lat: <existing expr>.max(1.0),
m_per_deg_lon: <existing expr>.max(1.0),
```

(If the fields are assigned via local variables before the struct literal, clamp those
variables instead — same effect.)

**Verify**: `cargo build -p algorithms` → exit 0.

### Step 3: Regression test

Add a test (in the existing `#[cfg(test)] mod tests` of `refine.rs`, or `crucible/mod.rs` if
that's where crucible's public `run` is tested — match where current crucible tests live)
that clusters a degenerate input and asserts **no panic**: include exact-duplicate points and
an extreme latitude (e.g. `89.9999`). The test passes if the call returns at all.

```rust
#[test]
fn crucible_does_not_panic_on_degenerate_input() {
    // Duplicates + near-pole latitude previously risked NaN in the local frame → a
    // partial_cmp().unwrap() panic. This must now return without panicking.
    let pts = vec![
        [89.9999_f64, 0.0_f64],
        [89.9999, 0.0],          // exact duplicate
        [89.9998, 0.0001],
        [-89.9999, 179.9999],
    ];
    // Call the same entry point the existing crucible tests use (e.g. Crucible{..}.run(&pts)).
    // Match the constructor/fields used by neighboring tests in this module.
    let _ = /* crucible run over pts */;
}
```

Fill the call to match the existing crucible test harness in this crate (look at a nearby
`#[test]` to copy the exact constructor and `run` signature).

**Verify**: `cargo test -p algorithms crucible` → all pass, including the new test.

## Test plan

- New regression test: degenerate input (duplicates + extreme latitude) must not panic.
- Pattern: copy the constructor/run call from an existing crucible `#[test]` in this crate.
- Verification: `cargo test -p algorithms crucible` → all pass. (The fix is primarily
  defensive; the test guards the path even though real GO data rarely reaches it.)

## Done criteria

ALL must hold:

- [ ] No `partial_cmp(...).unwrap()` remains in `refine.rs`:
      `grep -n "partial_cmp(.*).unwrap()" crates/algorithms/src/clustering/crucible/refine.rs` → empty.
- [ ] `m_per_deg_lat`/`m_per_deg_lon` are floored at the constructor (`.max(1.0)` or equivalent).
- [ ] The regression test exists and passes.
- [ ] `cargo build -p algorithms` exits 0; `cargo clippy -p algorithms --all-targets -- -D warnings` exits 0.
- [ ] `cargo test -p algorithms` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- There are more or fewer than four `partial_cmp(...).unwrap()` sites, or any is in test
  code — confirm each is production before changing it.
- The `LocalFrame` scale factors are not simple fields you can clamp at construction (e.g.
  they're recomputed per call) — report the actual shape rather than guessing.
- The existing crucible tests fail after the comparator change (the `Equal` fallback must
  not alter ordering for finite inputs; if a test breaks, something else is going on).

## Maintenance notes

- This is defensive hardening, not a correctness fix for a known-reproducing bug; the test
  documents intent. If a real NaN-producing input is ever found, add it as a concrete case.
- A reviewer should confirm the `Equal` fallback is only a tie-break (doesn't change finite
  ordering) and the clamp floor (1.0) is unreachable for real lat/lon.
- Plan 007 (deferred) proposes caching the crucible hot-path grid scans — if that lands, it
  touches the same file; re-run this plan's regression test after.
