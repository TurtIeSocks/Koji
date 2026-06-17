# koji-wasm — browser WASM build of the clustering algorithm

**Date:** 2026-06-10
**Status:** Design approved (delegate mode), pending spec review
**Scope:** Phase 1 — clustering only, for demo/portfolio

## 1. Goal

Ship a browser-loadable WebAssembly module exposing Koji's **clustering**
algorithm, for demo/portfolio use. No plugins, no server, no database. The real
`algorithms` crate code runs in the browser, multithreaded, unmodified.

Non-goals (Phase 1): routing/VRP (native or-tools), bootstrap, plugin execution,
any DB/network/golbat functionality, a polished demo UI.

## 2. The shape of the work

The new crate (`crates/koji-wasm`) is thin. The bulk of the effort is making the
existing `algorithms` crate compile to `wasm32-unknown-unknown`. It currently
cannot. Five blockers:

| # | Blocker | Sites | Resolution |
|---|---------|-------|------------|
| 1 | `rayon` (real threads) | 13 files | **Keep.** Run via `wasm-bindgen-rayon` (threaded wasm). No code change. |
| 2 | `sysinfo` (memory probe) | `clustering/greedy.rs`, `clustering/partition.rs` | `#[cfg(feature="native")]` real probe; wasm path uses a fixed memory constant + `threads = 1` baseline for chunk sizing. |
| 3 | `std::fs` (debug output writes) | `utils.rs` | Gate the file-IO helpers `#[cfg(feature="native")]`. |
| 4 | `koji-plugins` (hard dep) | dispatch `Custom` arms, `JoinFunction`, `PluginKind` imports | Make the dep optional behind `native`. On wasm, `ClusterMode::Custom` falls back to the auto algorithm (logs, ignores the plugin). |
| 5 | `std::time::Instant::now()` (panics on wasm) | `stats.rs`, `clustering/greedy.rs`, `clustering/mod.rs`, `clustering/partition.rs` | Swap `use std::time::Instant` → `use web_time::Instant` (drop-in; `web-time` == std on native, `performance.now()` on wasm). |

Decision: **`wasm-bindgen-rayon`** (real multithreading) over a single-threaded
sequential shim. Trade-off accepted by the user: the algorithm runs unmodified
and actually parallel — better portfolio story, zero risk of seq-shim behavioral
divergence — at the cost of a nightly wasm build and COOP/COEP hosting (§6).

## 3. Changes to the `algorithms` crate

The native build must remain **byte-for-byte unchanged**. Achieved with a
`native` feature that is on by default.

### 3.1 Feature gating

`crates/algorithms/Cargo.toml`:

```toml
[features]
default = ["native"]
# Native-only deps: filesystem, system probing, plugin execution.
native = ["dep:sysinfo", "dep:koji-plugins"]

[dependencies]
rayon = "1.11.0"            # kept on all targets (wasm via wasm-bindgen-rayon)
web-time = "1"             # std-compatible Instant on wasm, std on native
sysinfo = { version = "0.37.0", optional = true }
koji-plugins = { path = "../koji-plugins", optional = true }
# ... existing deps unchanged ...
```

`rayon` stays a hard dependency (it compiles and runs on threaded wasm). Only
`sysinfo` and `koji-plugins` become optional.

### 3.2 Module gating

In `lib.rs`, the modules not needed for the wasm clustering path are gated:

```rust
#[cfg(feature = "native")] pub mod bootstrap;   // imports PluginKind; Phase 2
pub mod clustering;                              // wasm surface
#[cfg(feature = "native")] mod plugins;          // koji-plugins bridge
#[cfg(feature = "native")] mod project;          // gate iff it fails wasm build
#[cfg(feature = "native")] pub mod routing;      // VRP / or-tools, native-only
mod rtree;                                        // wasm path
#[cfg(feature = "native")] pub mod s2;           // gate iff needed
#[cfg(feature = "native")] mod sec;              // native-only
pub mod stats;                                    // wasm path (web-time)
pub mod utils;                                    // file-IO helpers gated inside
```

The exact final gating set is discovered by iterating
`cargo build --target wasm32-unknown-unknown -p koji-wasm` until clean. The set
above is the expected starting point; residual gating is mechanical.

### 3.3 sysinfo fallback

```rust
#[cfg(feature = "native")]
let available_memory = { let sys = sysinfo::System::new_all(); sys.available_memory() };
#[cfg(not(feature = "native"))]
let available_memory: u64 = 512 * 1024 * 1024; // conservative wasm baseline
```

Thread/CPU counts used purely for chunk sizing fall back to `1` on wasm.

### 3.4 `ClusterMode::Custom` on wasm

`ClusterMode::Custom(_)` is a koji-core enum variant compiled on all targets, so
the match must stay exhaustive. On wasm it cannot execute a plugin:

```rust
#[cfg(not(feature = "native"))]
ClusterMode::Custom(_) => {
    log::warn!("custom clustering plugins unavailable in wasm; using auto algorithm");
    /* dispatch to the auto algorithm with cfg params */
}
```

## 4. New crate: `crates/koji-wasm`

```toml
[package]
name = "koji-wasm"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
algorithms = { path = "../algorithms", default-features = false } # native feature OFF
koji-core  = { path = "../koji-core" }
wasm-bindgen = "0.2"
wasm-bindgen-rayon = "1"
serde = { workspace = true }
serde-wasm-bindgen = "0.6"
tsify = { version = "0.4", features = ["js"] }   # or tsify-next
console_error_panic_hook = "0.1"
getrandom = { version = "0.2", features = ["js"] } # rand → getrandom backend on wasm
js-sys = "0.3"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

> Note: `wasm-bindgen-rayon` pins a compatible `wasm-bindgen` range; the exact
> versions are reconciled during implementation against the lockfile.

### 4.1 Typed API (tsify + serde-wasm-bindgen)

Wasm-local DTOs keep koji-core clean and auto-generate `.d.ts` TypeScript
interfaces. Conversions map DTOs to/from koji-core types.

```rust
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[derive(Tsify, serde::Deserialize)]
#[tsify(from_wasm_abi)]
pub struct ClusterRequest {
    pub points: Vec<[f64; 2]>,   // [[lat, lng], ...]
    pub radius: f64,
    pub min_points: usize,
    pub max_clusters: usize,
    pub cluster_mode: String,    // "fastest" | "balanced" | "best" | ...
    pub calculation_mode: String,// "radius" | "s2"
    pub center_clusters: bool,
    // s2 level/size when calculation_mode == "s2"
    pub s2_level: u8,
    pub s2_size: u8,
}

#[derive(Tsify, serde::Serialize)]
#[tsify(into_wasm_abi)]
pub struct ClusterResponse {
    pub clusters: Vec<[f64; 2]>, // cluster centers
    pub stats: StatsSummary,     // subset of algorithms::stats::Stats
}

#[derive(Tsify, serde::Serialize)]
#[tsify(into_wasm_abi)]
pub struct StatsSummary {
    pub total_points: usize,
    pub points_covered: usize,
    pub total_clusters: usize,
    pub cluster_time_ms: f64,
}

#[wasm_bindgen(start)]
pub fn start() { console_error_panic_hook::set_once(); }

#[wasm_bindgen]
pub fn cluster(req: ClusterRequest) -> Result<ClusterResponse, JsError> {
    let cfg: koji_core::ClusteringConfig = req.to_core()?;   // DTO → core, validates enums
    let points: koji_core::SingleVec = req.points;
    let mut stats = algorithms::stats::Stats::new("wasm".into(), cfg.min_points);
    let collection = geojson::FeatureCollection { /* empty; only used by S2 mode */ };
    let clusters = algorithms::clustering::main(&points, &cfg, collection, false, &mut stats);
    Ok(ClusterResponse::from_parts(clusters, &stats))
}

#[wasm_bindgen]
pub fn version() -> String { env!("CARGO_PKG_VERSION").into() }
```

`wasm-bindgen-rayon` additionally re-exports an `initThreadPool` the JS caller
awaits before invoking `cluster` (see §6.2). Expose it per the crate's docs:

```rust
pub use wasm_bindgen_rayon::init_thread_pool;
```

### 4.2 Conversion `to_core`

`ClusterRequest::to_core()` builds a `koji_core::ClusteringConfig`, parsing the
string enum fields into `ClusterMode` / `CalculationMode` and returning a
`JsError` on an unknown value. This is the single validation boundary between
untyped JS input and the typed algorithm.

## 5. Build configuration

`rust-toolchain.toml` does not exist yet; native stays on stable. The wasm build
uses nightly explicitly via the build command, not a pinned workspace toolchain.

`.cargo/config.toml` (workspace root — keyed to the wasm target only, so native
builds are unaffected):

```toml
[target.wasm32-unknown-unknown]
rustflags = ["-C", "target-feature=+atomics,+bulk-memory,+mutable-globals"]
```

Build command (documented in `crates/koji-wasm/README.md`):

```sh
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
wasm-pack build crates/koji-wasm --target web -- \
  -Z build-std=panic_abort,std
# emits crates/koji-wasm/pkg/  (.wasm + ES-module JS glue + .d.ts)
```

`wasm-pack` runs `wasm-opt` for size. Output `pkg/` is git-ignored (build
artifact).

## 6. Hosting / runtime requirements

### 6.1 COOP/COEP (cross-origin isolation) — Vercel

`SharedArrayBuffer` (required by threaded wasm) needs the page served with:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

**Hosting decision: Vercel** (sets headers natively — no service-worker shim, no
single-threaded fallback build needed). GitHub Pages was considered and rejected:
it cannot set headers, forcing the `coi-serviceworker` shim, whose `COEP:
require-corp` poisons cross-origin map tiles and hard-fails in incognito/SW-blocked
contexts — both fatal to a map-based portfolio demo.

`vercel.json` (scoped to the demo path so the rest of any site is unaffected):

```json
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        { "key": "Cross-Origin-Opener-Policy",   "value": "same-origin" },
        { "key": "Cross-Origin-Embedder-Policy",  "value": "require-corp" }
      ]
    }
  ]
}
```

Under `COEP: require-corp`, any cross-origin assets on the demo page (e.g. map
tiles, CDN scripts) must be CORS/CORP-clean — load Leaflet from a CORS CDN with
`crossorigin`, and use a tile provider that sends CORS (MapTiler/Mapbox) if the
demo plots a map.

### 6.2 JS init order

```js
import init, { initThreadPool, cluster } from "./pkg/koji_wasm.js";
await init();
await initThreadPool(navigator.hardwareConcurrency);
const res = cluster({ points: [[35.0,139.0], ...], radius: 100, min_points: 1, ... });
```

## 7. Demo harness (in scope, minimal)

`crates/koji-wasm/www/index.html` — a bare smoke page: load the module, init the
thread pool, cluster a small hardcoded sample, render the result count + centers
to the DOM. Proves the pipeline end-to-end. A polished Leaflet/portfolio UI is
**out of scope** (follow-up).

## 8. Testing

- **Native unaffected:** `cargo test` (workspace) passes unchanged — the `native`
  default feature preserves current behavior. Verify after the gating edits.
- **Wasm builds clean:** `cargo build --target wasm32-unknown-unknown -p koji-wasm`
  (with the nightly/build-std flags) is the gating check.
- **Wasm smoke:** one `wasm-bindgen-test` (headless Chrome) — cluster N sample
  points, assert non-empty clusters and `stats.total_points == N`.

## 9. Risks

- **wasm-bindgen / wasm-bindgen-rayon version skew.** Resolve against the
  lockfile during implementation; pin compatible versions.
- **`rand` on wasm.** Demo clustering path likely never calls an RNG, but
  `getrandom` with the `js` feature is included so any path that does won't panic.
- **Nightly drift.** `build-std` is nightly-only; the documented build may need a
  pinned nightly if a future nightly regresses. Acceptable for a portfolio
  artifact.
- **`project.rs` / `s2.rs` wasm-compat** unknown until the build is attempted;
  gate native-only if they pull native-only deps.

## 10. Assumptions (delegate-mode calls, user-approved)

1. Phase 1 = clustering only. Routing, bootstrap, plugins excluded.
2. `wasm-bindgen-rayon` (real threads), not a single-threaded shim.
3. Crate at `crates/koji-wasm` (cdylib).
4. Typed API via `tsify` + `serde-wasm-bindgen`, wasm-local DTOs.
5. Input points = `[[lat, lng], ...]`; config mirrors `ClusteringConfig`.
6. `native` feature on by default → native builds untouched.
7. Minimal smoke HTML in scope; polished UI out of scope.
8. `ClusterMode::Custom` on wasm falls back to the auto algorithm.
