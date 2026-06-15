//! Integration tests for the `#[time]` attribute macro.
//!
//! The macro wraps a function body with `Instant::start` / `log::info!` timers
//! (guarded `#[cfg(not(target_arch = "wasm32"))]`) and a closure to capture the
//! return value. Tests here verify:
//!   - The wrapped function still executes its body and returns correctly.
//!   - The optional message string is accepted (and its absence too).
//!   - `fn` returning `()`, `fn` returning a value, and early returns all compile.
//!   - `pub`/`pub(crate)` visibility modifiers are preserved.
//!
//! `log` is a dev-dep so `log::info!` resolves in the generated call sites.

// ---------------------------------------------------------------------------
// Basic wrapping: no message argument.
// ---------------------------------------------------------------------------

/// Returns a value — verifies the closure trick preserves the return value.
#[macros::time]
fn add_one(x: i32) -> i32 {
    x + 1
}

#[test]
fn time_no_message_returns_value() {
    assert_eq!(add_one(41), 42);
}

// ---------------------------------------------------------------------------
// With an explicit message string argument.
// ---------------------------------------------------------------------------

#[macros::time("custom timer")]
fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

#[test]
fn time_with_message_returns_value() {
    assert_eq!(multiply(6, 7), 42);
}

// ---------------------------------------------------------------------------
// Unit return: closure still resolves to `()`.
// ---------------------------------------------------------------------------

static SIDE_EFFECT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[macros::time("side effect")]
fn set_flag() {
    SIDE_EFFECT.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[test]
fn time_unit_fn_runs_body() {
    set_flag();
    assert!(SIDE_EFFECT.load(std::sync::atomic::Ordering::SeqCst));
}

// ---------------------------------------------------------------------------
// Visibility is preserved: `pub fn` still compiles and is callable.
// ---------------------------------------------------------------------------

mod inner {
    #[macros::time]
    pub fn public_fn() -> &'static str {
        "hello"
    }

    #[macros::time("crate-visible")]
    pub(crate) fn crate_fn() -> u32 {
        99
    }
}

#[test]
fn time_preserves_visibility() {
    assert_eq!(inner::public_fn(), "hello");
    assert_eq!(inner::crate_fn(), 99);
}

// ---------------------------------------------------------------------------
// Multiple parameters + returns a computed value.
// ---------------------------------------------------------------------------

#[macros::time]
fn sum_slice(xs: &[i32]) -> i32 {
    xs.iter().sum()
}

#[test]
fn time_multi_param_fn() {
    assert_eq!(sum_slice(&[1, 2, 3, 4, 5]), 15);
}

// ---------------------------------------------------------------------------
// Early return inside the body still works (the closure captures it).
// ---------------------------------------------------------------------------

#[macros::time]
fn first_positive(xs: &[i32]) -> Option<i32> {
    for &x in xs {
        if x > 0 {
            return Some(x);
        }
    }
    None
}

#[test]
fn time_early_return_works() {
    assert_eq!(first_positive(&[-1, -2, 3, 4]), Some(3));
    assert_eq!(first_positive(&[-1, -2]), None);
}
