# Plan 003: Stop `parse_property_value` from panicking on malformed property data

> **Executor instructions**: Follow this plan step by step. Run every verification command
> and confirm the expected result before moving on. If a "STOP condition" occurs, stop and
> report. When done, update this plan's row in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 658d67f..HEAD -- crates/koji-db/src/utils/json.rs`
> If the file changed since this plan was written, compare the "Current state" excerpt
> against the live code; on mismatch, STOP.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

`parse_property_value` (in `crates/koji-db/src/utils/json.rs`) turns a stored property
string into a JSON value based on the property's `Category`. Three branches `.unwrap()` on
fallible conversions, so **malformed database content panics the request handler** (HTTP
500 via panic, not a clean error):

- `Category::Number`: `serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.)).unwrap()`
  — Rust parses `"nan"`/`"inf"` to `NaN`/`Infinity`, and `Number::from_f64` returns `None`
  for non-finite values → the `.unwrap()` panics.
- `Category::Object` / `Category::Array`: `serde_json::Value::from_str(value).unwrap()` —
  panics if the stored string is not valid JSON (e.g. a truncated `{`).

This runs during feature/geofence rendering whenever a property is included, so one bad row
takes down the response. The fix is a localized swap to safe fallbacks; the function
signature stays `-> Value`.

## Current state

`crates/koji-db/src/utils/json.rs:404-415` (production; `pub fn`, not test code):

```rust
pub fn parse_property_value(value: &String, category: &Category) -> Value {
    match category {
        Category::String | Category::Color => serde_json::Value::String(value.to_string()),
        Category::Number => serde_json::Value::Number(
            serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.)).unwrap(),
        ),
        Category::Boolean => serde_json::Value::Bool(value.parse::<bool>().unwrap_or(false)),
        Category::Object => serde_json::Value::from_str(value).unwrap(),
        Category::Array => serde_json::Value::from_str(value).unwrap(),
        Category::Database => serde_json::Value::Null,
    }
}
```

- `Value` is `serde_json::Value`; `use std::str::FromStr;` is already in the file (it powers
  `Value::from_str`). `Category` is `crate::category::Category`.
- This file already has a `#[cfg(test)] mod tests` (starts ~line 419 with a `dcbv` helper) —
  add the new tests there.
- Convention: this crate returns `Result<_, ModelError>` on real error paths, but
  `parse_property_value` is infallible-by-contract (returns `Value`), so the correct fix is
  **safe fallback values**, not changing the signature to `Result` (that would ripple into
  every caller). Preserve the existing intent: unparseable numbers already fell back to `0.0`.

## Commands you will need

| Purpose | Command                                   | Expected         |
|---------|-------------------------------------------|------------------|
| Build   | `cargo build -p koji-db`                  | exit 0           |
| Lint    | `cargo clippy -p koji-db --all-targets -- -D warnings` | exit 0 |
| Tests   | `cargo test -p koji-db json`              | all pass, exit 0 |

## Scope

**In scope:**
- `crates/koji-db/src/utils/json.rs` — the `parse_property_value` body + new tests in the
  existing `#[cfg(test)] mod tests`.
- `plans/README.md` — status row.

**Out of scope (do NOT touch):**
- The function signature (`(&String, &Category) -> Value`) — keep it; callers depend on it.
- Any other function in `json.rs` (it has ~49 unwraps total; only the three in
  `parse_property_value` are in this plan's scope).

## Git workflow

- Branch: `advisor/003-parse-property-value-panics`.
- One commit: `fix(db): parse_property_value falls back instead of panicking on bad data`.
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Replace the three panicking branches with safe fallbacks

Rewrite the `match` body so no branch can panic. Target shape:

```rust
pub fn parse_property_value(value: &String, category: &Category) -> Value {
    match category {
        Category::String | Category::Color => Value::String(value.to_string()),
        Category::Number => Value::Number(
            // Non-finite (NaN/inf) and unparseable values fall back to 0 — a JSON number
            // cannot represent NaN/inf, and 0 preserves the prior `unwrap_or(0.)` intent.
            serde_json::Number::from_f64(value.parse::<f64>().unwrap_or(0.0))
                .unwrap_or_else(|| serde_json::Number::from(0)),
        ),
        Category::Boolean => Value::Bool(value.parse::<bool>().unwrap_or(false)),
        // Malformed Object/Array JSON resolves to Null rather than crashing the response.
        Category::Object | Category::Array => Value::from_str(value).unwrap_or(Value::Null),
        Category::Database => Value::Null,
    }
}
```

(`Value` is already in scope in this file; if the file refers to it as `serde_json::Value`
elsewhere, match that style.)

**Verify**: `cargo build -p koji-db` → exit 0. `grep -n "from_f64(.*).unwrap()\|from_str(value).unwrap()" crates/koji-db/src/utils/json.rs` → no matches.

### Step 2: Add characterization tests

In the existing `#[cfg(test)] mod tests` in `json.rs`, add tests that would have panicked
before this fix:

```rust
#[test]
fn parse_property_value_number_nan_falls_back_to_zero() {
    let v = parse_property_value(&"nan".to_string(), &Category::Number);
    assert_eq!(v, serde_json::json!(0));
}

#[test]
fn parse_property_value_number_inf_falls_back_to_zero() {
    let v = parse_property_value(&"inf".to_string(), &Category::Number);
    assert_eq!(v, serde_json::json!(0));
}

#[test]
fn parse_property_value_number_unparseable_falls_back_to_zero() {
    let v = parse_property_value(&"abc".to_string(), &Category::Number);
    assert_eq!(v, serde_json::json!(0));
}

#[test]
fn parse_property_value_malformed_object_is_null() {
    let v = parse_property_value(&"{not json".to_string(), &Category::Object);
    assert_eq!(v, serde_json::Value::Null);
}

#[test]
fn parse_property_value_malformed_array_is_null() {
    let v = parse_property_value(&"[1,".to_string(), &Category::Array);
    assert_eq!(v, serde_json::Value::Null);
}

#[test]
fn parse_property_value_valid_object_round_trips() {
    let v = parse_property_value(&r#"{"a":1}"#.to_string(), &Category::Object);
    assert_eq!(v, serde_json::json!({"a": 1}));
}
```

If `Category` or `parse_property_value` is not already in scope in the test module, add the
needed `use super::*;` / `use crate::category::Category;` (match how the existing tests in
this module import them).

**Verify**: `cargo test -p koji-db json` → all pass, including the 6 new tests.

## Test plan

- New tests (above): NaN / Infinity / unparseable number → `0`; malformed object/array →
  `Null`; valid object still round-trips. These are exactly the inputs that panicked before.
- Pattern: follow the existing `#[cfg(test)] mod tests` already in `json.rs`.
- Verification: `cargo test -p koji-db json` → all pass.

## Done criteria

ALL must hold:

- [ ] `parse_property_value` contains no `.unwrap()` (only `unwrap_or`/`unwrap_or_else`):
      `grep -n "unwrap()" crates/koji-db/src/utils/json.rs` shows none **inside**
      `parse_property_value` (other functions in the file are out of scope).
- [ ] The 6 new tests exist and pass: `cargo test -p koji-db json` exits 0.
- [ ] The function signature is unchanged.
- [ ] `cargo clippy -p koji-db --all-targets -- -D warnings` exits 0.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- The live `parse_property_value` doesn't match the "Current state" excerpt (drift).
- `Category` has variants beyond those in the excerpt (a new variant means a new branch
  decision — don't guess the fallback).
- Changing the Number fallback breaks an existing test that asserted a panic or a specific
  non-zero default (none is expected; if one exists, surface it).

## Maintenance notes

- Returning `Null` for malformed Object/Array is a deliberate fail-soft. If the product
  later needs to surface "this property is corrupt" to the user, do it at the rendering
  layer, not by reintroducing a panic here.
- A reviewer should confirm no `.unwrap()` remains in this function and that the Number
  branch still returns a number (not Null) for ordinary inputs.
- The other ~46 unwraps in `json.rs` were not audited for panic-safety here; if a similar
  panic is reported elsewhere in this file, treat it as a separate finding.
