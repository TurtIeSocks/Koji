//! Browser WASM build of Koji's clustering algorithm (demo/portfolio).
//! Plugins, routing, bootstrap, DB and network are intentionally excluded.

use wasm_bindgen::prelude::*;

mod convert;
mod dto;

pub use dto::{ClusterRequest, ClusterResponse, StatsSummary};

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

/// Cluster `points` under `req`'s parameters. Returns centers + stats, or a
/// `JsError` on invalid input.
#[wasm_bindgen]
pub fn cluster(req: ClusterRequest) -> Result<ClusterResponse, JsError> {
    let (points, cfg) = req.into_core()?;
    let mut stats = algorithms::stats::Stats::new("wasm".to_string(), cfg.min_points);
    // `collection` is only consulted by the S2 calculation mode; the radius demo
    // path passes an empty FeatureCollection.
    let collection = geojson::FeatureCollection {
        bbox: None,
        features: vec![],
        foreign_members: None,
    };
    let clusters = algorithms::clustering::main(&points, &cfg, collection, false, &mut stats);
    Ok(ClusterResponse::from_parts(clusters, &stats))
}
