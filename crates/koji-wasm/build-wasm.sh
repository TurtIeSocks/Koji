#!/usr/bin/env bash
# Threaded wasm build for koji-wasm. Run from anywhere.
#
# Produces crates/koji-wasm/pkg/ (ES module + .wasm + .d.ts), then applies a
# post-build patch so the bundle works under PLAIN STATIC HOSTING (Vercel, a
# local file server) without a bundler.
set -euo pipefail

# repo root = two levels up from this script (crates/koji-wasm/)
cd "$(dirname "$0")/../.."

rustup run nightly wasm-pack build crates/koji-wasm --target web -- \
  -Z build-std=panic_abort,std

# wasm-bindgen-rayon's worker bootstrap imports the main module via the bare
# directory '../../..', which only a bundler resolves to the package entry. On a
# static host that URL is a directory (404). Rewrite it to the explicit entry
# file so workers load the module directly.
helper=$(ls crates/koji-wasm/pkg/snippets/wasm-bindgen-rayon-*/src/workerHelpers.js)
sed -i.bak "s#await import('../../..')#await import('../../../koji_wasm.js')#" "$helper"
rm -f "$helper.bak"
echo "patched worker import in: $helper"
echo "done — pkg/ ready for static hosting"
