//! Browser WASM build of Koji's clustering algorithm (demo/portfolio).
//! Plugins, routing, bootstrap, DB and network are intentionally excluded.

use wasm_bindgen::prelude::*;

/// Re-export the wasm-bindgen-rayon thread-pool initializer. The JS caller must
/// `await initThreadPool(navigator.hardwareConcurrency)` before calling `cluster`.
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;

/// Install a panic hook so Rust panics surface as readable console errors.
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// Crate version string (for the demo footer / cache-busting).
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
