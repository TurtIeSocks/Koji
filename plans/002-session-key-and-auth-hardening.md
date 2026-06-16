# Plan 002: Replace the zero session-cookie key and harden auth comparisons

> **Executor instructions**: Follow this plan step by step. Run every verification
> command and confirm the expected result before moving on. If a "STOP condition" occurs,
> stop and report — do not improvise. When done, update this plan's row in
> `plans/README.md`. This plan changes authentication wiring — be precise and do not
> broaden scope.
>
> **Drift check (run first)**:
> `git diff --stat 658d67f..HEAD -- crates/koji-service/src/lib.rs crates/koji-service/src/utils/auth.rs crates/koji-service/src/public/v2/auth.rs`
> If any of these changed since this plan was written, compare the "Current state"
> excerpts against the live code; on mismatch, STOP.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: MED (touches auth/session wiring; a mistake locks users out or weakens auth)
- **Depends on**: 001 (so the change is gated by CI tests) — soft dependency; can land first
  if 001 is not yet done, but prefer after.
- **Category**: security
- **Planned at**: commit `658d67f`, 2026-06-16

## Why this matters

The production HTTP server signs/encrypts its session cookie with a **hardcoded all-zero
key** (`Key::from(&[0; 64])` in `crates/koji-service/src/lib.rs:306`, inside `start()`).
The session carries a single `logged_in: bool` flag (`utils/auth.rs:14`). Because the key
is a public compile-time constant, anyone can forge a cookie with `logged_in=true` and
**bypass authentication on every `/api/v2` route** (`public_validator` returns `Ok` the
moment `logged_in(&req)` is true). The same middleware sets `cookie_secure(false)`, so the
cookie also rides plain HTTP. A global API-security rework is already planned for this
branch; this plan is the concrete, highest-impact slice of it. Secondary: bearer-token and
password equality use non-constant-time `==` (`utils/auth.rs:32`, `public/v2/auth.rs:39,61`).

## Current state

- **`crates/koji-service/src/lib.rs`** — the production server factory `start()`. The
  offending middleware (production path) at lines ~305–309:
  ```rust
  .wrap(
      SessionMiddleware::builder(CookieSessionStore::default(), Key::from(&[0; 64]))
          .cookie_secure(false)
          .build(),
  )
  ```
  Imports already present: `use actix_web::{cookie::Key, ...}` and
  `use actix_session::{SessionMiddleware, storage::CookieSessionStore}`.
  There are **two more** `Key::from(&[0; 64])` sites at lines ~98 and ~139 — these are in
  `test_db_app()` and `test_db_free_app()` (`#[doc(hidden)]` test-only App builders). **Leave
  those two as-is** (tests deliberately use a fixed key).
- **`crates/koji-service/src/utils/auth.rs`** — `public_validator` (the bearer gate). The
  token comparison at line 31–35:
  ```rust
  if let Some(credentials) = credentials
      && credentials.token() == env::var("KOJI_SECRET").unwrap_or("".to_string())
  {
      return Ok(req);
  }
  ```
  Note: the empty-`KOJI_SECRET`-is-open branch (lines 28–30) is **intentional, documented
  dev behavior** (`public/v2/auth.rs:27`). Do NOT remove it. (An optional fail-closed-in-prod
  hardening is listed as a follow-up in Maintenance notes — not in scope here.)
- **`crates/koji-service/src/public/v2/auth.rs`** — the `login` handler at line 61:
  `if payload.password == env::var("KOJI_SECRET").unwrap_or_default()`, and `decide_via`
  at line 39: `} else if bearer == Some(secret) {`.
- **Dependency note**: the `subtle` crate (constant-time comparison) is already in
  `Cargo.lock` (transitive). Adding it as a direct dependency of `koji-service` pulls no new
  download.
- **actix API facts** (verified against the installed version): `actix_web::cookie::Key`
  has `Key::generate()` (random) and `Key::derive_from(&[u8])` (HKDF; **panics if input <
  32 bytes**). `SessionMiddleware::builder(...)` supports `.cookie_secure(bool)`,
  `.cookie_http_only(bool)`, `.cookie_same_site(actix_web::cookie::SameSite)`.

## Commands you will need

| Purpose   | Command                                              | Expected           |
|-----------|------------------------------------------------------|--------------------|
| Build     | `cargo build -p koji-service`                        | exit 0             |
| Lint      | `cargo clippy -p koji-service --all-targets -- -D warnings` | exit 0       |
| Tests     | `cargo test -p koji-service`                         | all pass, exit 0   |

## Scope

**In scope:**
- `crates/koji-service/src/lib.rs` — the `start()` session middleware (lines ~305–309) +
  a new `session_key()` helper fn.
- `crates/koji-service/src/utils/auth.rs` — constant-time token compare + helper.
- `crates/koji-service/src/public/v2/auth.rs` — constant-time password/bearer compare.
- `crates/koji-service/Cargo.toml` — add `subtle` dependency.
- `.env.example` — document `KOJI_SESSION_KEY` and `KOJI_INSECURE_COOKIES`.
- `plans/README.md` — status row.

**Out of scope (do NOT touch):**
- The two test-only `Key::from(&[0; 64])` sites in `test_db_app()`/`test_db_free_app()`.
- The empty-`KOJI_SECRET`-is-open behavior (intentional dev convenience).
- Any per-route authorization redesign — that is the broader security rework, not this plan.

## Git workflow

- Branch: `advisor/002-session-key-auth`.
- Commit per logical unit; conventional-commit style, e.g.
  `fix(service)!: derive session key from KOJI_SESSION_KEY (was zero key)`.
- Do NOT push or open a PR unless the operator instructed it.

## Steps

### Step 1: Add a `session_key()` helper and use it in `start()`

In `crates/koji-service/src/lib.rs`, add this free function near `start()` (above it):

```rust
/// Build the session cookie key. Production MUST set `KOJI_SESSION_KEY` (≥32 bytes of
/// high-entropy text) so sessions are unforgeable and survive restarts / multiple
/// instances. When unset (or too short) we generate an ephemeral random key and warn —
/// acceptable for local dev only (sessions reset on restart and won't validate across
/// instances).
fn session_key() -> actix_web::cookie::Key {
    use actix_web::cookie::Key;
    match std::env::var("KOJI_SESSION_KEY") {
        Ok(s) if s.len() >= 32 => Key::derive_from(s.as_bytes()),
        Ok(_) => {
            log::warn!(
                "[koji] KOJI_SESSION_KEY is set but < 32 bytes; generating an ephemeral key"
            );
            Key::generate()
        }
        Err(_) => {
            log::warn!(
                "[koji] KOJI_SESSION_KEY unset; generating an ephemeral session key \
                 (dev only — sessions won't survive a restart or load-balance across instances)"
            );
            Key::generate()
        }
    }
}
```

Then replace the production middleware block in `start()` (the one at ~305–309, the only
one inside `start()`) with:

```rust
.wrap(
    SessionMiddleware::builder(CookieSessionStore::default(), session_key())
        .cookie_secure(std::env::var("KOJI_INSECURE_COOKIES").is_err())
        .cookie_http_only(true)
        .cookie_same_site(actix_web::cookie::SameSite::Lax)
        .build(),
)
```

`cookie_secure` is `true` unless `KOJI_INSECURE_COOKIES` is set (local HTTP dev escape
hatch). Leave the two test-helper middleware blocks (lines ~98, ~139) untouched.

**Verify**: `cargo build -p koji-service` → exit 0. `cargo test -p koji-service` → all pass.

### Step 2: Add `subtle` and a constant-time string compare to `utils/auth.rs`

In `crates/koji-service/Cargo.toml`, under `[dependencies]`, add:

```toml
subtle = "2.6"
```

In `crates/koji-service/src/utils/auth.rs`, add a helper and use it for the token compare:

```rust
use subtle::ConstantTimeEq;

/// Constant-time string equality — avoids leaking the secret via comparison timing.
/// Different-length inputs compare unequal (length is not the secret here).
fn ct_eq(a: &str, b: &str) -> bool {
    a.as_bytes().ct_eq(b.as_bytes()).into()
}
```

Replace the token comparison (the `credentials.token() == env::var(...)` expression) with:

```rust
if let Some(credentials) = credentials
    && ct_eq(credentials.token(), &env::var("KOJI_SECRET").unwrap_or_default())
{
    return Ok(req);
}
```

**Verify**: `cargo build -p koji-service` → exit 0.

### Step 3: Constant-time compare in `public/v2/auth.rs`

In `crates/koji-service/src/public/v2/auth.rs`:
- Add `use subtle::ConstantTimeEq;` and reuse the same `ct_eq` idea (define a local
  `ct_eq` here too, or make the one in `utils/auth.rs` `pub(crate)` and import it — prefer
  making it `pub(crate) fn ct_eq` in `utils/auth.rs` and importing
  `use crate::utils::auth::ct_eq;`).
- Replace `payload.password == env::var("KOJI_SECRET").unwrap_or_default()` (line ~61) with
  `ct_eq(&payload.password, &env::var("KOJI_SECRET").unwrap_or_default())`.
- Replace `bearer == Some(secret)` in `decide_via` (line ~39) with a constant-time check:
  `bearer.is_some_and(|b| ct_eq(b, secret))`.

The existing `decide_via` unit tests (`public/v2/auth.rs` `#[cfg(test)] mod tests`) must
still pass unchanged — they assert the same precedence.

**Verify**: `cargo test -p koji-service` → all pass (including the `decide_via` tests).

### Step 4: Document the new env vars

In `.env.example`, add (with explanatory comments, no real values):

```
# 64+ random bytes (e.g. `openssl rand -hex 48`). REQUIRED in production: signs the
# session cookie. If unset, an ephemeral key is generated and sessions reset on restart.
KOJI_SESSION_KEY=
# Set (to anything) ONLY for local HTTP development to allow non-Secure cookies.
# Leave UNSET in production so cookies are Secure (HTTPS-only).
# KOJI_INSECURE_COOKIES=1
```

**Verify**: `git diff .env.example` shows only the two additions.

## Test plan

- Add a unit test in `crates/koji-service/src/utils/auth.rs` `#[cfg(test)] mod tests`
  (create the module if absent) for `ct_eq`: equal strings → true; differing strings →
  false; different lengths → false; empty vs empty → true.
- The session-key change is wiring inside `start()` (needs a live server to exercise), so
  it is verified by `cargo build` + the existing handler tests passing, plus the manual
  check below.
- Model the test module after the existing `#[cfg(test)] mod tests` in
  `public/v2/auth.rs` (same file uses pure helpers for unit-testability).
- Verification: `cargo test -p koji-service` → all pass, including the new `ct_eq` tests.
- Manual (operator, optional): start the server with `KOJI_SESSION_KEY` set and confirm a
  login session works; restart with the same key and confirm the session still validates;
  change the key and confirm the old cookie is rejected.

## Done criteria

ALL must hold:

- [ ] `grep -n "Key::from(&\[0; 64\])" crates/koji-service/src/lib.rs` returns only the two
      test-helper sites (lines ~98, ~139), NOT a site inside `start()`.
- [ ] `start()` uses `session_key()` with `.cookie_secure(...)` + `.cookie_http_only(true)`
      + `.cookie_same_site(Lax)`.
- [ ] No `== env::var("KOJI_SECRET")` / `payload.password ==` / `bearer == Some(secret)`
      remains: `grep -rn "== env::var(\"KOJI_SECRET\")\|payload.password ==\|bearer == Some" crates/koji-service/src` → no matches.
- [ ] `subtle` is a direct dependency in `crates/koji-service/Cargo.toml`.
- [ ] `.env.example` documents `KOJI_SESSION_KEY` and `KOJI_INSECURE_COOKIES`.
- [ ] `cargo clippy -p koji-service --all-targets -- -D warnings` exits 0.
- [ ] `cargo test -p koji-service` exits 0; the new `ct_eq` tests pass.
- [ ] `plans/README.md` status row updated.

## STOP conditions

Stop and report if:

- `Key::derive_from` is not available on the installed actix `cookie::Key` (API drift) —
  do NOT substitute an arbitrary key derivation; report it.
- Any existing `koji-service` test fails after the change in a way that implies the session
  contract changed (e.g. a handler test that relied on the fixed-key cookie). The
  test-helper Apps keep the fixed key, so this should not happen — if it does, STOP.
- Adding `subtle` triggers a large dependency-tree change (it should already be in the lock;
  if Cargo wants to add many new crates, STOP and report).

## Maintenance notes

- **Deferred (intentionally out of scope)**: making auth *fail-closed in production* when
  `KOJI_SECRET` is unset (today an empty secret opens the API — documented dev behavior).
  That is a behavior change for the broader security rework; decide it there.
- A reviewer should confirm: production no longer uses a constant key; `cookie_secure`
  defaults to true; the empty-secret dev path is untouched; the test Apps still build.
- When the broader security rework lands per-route authorization, the `logged_in` session
  model and this key handling are the foundation it builds on — keep `session_key()` as the
  single source of the key.
