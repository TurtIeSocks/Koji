# koji-wasm

Browser WASM build of Koji's clustering algorithm (demo/portfolio). Multithreaded
via `wasm-bindgen-rayon`. Clustering only — no plugins/routing/bootstrap/DB.

## Build

Threaded wasm needs nightly + a rebuilt std (`-Z build-std`). Use the build
script — it runs `wasm-pack` and then applies a required post-build patch (see
below):

```sh
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

./crates/koji-wasm/build-wasm.sh
```

Output: `crates/koji-wasm/pkg/` (`.wasm` + ES-module JS glue + `.d.ts`).

The build flags live in `.cargo/config.toml` (`[target.wasm32-unknown-unknown]`):
shared memory for the worker threads (`--shared-memory --import-memory
--max-memory`), the TLS/heap symbol exports wasm-bindgen's threading transform
needs, and the `getrandom` `wasm_js` backend. `wasm-opt` is disabled
(`Cargo.toml` metadata) because the bundled version strips threading.

**Post-build patch:** `wasm-bindgen-rayon`'s worker bootstrap imports the main
module via the bare directory `'../../..'`, which only a bundler resolves. The
script rewrites it to the explicit entry file so the bundle works under plain
static hosting (Vercel) with no bundler. This is why you must build via the
script, not a raw `wasm-pack` invocation.

## Run the demo locally

The page must be cross-origin isolated (COOP/COEP) for `SharedArrayBuffer`. A
small static server that sets those headers is included:

```sh
node crates/koji-wasm/serve.mjs   # → http://localhost:8080/www/
```

(Production: Vercel sets these headers via `vercel.json`.)

## Usage

```js
import init, { initThreadPool, cluster, version } from "./pkg/koji_wasm.js";
await init();
await initThreadPool(navigator.hardwareConcurrency);
const res = cluster({ points: [[35.0, 139.0]], radius: 100, min_points: 1, max_clusters: 0 });
```
