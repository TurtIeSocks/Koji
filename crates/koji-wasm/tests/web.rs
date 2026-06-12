#![cfg(target_arch = "wasm32")]
//! Headless-browser smoke test for the non-threaded surface. The threaded
//! `cluster()` path needs initThreadPool + cross-origin isolation, which the
//! headless harness does not provide — that path is validated by the browser
//! demo. Here we only prove the module loads and the JS boundary works.

use koji_wasm::{ClusterRequest, version};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn version_is_nonempty() {
    assert!(!version().is_empty());
}

#[wasm_bindgen_test]
fn request_deserializes_from_js() {
    // Round-trip through JSON string → serde_json → ClusterRequest (native serde path).
    // This exercises the serde Deserialize impl (including defaults) without
    // touching the tsify from_wasm_abi path, which requires the full JS ABI.
    let req: ClusterRequest = serde_json::from_str(
        r#"{
        "points": [[35.0, 139.0]],
        "radius": 100.0,
        "min_points": 1,
        "max_clusters": 0
    }"#,
    )
    .unwrap();
    assert_eq!(req.points.len(), 1);
    assert_eq!(req.cluster_mode, "balanced"); // default applied
    assert_eq!(req.calculation_mode, "radius"); // default applied
}
