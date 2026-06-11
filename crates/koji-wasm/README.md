# koji-wasm

Browser WASM build of Koji's clustering algorithm (demo/portfolio). Multithreaded
via `wasm-bindgen-rayon`. Clustering only — no plugins/routing/bootstrap/DB.

## Build

Threaded wasm needs nightly + a rebuilt std (`-Z build-std`):

```sh
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

rustup run nightly wasm-pack build crates/koji-wasm --target web -- \
  -Z build-std=panic_abort,std
```

Output: `crates/koji-wasm/pkg/` (`.wasm` + ES-module JS glue + `.d.ts`).

## Run the demo locally

The page must be cross-origin isolated (COOP/COEP) for `SharedArrayBuffer`:

```sh
npx http-server crates/koji-wasm/www -p 8080 \
  --cors -c-1 \
  --header "Cross-Origin-Opener-Policy: same-origin" \
  --header "Cross-Origin-Embedder-Policy: require-corp"
```

(Production: Vercel sets these headers via `vercel.json`.)

## Usage

```js
import init, { initThreadPool, cluster, version } from "./pkg/koji_wasm.js";
await init();
await initThreadPool(navigator.hardwareConcurrency);
const res = cluster({ points: [[35.0, 139.0]], radius: 100, min_points: 1, max_clusters: 0 });
```
